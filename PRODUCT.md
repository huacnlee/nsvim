# Product

<!-- impeccable:product-schema 1 -->

## Platform

web (this record covers the promo site). The product itself is a native desktop app for Linux
macOS and Windows, built with NSH.

## Stack

Website: Astro, static output, deployed to GitHub Pages from `website/` by a GitHub Actions
workflow (confirmed by the user).

## Users

All developers who open code to read it, find their way around it and make quick edits:
Neovim users who want their motions in a fast GUI, people who prefer classic shortcuts, and
Linux desktop users (Omarchy among them). The site speaks to all of them; neither keymap leads.

## Product Purpose

editor opens code fast, makes it pleasant to read, and makes quick edits quick. It is not an
IDE and not for writing all day. Success: a developer reaches for it for every "look at this
file" moment, and it opens before they finish the thought.

## Positioning

A GPU-rendered native editor that keeps real Vim semantics (or no modes at all, with multiple
cursors) and looks deliberately designed: 16 themes, live Omarchy theming, a five-step setup,
Settings as a buffer. Speed is the deciding rule for every feature.

## Operating Context

Launched from a terminal (`editor .`, `editor file:42`), a launcher, `$EDITOR` and git commit
messages. Used alongside git checkouts and AI agents that change files underneath it.

## Capabilities and Constraints

- Neovim keymap: counts, operators, text objects, dot repeat, registers, macros, marks,
  Replace and Visual block modes, `:s` with confirm, `:g`.
- Classic keymap: Ctrl or ⌘ shortcuts, multiple cursors (`Ctrl D`, Alt-click).
- Tree-sitter text objects and motions in 13 languages.
- LSP: diagnostics, definition, references, hover, completion, rename, code actions.
- Fuzzy finder, ripgrep project search, file tree with git status, leader menu.
- Crash recovery, undo across restarts, reload on outside changes, atomic saves.
- AI agents drive a running editor over `editor ctl` and `editor mcp`: read the selection and
  unsaved buffers, point at lines, propose changes shown as a diff (`y` accepts). No AI runs
  inside the editor.
- 16 themes; follows the Omarchy desktop theme live.
- Release binaries: Linux x86-64 and ARM64, macOS Apple silicon and Intel (unsigned on
  macOS). The website is at https://huacnlee.github.io/nsvim/. Install also through `cargo install --git`.
- Not built: tree-sitter highlighting, `gr`, `gF`.

## Brand Commitments

- Name: `editor`, lowercase.
- Tagline: "Read fast. Edit quick."
- Icon: a letter N on a dark tile. The left bar is light, a grey stroke joins the bars, and the
  right bar is an orange block cursor, the Command mode colour. The website lets the cursor
  cycle through the mode colours. It takes nothing from the Neovim logo.
  `assets/editor.svg`, and `assets/editor-light.svg` for light backgrounds.
- Voice: plain, short, technical, no hype (ASD-STE100 style).

## Evidence on Hand

- Real app screenshots, rendered headless from the app: `docs/media/*.png` (editor, finder,
  leader, settings, setup, setup-look, light, omarchy, multi, menu). More can be rendered
  from scenes with `editor --snap`.
- Demo GIF `docs/media/demo.gif` and promo video `docs/media/editor-promo.mp4`.
- Measured numbers: a 100 MB file opens in 175 ms; 0.3 ms median and 1.4 ms p95 CPU per
  keystroke frame; an idle window draws nothing.
- No testimonials, users, stars, press or download counts exist. Do not invent any.

## Product Principles

1. Speed decides. A feature that costs latency does not ship.
2. Keep muscle memory. Real Vim semantics, or real classic shortcuts, never a half of each.
3. Read first. The default view is for reading code; editing is quick and exact.
4. Show, don't claim. Real screenshots and measured numbers only.

## Accessibility & Inclusion

Text contrast 4.5:1, keyboard reachable controls, reduced-motion support, alt text on every
product image.
