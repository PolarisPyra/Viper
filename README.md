# musicplayer

A small desktop music player focused on browsing an album art grid. It is built with Rust and egui, scans a selected music folder recursively with `std::fs::read_dir`, and uses FFmpeg's `ffplay` as its audio playback process.

## Requirements

- Rust and Cargo
- FFmpeg installed with `ffplay`, `ffprobe`, and `ffmpeg` available on `PATH`

## Run

```sh
cargo run --release
```

## Install desktop entry (Linux)

Build and install the app, its icon, and a matching desktop entry for taskbar integration:

```sh
./packaging/install-linux.sh
```

The installer places the app and desktop integration in your user-local directories. You can then launch **Music Player** from your desktop environment's application menu.

## Layout

- `src/main.rs` only calls the application entry point.
- `src/views/album-view.rs`, `sidepanel.rs`, `topbar.rs`, and `playerbar.rs` own the main UI sections.
- `src/metadata.rs` scans folders and reads track and artwork metadata.
- `src/storage/settings.rs` loads and saves app settings.
- `src/playback.rs` controls the `ffplay` process and album queue.

The first launch opens a folder picker. The chosen path is saved in `~/.config/musicplayer/musicplayer.json` and rescanned automatically on later launches. Audio metadata is read with `ffprobe`; missing title and album tags fall back to filenames and folders. Missing artist tags fall back to the artist directory in an `Artist/Album/track` layout. Albums are grouped by album artist when tags provide it, while individual track artists remain visible in the track list. Disc subfolders named `CD1`, `Disc 2`, or `Disk 3` use their parent folder as the album name and artwork location. Cover images are loaded from common neighboring names (`cover.jpg`, `front.png`, `folder.png`, and similar), then from embedded audio artwork via FFmpeg. Artwork is reduced to a 512 px thumbnail before display. Click an album to select it, use **Tracks** to open its track list, and click a track's play button to start there and continue through the rest of the album. Use **Play selected album** or double-click an album to play it from the start. The player includes previous, next, and stop controls. Tracks play in disc and track order. The scanner recognizes MP3, FLAC, Ogg, Opus, WAV, M4A, AAC, AIFF, WMA, APE, WavPack, DSF, DFF, and WebM files, subject to the installed FFmpeg build's codecs.
