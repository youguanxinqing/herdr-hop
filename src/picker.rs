//! The `pick` popup: paint a label on every target pane, read one keystroke, jump.
//!
//! Runs inside a zero-size focused popup that owns the keyboard for the duration. Because it is a
//! real pane it can measure the host cell size from its own pty (see [`crate::cellsize`]), so it
//! renders correctly the moment it is installed — no terminal re-attach required.

use crate::cellsize;
use crate::herdr::HerdrClient;
use crate::model::{CellSize, Target};
use crate::render::overlay_for;
use anyhow::{Context, Result};
use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

/// How long to wait for the terminal's cell-size report before falling back.
const CELL_SIZE_TIMEOUT: Duration = Duration::from_millis(150);
/// How long the picker waits for the user to choose before giving up and cleaning up.
const KEYSTROKE_TIMEOUT: Duration = Duration::from_secs(20);
/// Used only if the terminal never answers the cell-size query; Herdr scales the image into the
/// pane grid regardless, so labels still appear, just at a guessed aspect ratio.
const FALLBACK_CELL_SIZE: CellSize = CellSize {
    width_px: 10,
    height_px: 20,
};

pub fn run() -> Result<()> {
    // The cursor is already hidden: main() writes `?25l` before clap parses, because beating
    // Herdr's ~20ms cursor-settle window is what keeps the box free of a flashing block glyph.

    let targets: Vec<Target> = serde_json::from_str(
        &std::env::var("HOP_TARGETS_JSON").context("picker launched without HOP_TARGETS_JSON")?,
    )
    .context("failed to parse HOP_TARGETS_JSON")?;

    let socket = std::env::var_os("HERDR_SOCKET_PATH")
        .context("HERDR_SOCKET_PATH is missing in the picker popup")?;
    let client = HerdrClient::new(socket);

    let _raw = RawModeGuard::enable()?;
    let mut stdout = std::io::stdout();
    let pump = InputPump::spawn();

    let cell = measure_cell_size(&mut stdout, || pump.try_next(), CELL_SIZE_TIMEOUT);

    // Paint every label, remembering which panes we touched so cleanup is exact.
    let mut painted: Vec<String> = Vec::with_capacity(targets.len());
    for (index, target) in targets.iter().enumerate() {
        let overlay = overlay_for(target, cell, index);
        if client.graphics_set(&overlay).is_ok() {
            painted.push(target.pane_id.0.clone());
        }
    }

    // The popup itself stays blank on purpose: it exists only to own the keyboard. Painting a
    // prompt or diagnostics into its four-column pty leaves wrapped fragments behind and reads as
    // flicker. The labels on the panes are the whole UI; failed overlays stay visually absent.

    let chosen = read_choice(&pump, &targets);

    // Clear before focusing: focus may move us off this tab, and stale labels there look broken.
    for pane_id in &painted {
        let _ = client.graphics_clear(pane_id);
    }

    if let Some(pane_id) = chosen {
        client
            .focus_pane(&pane_id)
            .with_context(|| format!("failed to focus pane {pane_id}"))?;
    }
    Ok(())
}

/// Measures the host cell or falls back without leaving text on the popup's visible PTY.
fn measure_cell_size<W, F>(out: &mut W, next_byte: F, timeout: Duration) -> CellSize
where
    W: Write,
    F: FnMut() -> Option<u8>,
{
    cellsize::measure_via(out, next_byte, timeout).unwrap_or(FALLBACK_CELL_SIZE)
}

