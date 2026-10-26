#!/usr/bin/env bash
# Stop hook: keeps each session's last reply in ~/.claude/handoff/last/<session id>.md, which
# handoff-last prints. last_assistant_message is the documented source; the transcript is not.
payload=$(cat)
id=$(jq -r '.session_id // empty' <<<"$payload" 2>/dev/null)
[[ -n $id ]] || exit 0

dir="$HOME/.claude/handoff/last"
mkdir -p "$dir"
tmp="$dir/.$id.$$"
if jq -r '.last_assistant_message // empty' <<<"$payload" > "$tmp" && [[ -s $tmp ]]; then
  mv "$tmp" "$dir/$id.md"
else
  rm -f "$tmp"
fi
find "$dir" -name '*.md' -mtime +30 -delete 2>/dev/null
exit 0
