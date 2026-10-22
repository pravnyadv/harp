#!/usr/bin/env bash
# Puts this fork's personal config in place: ~/.harp settings and theme, and `harp-build` on PATH.
# Existing files are backed up next to themselves with a timestamp.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
stamp=$(date +%Y%m%d-%H%M%S)
backup() { [[ -e $1 ]] && cp "$1" "$1.$stamp.bak" && echo "backed up $1"; return 0; }

mkdir -p "$HOME/.harp/themes" "$HOME/.local/bin"
backup "$HOME/.harp/settings.toml"
sed "s#__HOME__#$HOME#g" "$here/settings.toml" > "$HOME/.harp/settings.toml"
cp "$here/themes/iterm-oceanic.yaml" "$HOME/.harp/themes/"
ln -sf "$here/harp-build" "$HOME/.local/bin/harp-build"
chmod +x "$here/harp-build"

echo "Installed ~/.harp/settings.toml, ~/.harp/themes/iterm-oceanic.yaml and ~/.local/bin/harp-build."
[[ ":$PATH:" == *":$HOME/.local/bin:"* ]] || echo "Add ~/.local/bin to PATH to run harp-build directly."