/// Reads keystrokes until a label matches, the user cancels, or the wait times out.
///
/// Escape sequences (arrow keys, a late cell-size report) are drained and ignored rather than being
/// misread as a cancel or a label.
fn read_choice(pump: &InputPump, targets: &[Target]) -> Option<String> {
    loop {
        let byte = pump.next_within(KEYSTROKE_TIMEOUT)?;
        match byte {
            0x03 => return None, // Ctrl-C
            0x1b => match pump.next_within(Duration::from_millis(20)) {
                None => return None, // a lone Esc is a real cancel
                Some(b'[') | Some(b'O') => {
                    // CSI/SS3 sequence: consume through its final byte, then keep waiting.
                    while let Some(next) = pump.next_within(Duration::from_millis(20)) {
                        if (0x40..=0x7e).contains(&next) {
                            break;
                        }
                    }
                }
                Some(_) => {}
            },
            b => {
                let key = (b as char).to_ascii_lowercase();
                if let Some(target) = targets.iter().find(|t| t.label == key) {
                    return Some(target.pane_id.0.clone());
                }
                // Ignore any other key and keep waiting.
            }
        }
    }
}

/// Background reader that pumps raw stdin bytes into a channel, unifying the cell-size report and
/// the keystroke read behind one buffered source.
struct InputPump {
    rx: Receiver<u8>,
}

impl InputPump {
    fn spawn() -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut stdin = std::io::stdin().lock();
            let mut byte = [0u8; 1];
            while let Ok(1) = stdin.read(&mut byte) {
                if tx.send(byte[0]).is_err() {
                    break;
                }
            }
        });
        Self { rx }
    }

    /// Next buffered byte if one is already available, without blocking.
    fn try_next(&self) -> Option<u8> {
        self.rx.try_recv().ok()
    }

    /// Next byte, waiting up to `timeout`.
    fn next_within(&self, timeout: Duration) -> Option<u8> {
        self.rx.recv_timeout(timeout).ok()
    }
}

/// Enables terminal raw mode for the popup's lifetime and restores cooked mode on drop.
struct RawModeGuard;

impl RawModeGuard {
    fn enable() -> Result<Self> {
        crossterm::terminal::enable_raw_mode().context("failed to enable raw mode")?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        // No cursor::Show here on purpose: this pty dies with the process, and un-hiding paints
        // the cursor block back into the popup's teardown frames.
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{PaneId, Rect};

    fn targets() -> Vec<Target> {
        ['1', '2', 'a']
            .iter()
            .map(|&label| Target {
                label,
                pane_id: PaneId::new(format!("w:p{label}")),
                rect: Rect {
                    x: 0,
                    y: 0,
                    width: 40,
                    height: 20,
                },
                focused: false,
            })
            .collect()
    }

    /// Drives `read_choice` from a scripted byte sequence, so the decision logic is tested without
    /// a real terminal.
    fn choose(bytes: &[u8]) -> Option<String> {
        let (tx, rx) = mpsc::channel();
        for &b in bytes {
            tx.send(b).unwrap();
        }
        let pump = InputPump { rx };
        read_choice(&pump, &targets())
    }

    #[test]
    fn a_matching_label_selects_its_pane() {
        assert_eq!(choose(b"2").as_deref(), Some("w:p2"));
        assert_eq!(
            choose(b"A").as_deref(),
            Some("w:pa"),
            "labels are case-insensitive"
        );
    }

    #[test]
    fn ctrl_c_and_lone_escape_cancel() {
        assert_eq!(choose(b"\x03"), None);
        assert_eq!(choose(b"\x1b"), None);
    }

    #[test]
    fn an_arrow_key_is_ignored_then_a_label_still_works() {
        // ESC [ C is right-arrow; it must not cancel, and the following '1' should win.
        assert_eq!(choose(b"\x1b[C1").as_deref(), Some("w:p1"));
    }

    #[test]
    fn an_unknown_label_is_ignored_then_a_valid_one_wins() {
        assert_eq!(choose(b"z1").as_deref(), Some("w:p1"));
    }

    #[test]
    fn cell_size_timeout_keeps_popup_surface_blank() {
        let mut surface = Vec::new();

        let cell = measure_cell_size(&mut surface, || None, Duration::ZERO);

        assert_eq!(cell, FALLBACK_CELL_SIZE);
        assert_eq!(surface, cellsize::QUERY_CELL_SIZE);
    }
}
