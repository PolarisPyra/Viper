#!/usr/bin/env bash
set -euo pipefail

if ! command -v sudo >/dev/null 2>&1 && (( EUID != 0 )); then
    printf 'Run as root or install sudo to continue.\n' >&2
    exit 1
fi

if (( EUID == 0 )); then
    as_root=()
else
    as_root=(sudo)
fi

"${as_root[@]}" rm -f \
    /usr/bin/viper \
    /usr/share/applications/viper.desktop \
    /usr/share/icons/hicolor/512x512/apps/viper.png

if (( EUID != 0 )) && command -v kbuildsycoca6 >/dev/null 2>&1; then
    kbuildsycoca6 --noincremental
fi

printf 'Viper was removed. Settings and music files were left untouched.\n'
