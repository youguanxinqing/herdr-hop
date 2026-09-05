## Code documentation

- Add SUCCINCT docblocks to structs and methods.
- Give an added focus to Domain definitions.
- Do NOT document things that are painfully obvious.
- If an implication is particularly complicated, document implementation sections.

## Verifying (MUST BE RUN BEFORE CONSIDERING A TASK COMPLETE)

- `just verify` — runs all three checks below:
    - `cargo fmt --all -- --check`
    - `cargo test --all-features`
    - `cargo clippy --all-targets --all`
- `just build` stages the release binary for the linked plugin (`scripts/build.sh` stays the
  Herdr install entrypoint so plugin consumers never need just).
- Any live, end-to-end testing can be done with the `herdr` CLI.

## Architecture

Two subcommands, one binary:

- `jump` (action) — launched by Herdr from the keybinding. Reads `pane.layout`, assigns a label
  to each visible pane, and opens the picker popup, passing the labeled panes in via env.
- `pick` (popup pane) — runs inside a zero-size focused popup that owns the keyboard. It measures
  the host cell size by querying its OWN pty (`ESC [ 16 t`), paints one big label onto each target
  pane via `pane.graphics.set`, reads a single keystroke, then `pane.focus`es the chosen pane and
  clears every overlay.

The cell-size self-query is deliberate: measuring via the popup's own pty (rather than Herdr's
global `pane.graphics.info`) is what lets the plugin render immediately after install without
re-attaching the terminal — Herdr answers `ESC [ 16 t` from the pane's own cell geometry, which is
known regardless of when the client attached or when kitty_graphics was enabled.
