ALTER TABLE app_settings
ADD COLUMN shuffle_scope TEXT NOT NULL DEFAULT '"current_album"';
