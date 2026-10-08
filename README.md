# Viper

A desktop music player built with Rust and egui, centered on an album-art-focused browsing experience for exploring your library and playing tracks with Symphonia-backed decoding.

## Requirements

- Rust and Cargo

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
- Read audio metadata and embedded artwork with Lofty and Symphonia.

Preferences let you choose your music folder and startup page. Preferences and album state are stored in SQLite at `~/.config/viper/viper.sqlite3`. Existing settings databases are copied from the previous `~/.config/viper/` location on first launch.

## Linux desktop integration

Build and install the app, its icon, and a matching desktop entry in user-local directories:

```sh
./packaging/install-linux.sh
```

The app can then be launched from the desktop environment's application menu.

## Project layout

- `src/main.rs` starts the application; `src/app/initialization.rs` configures fonts and the native window, then creates `ViperApp`.
- `src/app/` coordinates application state and initial library loading in `library_loader.rs` and filesystem updates in `library_watcher.rs`.
- `src/views/home.rs` and `src/views/album_grid.rs` render the Home and Albums pages; album details and track rows live under `src/components/`.
- `src/library/` owns models, `sorting`, `scanner`, `modifications`, audio-file helpers, tag reading, metadata caching, and folder watching.
- `src/artwork/` scans for and prepares cover art in `scanner.rs`, and manages decoded artwork textures in `cache.rs`.
- `src/playback/player.rs` handles Rodio playback with Symphonia decoding; `src/storage/` stores settings and metadata cache records in SQLite.
