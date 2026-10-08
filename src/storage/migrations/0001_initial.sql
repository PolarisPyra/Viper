CREATE TABLE app_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    music_path TEXT,
    window_width REAL,
    window_height REAL,
    startup_view TEXT NOT NULL,
    left_panel_width REAL NOT NULL,
    left_panel_hidden INTEGER NOT NULL,
    right_panel_width REAL NOT NULL,
    volume INTEGER NOT NULL,
    album_sort TEXT NOT NULL,
    sort_ascending INTEGER NOT NULL
);

CREATE TABLE album_state (
    album_key TEXT PRIMARY KEY,
    favorite INTEGER NOT NULL DEFAULT 0,
    rating INTEGER,
    play_count INTEGER,
    last_played INTEGER,
    added INTEGER
);

CREATE TABLE track_metadata (
    path TEXT PRIMARY KEY,
    metadata_version INTEGER NOT NULL,
    file_size INTEGER NOT NULL,
    modified_secs INTEGER NOT NULL,
    modified_nanos INTEGER NOT NULL,
    track BLOB NOT NULL
);
