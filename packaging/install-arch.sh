#!/usr/bin/env bash
set -euo pipefail

# Build and install Viper from this checkout using Arch Linux packages and
# the same system locations an AUR package would use.
repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)

if ! command -v sudo >/dev/null 2>&1 && (( EUID != 0 )); then
    printf 'Run as root or install sudo to continue.\n' >&2
    exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
    printf 'Cargo was not found. Install the Rust toolchain before running this script.\n' >&2
    exit 1
fi

if (( EUID == 0 )); then
    as_root=()
else
    as_root=(sudo)
fi

cargo build --release --locked --manifest-path "$repo_root/Cargo.toml"

"${as_root[@]}" install -Dm755 "$repo_root/target/release/viper" /usr/bin/viper
"${as_root[@]}" install -Dm644 "$repo_root/packaging/viper.desktop" /usr/share/applications/viper.desktop
"${as_root[@]}" install -Dm644 "$repo_root/assets/viper.png" /usr/share/icons/hicolor/512x512/apps/viper.png

printf 'Viper installed. Launch it with `viper` or from your application menu.\n'
