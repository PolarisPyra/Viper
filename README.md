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
- Load cover art from neighboring image files or embedded audio artwork; artwork is reduced to a 512 px thumbnail.
- Scan MP3, FLAC, Ogg, Opus, WAV, M4A, AAC, AIFF, WMA, APE, WavPack, DSF, DFF, and WebM files, subject to FFmpeg codec support.

The Settings window is currently an empty placeholder. Settings data, including the selected folder and volume, is stored at `~/.config/musicplayer/settings.json`.

## Linux desktop integration

Build and install the app, its icon, and a matching desktop entry in user-local directories:

```sh
./packaging/install-linux.sh
```

The app can then be launched from the desktop environment's application menu.

## Project layout

- `src/main.rs` starts the application; `src/lib.rs` configures the native window and icon.
- `src/app.rs` owns library state, scanning, and application updates.
- `src/views/top-bar.rs`, `sidepanel.rs`, `home.rs`, `album-view.rs`, `scrubber-controls.rs`, and `dialogs.rs` implement the UI.
- `src/metadata.rs` scans tracks and reads tags and artwork; `src/lib/watcher.rs` watches the selected folder.
- `src/playback.rs` controls `ffplay`; `src/storage/settings.rs` loads and saves settings.
