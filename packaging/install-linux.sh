#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
app_path="$HOME/.local/bin/musicplayer"
icon_path="$HOME/.local/share/icons/hicolor/256x256/apps/musicplayer.png"
desktop_path="$HOME/.local/share/applications/musicplayer.desktop"

cargo build --release --manifest-path "$repo_root/Cargo.toml"

install -Dm755 "$repo_root/target/release/musicplayer" "$app_path"
install -Dm644 "$repo_root/assets/logo.png" "$icon_path"
install -d "$(dirname -- "$desktop_path")"

escaped_app_path=$(printf '%s' "$app_path" | sed 's/[\\"]/\\&/g')
printf '%s\n' \
    '[Desktop Entry]' \
    'Type=Application' \
    'Name=Music Player' \
    'Comment=Browse and play your music library' \
    "Exec=\"$escaped_app_path\"" \
    "Icon=$icon_path" \
    'Terminal=false' \
    'Categories=AudioVideo;Audio;Player;' \
    'StartupWMClass=musicplayer' \
    > "$desktop_path"

printf 'Installed Music Player. Log out and back in if the desktop shell does not refresh the icon.\n'