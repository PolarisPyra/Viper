# musicplayer

A desktop music player built with Rust and egui for browsing an album-art library and playing tracks with FFmpeg.

## Requirements

- Rust and Cargo
- FFmpeg with `ffplay`, `ffprobe`, and `ffmpeg` available on `PATH`

## Run

```sh
cargo run --release
```

The app opens to the Home view. To select or change the music folder, choose **Library → Choose Music Folder**. The selected folder is saved and scanned automatically on later launches. Scan progress appears beside the current page title.

## Features

- Browse the Home and Albums views, search the album library, and open album track lists.
- Play an album or start from a selected track; use previous, play/pause, next, stop, seek, and volume controls.
- Navigate with the File, Library, Playback, and View menu dropdowns.
- Choose **Set Home/Albums as startup view** from the corresponding sidebar item's context menu.
- Album metadata falls back to filenames and folders when tags are missing. Albums are grouped by album artist when available, while track artists remain visible.
- Disc folders such as `CD1`, `Disc 2`, and `Disk 3` use the parent folder as the album name and artwork location.
- Load cover art from neighboring image files or embedded audio artwork; artwork is reduced to a 320 px thumbnail and cached.
- Scan MP3, FLAC, Ogg, Opus, WAV, M4A, AAC, AIFF, WMA, APE, WavPack, DSF, DFF, and WebM files, subject to FFmpeg codec support.

Preferences let you choose your music folder and startup page. Preferences data, including the selected folder and volume, is stored at `~/.config/musicplayer/settings.json`.

## Linux desktop integration

Build and install the app, its icon, and a matching desktop entry in user-local directories:

```sh
./packaging/install-linux.sh
```

The app can then be launched from the desktop environment's application menu.

## Project layout

- `src/main.rs` starts the application; `src/lib.rs` exposes the app modules, and `src/desktop.rs` configures the native window, icon, and system font fallbacks.
- `src/app/` owns application state and coordinates app lifecycle, library scans, and album ordering.
- `src/ui/` contains the egui screens, shared icons, and Preferences UI.
- `src/library/` owns track and album models, library scanning, tag reading, metadata caching, and music-folder watching.
- `src/artwork/` discovers and prepares cover art and manages decoded artwork textures.
- `src/playback/` controls `ffplay`; `src/storage/` loads and saves settings.
