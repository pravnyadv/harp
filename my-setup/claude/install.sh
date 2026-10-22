#!/usr/bin/env bash
# Installs the Claude Code side of this setup: the /handoff skill, its two scripts, and the tab
# configs Harp and Warp open for it. Existing files are backed up with a timestamp.
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

# One tab config per family color: `handoff` picks the color from the parent session's name.
for app in harp warp; do
  dir="$HOME/.$app/tab_configs"
  mkdir -p "$dir"
  for color in blue magenta cyan green yellow red; do
    cat > "$dir/handoff-$color.toml" <<EOF
# Opened by ~/.claude/bin/handoff via $app://tab_config/handoff-$color (one file per family color).
name = "Handoff ($color)"
color = "$color"

[[panes]]
id = "main"
type = "terminal"
commands = ["$HOME/.claude/bin/handoff-open"]
EOF
  done
done
echo "Installed /handoff (skill, scripts, and tab configs for Harp and Warp)."
