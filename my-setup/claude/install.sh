#!/usr/bin/env bash
# Installs the Claude Code side of this setup: the /handoff skill and its two scripts.
# Existing files are backed up with a timestamp.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
stamp=$(date +%Y%m%d-%H%M%S)
backup() { [[ -e $1 ]] && cp "$1" "$1.$stamp.bak" && echo "backed up $1"; return 0; }

mkdir -p "$HOME/.claude/skills/handoff" "$HOME/.claude/bin"
for file in skills/handoff/SKILL.md bin/handoff bin/handoff-open; do
  backup "$HOME/.claude/$file"
  cp "$here/$file" "$HOME/.claude/$file"
done
chmod +x "$HOME/.claude/bin/handoff" "$HOME/.claude/bin/handoff-open"

# Older versions kept six fixed tab configs; `handoff` now writes one per task.
rm -f "$HOME"/.{harp,warp}/tab_configs/handoff-{blue,magenta,cyan,green,yellow,red}.toml
echo "Installed /handoff (skill and scripts)."
