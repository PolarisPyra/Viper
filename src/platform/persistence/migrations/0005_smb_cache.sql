CREATE TABLE smb_metadata (
    root TEXT NOT NULL,
    path TEXT NOT NULL,
    data BLOB NOT NULL,
    PRIMARY KEY (root, path)
);

CREATE TABLE smb_artwork (
    root TEXT NOT NULL,
    id INTEGER NOT NULL,
    data BLOB NOT NULL,
    PRIMARY KEY (root, id)
);
