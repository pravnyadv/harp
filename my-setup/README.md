# My Harp setup

Personal config for this fork: settings, theme, and a fast build script. Everything here is
mine, not upstream Harp's.

## Set up a new Mac

1. Toolchain (once):
   - Xcode, then `xcodebuild -downloadComponent MetalToolchain`
   - `brew install rustup jq protobuf pkgconf` (the repo's `rust-toolchain.toml` pins Rust 1.92.0)
   - Homebrew's rustup is keg-only: `export PATH="$(brew --prefix rustup)/bin:$HOME/.cargo/bin:$PATH"`
     (add it to `~/.zshrc`), then `rustup default stable`
   - `cargo install cargo-bundle --version 0.11.0 --locked`
2. `git clone -b theme-sidebar https://github.com/pravnyadv/harp.git && cd harp`
3. `./my-setup/install.sh` copies the settings and theme to `~/.harp` and links `harp-build`.
4. `harp-build` builds and installs `/Applications/Harp.app`. The first build compiles every
   dependency and takes much longer; after that a change rebuilds in about 3 minutes.
5. Open Harp from the Dock or Applications, never from a terminal running Claude Code: Harp would
   inherit that session's `CLAUDE_*` variables, and Claude sessions in it would stop saving
   transcripts.

## What the settings do

All live in `~/.harp/settings.toml` and apply on save, except `sidebar_font`.

| Setting | Mine | Effect |
|---|---|---|
| `appearance.themes.theme` | iTerm Oceanic | My iTerm2 colors (`themes/iterm-oceanic.yaml`). |
| `appearance.text.font_name` / `font_size` | Source Code Pro for Powerline, 14 | Same as iTerm2. Install the font first. |
| `appearance.vertical_tabs.sidebar_palette` | `theme` | Sidebar colors are shades of the theme background instead of Harp's greys. |
| `appearance.vertical_tabs.sidebar_glass` | `false` | Solid sidebar instead of frosted glass. |
| `appearance.vertical_tabs.sidebar_subtitle` | `directory` | Second line under each row title: the path. Also `branch_and_directory`, `none`. |
| `appearance.vertical_tabs.sidebar_title_size` | `14` | Row title size (10 to 20). |
| `appearance.vertical_tabs.sidebar_row_style` | `inset_group` | Each group's rows share one rounded card. Also `dividers`, `plain`. |
| `appearance.vertical_tabs.sidebar_row_gap` | `1` | Space on each side of the row divider. |
| `appearance.vertical_tabs.auto_group` | `project` | Ungrouped tabs join a group named after their git repo (worktrees fold into their repo). |
| `appearance.vertical_tabs.display_granularity` | `tabs` | One row per tab, no pane belt. |
| `appearance.vertical_tabs.card_padding` | `0` | No gap left of the terminal inside the card. |
| `appearance.vertical_tabs.sidebar_colors` | unset | Override any sidebar color by token name, e.g. `{ row_fill = "#1f2f38" }`. |
| `appearance.vertical_tabs.sidebar_font` | unset | Sidebar font family (restart to apply). |
| `appearance.blocks.shade_restored_blocks` | `false` | Output from the previous session looks like new output. |
| `appearance.blocks.selected_block_fill` | `6` | Faint tint on a selected block (Harp default 25; 0 keeps only the border). |
| `general.snackbar_enabled` | `true` | Keeps the command pinned at the top while scrolling long output. |
| `general.resume_agent_sessions` | `true` | Tabs that had a live Claude session when Harp quit run `claude --resume <id>` on restore. The ids are kept in `~/.harp/agent_sessions.json`. Needs the Warp plugin below. |

The default Warp-mode prompt also changed: repository name, branch, branch status, Node version
and PHP version (only in a Composer project) sit before the path, like the Spaceship prompt. If a
prompt was customized earlier, open the prompt editor and pick "Restore default" to see it.

## Claude Code side

These live outside Harp, in `~/.claude`, `~/.claude.json` and `~/.zshrc`. Open Claude Code in this
repo on the new Mac and paste the prompt below; it does each step and reports back.

```text
Set up my Claude Code side from my-setup/README.md in this repo. Do each step, show me what changed,
and back up any file before editing it:
1. Run ./my-setup/claude/install.sh (installs the /handoff skill and its two scripts).
2. Install Warp's Claude Code plugin, which Harp uses for each tab's working / needs-you / done
   status: `claude plugin marketplace add warpdotdev/claude-code-warp`, then
   `claude plugin install warp@claude-code-warp`. Tell me to run /reload-plugins in open sessions.
3. In ~/.claude/settings.json, remove any hook whose command runs `afplay` (notification sounds),
   keeping every other hook in the same groups. Check the file is still valid JSON.
4. In ~/.zshrc, right after the line `source $ZSH/oh-my-zsh.sh`, add these two lines so tab
   titles show the folder name instead of user@host (skip if oh-my-zsh is not used):
     ZSH_THEME_TERM_TITLE_IDLE="%1~"
     ZSH_THEME_TERM_TAB_TITLE_IDLE="%1~"
5. Make new shells start fast (Harp shows "Starting zsh" until ~/.zshrc finishes). First time
   `zsh -i -c exit` three times with /usr/bin/time and note the numbers. Then:
   a. If ~/.zshrc loads nvm with `\. "$NVM_DIR/nvm.sh"`, replace the NVM_DIR export and the
      nvm.sh / bash_completion lines with my-setup/shell/nvm-lazy.zsh, verbatim.
   b. If ~/.zshrc runs `brew ls --versions` at startup to list PHP versions into
      `installedPhpVersions`, replace that one line with my-setup/shell/php-versions.zsh, and
      check `echo $installedPhpVersions` gives the same list as before.
   c. If oh-my-zsh is used: run `omz update` once, then put my-setup/shell/omz-update.zsh above
      the `source $ZSH/oh-my-zsh.sh` line (replacing any active `zstyle ':omz:update'` lines).
   d. For anything else slow, profile with `PS4='+%D{%s.%6.} %N:%i> ' zsh -i -x -c exit` and show
      me the slowest lines before changing them.
   Time it again and show me before and after. Check `node -v` and `nvm current` still work.
6. In ~/.claude/settings.json, time each SessionStart hook (pipe it a small JSON like
   {"session_id":"x","cwd":"<a repo>","source":"startup"}). For any over ~300ms, such as
   ~/.claude/hooks/mr-health-after-push.sh, add "async": true to that hook entry: Claude's first
   reply otherwise waits for it.
7. Run `claude mcp list`. Show me any server that fails to connect (on the office laptop the
   sonarqube Docker server failed in every session) and remove it only if I agree.
```

What each piece does:

| Piece | Where | Effect |
|---|---|---|
| `/handoff` skill | `~/.claude/skills/handoff/SKILL.md` | "hand this off" starts a separate Claude session for a task, in its own tab. |
| `handoff` | `~/.claude/bin/handoff` | Queues the task, then opens a `harp://` tab in the task folder through a one-off tab config (Warp if Harp is closed, a background session if neither runs). Children of one session share a tab color. It records who started whom under `~/.claude/handoff/parents` and `children`, which Harp's sidebar reads: the parent row gets a `◉`, a child row reads `from <parent>` (plus its folder when that differs), and a new child's tab opens right after its parent's. |
| `handoff-open` | `~/.claude/bin/handoff-open` | Runs in the new tab: claims the task, titles the tab `⎿ <name>`, starts `claude -n <name>`. |
| Warp plugin | Claude Code plugins | Sends each session's status to Harp's sidebar. |
| No sound hooks | `~/.claude/settings.json` | Silent Claude notifications. |
| Tab titles | `~/.zshrc` | Shells show their folder name. |
| Fast shells | `~/.zshrc`, `my-setup/shell/` | nvm loads on first use, PHP versions come from the Cellar instead of `brew`, oh-my-zsh updates daily in the background. On the office laptop a shell went from 0.85s to 0.05s. |
| Async hooks | `~/.claude/settings.json` | Slow SessionStart hooks no longer hold up Claude's first reply. |
