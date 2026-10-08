use crate::storage::settings::Settings;
use eframe::egui;
use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet, VecDeque},
    fs,
    hash::Hasher,
    io::{self, Cursor, ErrorKind},
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, Receiver, TryRecvError},
        Arc,
    },
};

const MAX_CACHED_ALBUM_TEXTURES: usize = 48;
static TEMP_ID: AtomicUsize = AtomicUsize::new(0);

pub(crate) struct ArtworkCache {
    textures: HashMap<usize, egui::TextureHandle>,
    texture_lru: VecDeque<usize>,
    visible_texture_indices: HashSet<usize>,
    texture_requests: HashSet<(usize, usize)>,
    texture_epoch: usize,
    texture_workers: Arc<AtomicUsize>,
    texture_receiver: Receiver<(usize, usize, Option<egui::ColorImage>)>,
    texture_sender: mpsc::Sender<(usize, usize, Option<egui::ColorImage>)>,
}

impl ArtworkCache {
    pub(crate) fn new() -> Self {
        let (texture_sender, texture_receiver) = mpsc::channel();
        Self {
            textures: HashMap::new(),
            texture_lru: VecDeque::new(),
            visible_texture_indices: HashSet::new(),
            texture_requests: HashSet::new(),
            texture_epoch: 0,
            texture_workers: Arc::new(AtomicUsize::new(0)),
            texture_receiver,
            texture_sender,
        }
    }

    pub(crate) fn clear(&mut self) {
        self.texture_epoch = self.texture_epoch.wrapping_add(1);
        self.texture_requests.clear();
        self.textures.clear();
        self.texture_lru.clear();
        self.visible_texture_indices.clear();
    }

    pub(crate) fn begin_frame(&mut self) {
        self.visible_texture_indices.clear();
    }

    pub(crate) fn texture(
        &mut self,
        ctx: &egui::Context,
        album_index: usize,
        bytes: &[u8],
    ) -> Option<egui::TextureHandle> {
        self.visible_texture_indices.insert(album_index);
        if let Some(texture) = self.textures.get(&album_index).cloned() {
            self.texture_lru.retain(|&index| index != album_index);
            self.texture_lru.push_back(album_index);
            return Some(texture);
        }

        let request = (self.texture_epoch, album_index);
        if !self.texture_requests.contains(&request) {
            let mut active = self.texture_workers.load(Ordering::Relaxed);
            while active < 4 {
                match self.texture_workers.compare_exchange_weak(
                    active,
                    active + 1,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(observed) => active = observed,
                }
            }
            if active < 4 {
                let bytes = bytes.to_vec();
                let sender = self.texture_sender.clone();
                let repaint = ctx.clone();
                let workers = Arc::clone(&self.texture_workers);
                let epoch = self.texture_epoch;
                if std::thread::Builder::new()
                    .name("album-art-decode".into())
                    .spawn(move || {
                        let decoded = decode_album_art(&bytes);
                        let _ = sender.send((epoch, album_index, decoded));
                        workers.fetch_sub(1, Ordering::Relaxed);
                        repaint.request_repaint();
                    })
                    .is_ok()
                {
                    self.texture_requests.insert(request);
                } else {
                    self.texture_workers.fetch_sub(1, Ordering::Relaxed);
                }
            }
        }
        None
    }

    pub(crate) fn poll(&mut self, ctx: &egui::Context) {
        loop {
            let result = match self.texture_receiver.try_recv() {
                Ok(result) => result,
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            };
            let (epoch, album_index, image) = result;
            self.texture_requests.remove(&(epoch, album_index));
            if epoch != self.texture_epoch {
                continue;
            }
            let Some(image) = image else {
                continue;
            };
            let texture = ctx.load_texture(
                format!("album-art-{epoch}-{album_index}"),
                image,
                egui::TextureOptions::LINEAR,
            );
            while self.textures.len() >= MAX_CACHED_ALBUM_TEXTURES {
                let Some(oldest_position) = self
                    .texture_lru
                    .iter()
                    .position(|index| !self.visible_texture_indices.contains(index))
                else {
                    break;
                };
                if let Some(oldest) = self.texture_lru.remove(oldest_position) {
                    self.textures.remove(&oldest);
                }
            }
            self.textures.insert(album_index, texture);
            self.texture_lru.push_back(album_index);
        }
    }
}

