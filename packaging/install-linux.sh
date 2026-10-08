#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
app_path="$HOME/.local/bin/viper"
icon_path="$HOME/.local/share/icons/hicolor/256x256/apps/viper.png"
desktop_path="$HOME/.local/share/applications/viper.desktop"
legacy_desktop_path="$HOME/.local/share/applications/musicplayer.desktop"

if [ -f "$legacy_desktop_path" ] && ! grep -q '^Hidden=true$' "$legacy_desktop_path"; then
    printf '%s\n' 'Hidden=true' >> "$legacy_desktop_path"
fi

cargo build --release --manifest-path "$repo_root/Cargo.toml"

install -Dm755 "$repo_root/target/release/viper" "$app_path"
install -Dm644 "$repo_root/assets/logo.png" "$icon_path"
install -d "$(dirname -- "$desktop_path")"

escaped_app_path=$(printf '%s' "$app_path" | sed 's/[\\"]/\\&/g')
printf '%s\n' \
    '[Desktop Entry]' \
    'Type=Application' \
    'Name=Viper' \
    'Comment=Browse and play your music library' \
    "Exec=\"$escaped_app_path\"" \
    "Icon=$icon_path" \
    'Terminal=false' \
    'Categories=AudioVideo;Audio;Player;' \
    'StartupWMClass=viper' \
    > "$desktop_path"

printf 'Installed Viper. Log out and back in if the desktop shell does not refresh the icon.\n'
