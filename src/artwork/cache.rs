use eframe::egui;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, Receiver, TryRecvError},
        Arc,
    },
};

const MAX_CACHED_ALBUM_TEXTURES: usize = 48;

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
    match image::load_from_memory(bytes) {
        Ok(image) => Some(color_image(image)),
        Err(error) => {
            eprintln!("Could not decode album artwork: {error}");
            None
        }
    }
}

fn color_image(image: image::DynamicImage) -> egui::ColorImage {
    let image = image.into_rgba8();
    egui::ColorImage::from_rgba_unmultiplied(
        [image.width() as usize, image.height() as usize],
        image.as_raw(),
    )
}
