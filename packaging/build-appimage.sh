#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
arch=$(uname -m)
if [[ "$arch" != "x86_64" ]]; then
    printf 'AppImage packaging currently supports x86_64; detected %s.\n' "$arch" >&2
    exit 1
fi

tools_dir="${XDG_CACHE_HOME:-$HOME/.cache}/viper/appimage-tools"
linuxdeploy="$tools_dir/linuxdeploy-x86_64.AppImage"
appimage_dir="$repo_root/target/appimage"
appdir="$appimage_dir/AppDir"
output="$appimage_dir/Viper-x86_64.AppImage"
version=$(awk -F '"' '/^version = / { print $2; exit }' "$repo_root/Cargo.toml")

mkdir -p "$tools_dir" "$appimage_dir"
if [[ ! -x "$linuxdeploy" ]]; then
    curl --fail --location --retry 3 \
        https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage \
        --output "$linuxdeploy"
    chmod +x "$linuxdeploy"
fi

cargo build --release --manifest-path "$repo_root/Cargo.toml"
rm -rf "$appdir"

cd "$appimage_dir"
LDAI_OUTPUT="$output" \
LDAI_VERSION="$version" \
APPIMAGE_EXTRACT_AND_RUN=1 "$linuxdeploy" \
    --appdir "$appdir" \
    --executable "$repo_root/target/release/viper" \
    --desktop-file "$repo_root/packaging/viper.desktop" \
    --icon-file "$repo_root/assets/viper.png" \
    --output appimage
printf 'Built %s\n' "$output"