fn decode_album_art(bytes: &[u8]) -> Option<egui::ColorImage> {
    let cache_path = match album_art_cache_path(bytes) {
        Ok(path) => Some(path),
        Err(error) => {
            eprintln!("Could not locate album artwork cache: {error}");
            None
        }
    };

    if let Some(path) = &cache_path {
        match fs::read(path) {
            Ok(cached_bytes) => match image::load_from_memory(&cached_bytes) {
                Ok(image) => return Some(color_image(image)),
                Err(error) => {
                    eprintln!(
                        "Could not decode cached album artwork {}: {error}",
                        path.display()
                    );
                    if let Err(remove_error) = fs::remove_file(path) {
                        if remove_error.kind() != ErrorKind::NotFound {
                            eprintln!(
                                "Could not remove invalid album artwork cache {}: {remove_error}",
                                path.display()
                            );
                        }
                    }
                }
            },
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => eprintln!(
                "Could not read album artwork cache {}: {error}",
                path.display()
            ),
        }
    }

    let image = match image::load_from_memory(bytes) {
        Ok(image) => image.into_rgba8(),
        Err(error) => {
            eprintln!("Could not decode album artwork: {error}");
            return None;
        }
    };
    let color_image = egui::ColorImage::from_rgba_unmultiplied(
        [image.width() as usize, image.height() as usize],
        image.as_raw(),
    );

    if let Some(path) = cache_path {
        if let Some(directory) = path.parent() {
            if let Err(error) = fs::create_dir_all(directory) {
                eprintln!(
                    "Could not create album artwork cache {}: {error}",
                    directory.display()
                );
            } else {
                let mut encoded = Vec::new();
                match image::DynamicImage::ImageRgba8(image)
                    .write_to(&mut Cursor::new(&mut encoded), image::ImageFormat::Png)
                {
                    Ok(()) => {
                        let temporary_id = TEMP_ID.fetch_add(1, Ordering::Relaxed);
                        let temporary_path = path.with_extension(format!("{temporary_id}.tmp"));
                        if let Err(error) = fs::write(&temporary_path, encoded) {
                            eprintln!(
                                "Could not write album artwork cache {}: {error}",
                                temporary_path.display()
                            );
                        } else if let Err(error) = fs::rename(&temporary_path, &path) {
                            eprintln!(
                                "Could not save album artwork cache {}: {error}",
                                path.display()
                            );
                            if let Err(remove_error) = fs::remove_file(&temporary_path) {
                                if remove_error.kind() != ErrorKind::NotFound {
                                    eprintln!(
                                        "Could not remove temporary artwork cache {}: {remove_error}",
                                        temporary_path.display()
                                    );
                                }
                            }
                        }
                    }
                    Err(error) => eprintln!("Could not encode album artwork cache: {error}"),
                }
            }
        }
    }

    Some(color_image)
}

fn color_image(image: image::DynamicImage) -> egui::ColorImage {
    let image = image.into_rgba8();
    egui::ColorImage::from_rgba_unmultiplied(
        [image.width() as usize, image.height() as usize],
        image.as_raw(),
    )
}

fn album_art_cache_path(bytes: &[u8]) -> io::Result<PathBuf> {
    let settings_path = Settings::file_path()?;
    let mut hasher = DefaultHasher::new();
    hasher.write(bytes);
    Ok(settings_path
        .with_file_name("cached")
        .join("art-v1")
        .join(format!("{:016x}.png", hasher.finish())))
}
