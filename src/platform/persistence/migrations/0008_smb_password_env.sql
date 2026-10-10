CREATE TABLE samba_settings_without_password (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    share_url TEXT NOT NULL,
    username TEXT NOT NULL DEFAULT '',
    workgroup TEXT NOT NULL DEFAULT ''
);

INSERT INTO samba_settings_without_password (id, share_url, username, workgroup)
SELECT id, share_url, username, workgroup FROM samba_settings;

DROP TABLE samba_settings;
ALTER TABLE samba_settings_without_password RENAME TO samba_settings;
