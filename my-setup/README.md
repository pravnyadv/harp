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
