CREATE TABLE samba_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    share_url TEXT NOT NULL,
    username TEXT NOT NULL DEFAULT '',
    password TEXT NOT NULL DEFAULT '',
    workgroup TEXT NOT NULL DEFAULT ''
);
