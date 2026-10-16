ALTER TABLE app_settings
ADD COLUMN lastfm_scrobbling_enabled INTEGER NOT NULL DEFAULT 0;

CREATE TABLE lastfm_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    api_key TEXT NOT NULL,
    api_secret TEXT NOT NULL,
    session_key TEXT,
    username TEXT
);