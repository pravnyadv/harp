#!/usr/bin/env bash
# Installs the Claude Code side of this setup: the /handoff skill, its scripts and its Stop hook.
# Existing files are backed up with a timestamp.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
stamp=$(date +%Y%m%d-%H%M%S)
backup() { [[ -e $1 ]] && cp "$1" "$1.$stamp.bak" && echo "backed up $1"; return 0; }

files=(skills/handoff/SKILL.md bin/handoff bin/handoff-open bin/handoff-last hooks/handoff-snapshot.sh)
mkdir -p "$HOME/.claude/skills/handoff" "$HOME/.claude/bin" "$HOME/.claude/hooks"
for file in "${files[@]}"; do
  backup "$HOME/.claude/$file"
  cp "$here/$file" "$HOME/.claude/$file"
  [[ $file == *.md ]] || chmod +x "$HOME/.claude/$file"
done

# The Stop hook that snapshots each reply for handoff-last, registered once.
settings="$HOME/.claude/settings.json"
hook='~/.claude/hooks/handoff-snapshot.sh'
[[ -e $settings ]] || echo '{}' > "$settings"
if ! jq -e --arg c "$hook" '[.hooks.Stop[]?.hooks[]?.command] | index($c)' "$settings" >/dev/null; then
  backup "$settings"
  jq --arg c "$hook" '.hooks.Stop += [{matcher: "", hooks: [{type: "command", command: $c, timeout: 5}]}]' \
    "$settings" > "$settings.tmp" && mv "$settings.tmp" "$settings"
  echo "registered the Stop hook in $settings"
fi

# Older versions kept six fixed tab configs; `handoff` now writes one per task.
rm -f "$HOME"/.{harp,warp}/tab_configs/handoff-{blue,magenta,cyan,green,yellow,red}.toml
# Older versions moved claimed jobs to claimed/; `handoff-open` now deletes a job once it starts it.
rm -rf "$HOME/.claude/handoff/claimed"
echo "Installed /handoff (skill, scripts and Stop hook)."
