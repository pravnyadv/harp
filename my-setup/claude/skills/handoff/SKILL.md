---
name: handoff
description: Hand a task to a brand-new Claude Code session that opens in its own Harp (or Warp) tab, so the user can watch it and talk to it while this session keeps going. Use when the user says /handoff, "hand this off", "run this in a new session" or "spin up a session for this", and on your own, without asking, when a separable piece of work needs its own conversation (several rounds of user input or review, back-and-forth messages with this session) or is big enough to crowd this context. Not for one-shot work whose result this session needs next; that is a subagent.
---

# Handoff

`~/.claude/bin/handoff` queues the task and opens a Harp tab (Warp if Harp is not running) that starts it as `claude -n <name>`. The tab is titled `⎿ <name>`, and all children of this session share one tab color.

1. **Your session name:** call `ListAgents`. Its first line says "This session is <name> [...]". Pass that name (without the bracketed ref) as `--parent`.
2. **Child name:** short kebab-case naming the task (`fix-webhook-retry`), at most 20 characters so it fits the sidebar row. It becomes the tab title and the child's address for `SendMessage`.
3. **Mode:** what the user asked for, else `report`.
   - `standalone`: the child never messages this session.
   - `report`: the child messages this session only for blockers, decisions it can't make, and its final result.
   - `interactive`: the user talks to the child directly; it reports only the final result.
4. **Folder:** the project the task belongs to. If the child will edit code and that repo's checkout is on master/main, create the worktree first (per the global branch rules) and pass its path as `--cwd`.
5. **Prompt:** the child starts with none of this conversation. Write a self-contained brief: goal, relevant files and findings so far, constraints, and what "done" means. The script appends the mode rules itself, so don't restate them.
6. **Launch:**

   ```bash
   ~/.claude/bin/handoff --name <name> --parent <your-session> --mode <mode> --cwd <dir> <<'PROMPT'
   <brief>
   PROMPT
   ```

7. Tell the user the tab name and mode in one line. If the script says neither Harp nor Warp is running, it started the session in the background instead; tell them `claude attach <name>` opens it.

Replies from the child arrive here as `<cross-session-message from="<name>">`. Answer one by copying its `from` as your `to`. To hear when the child goes idle, send it `notify_when_idle: true` once; never poll.
