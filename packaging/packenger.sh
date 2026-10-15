#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
action=${1:-}

usage() {
    printf 'Usage: %s {install|uninstall}\n' "${0##*/}"
}

if (( $# == 1 )) && [[ "$action" == "-h" || "$action" == "--help" ]]; then
    usage
    exit 0
fi

if (( $# != 1 )) || [[ "$action" != "install" && "$action" != "uninstall" ]]; then
    usage >&2
    exit 2
fi

if (( EUID != 0 )) && ! command -v sudo >/dev/null 2>&1; then
    printf 'Run as root or install sudo to continue.\n' >&2
    exit 1
fi

run_privileged() {
    if (( EUID == 0 )); then
        "$@"
    else
        sudo "$@"
    fi
}

refresh_desktop_database() {
    if command -v update-desktop-database >/dev/null 2>&1; then
        run_privileged update-desktop-database /usr/share/applications
    fi
    if (( EUID != 0 )) && command -v kbuildsycoca6 >/dev/null 2>&1; then
        kbuildsycoca6 --noincremental
    fi
}

case "$action" in
    install)
        if ! command -v cargo >/dev/null 2>&1; then
            printf 'Cargo was not found. Install the Rust toolchain before running this script.\n' >&2
            exit 1
        fi

        cargo build --release --locked --manifest-path "$repo_root/Cargo.toml"

        run_privileged install -Dm755 "$repo_root/target/release/viper" /usr/bin/viper
        run_privileged install -Dm644 "$repo_root/packaging/viper.desktop" /usr/share/applications/viper.desktop
        run_privileged install -Dm644 \
            "$repo_root/assets/logo.png" \
            /usr/share/icons/hicolor/512x512/apps/viper.png

        # Remove files installed by older versions of this script.
        run_privileged rm -f /usr/share/viper/logo.png
        if [[ -d /usr/share/viper ]]; then
            run_privileged rmdir --ignore-fail-on-non-empty /usr/share/viper
        fi

        refresh_desktop_database
        printf 'Viper installed. Launch it with `viper` or from your application menu.\n'
        ;;
    uninstall)
        run_privileged rm -f \
            /usr/bin/viper \
            /usr/share/applications/viper.desktop \
            /usr/share/icons/hicolor/512x512/apps/viper.png \
            /usr/share/viper/logo.png
        if [[ -d /usr/share/viper ]]; then
            run_privileged rmdir --ignore-fail-on-non-empty /usr/share/viper
        fi

        refresh_desktop_database
        printf 'Viper was removed. Settings and music files were left untouched.\n'
        ;;
esac
