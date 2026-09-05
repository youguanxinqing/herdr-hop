//! Host cell size measured by querying our OWN pty, not Herdr's global graphics state.
//!
//! The picker runs inside a real pane, so writing `ESC [ 16 t` to stdout makes Herdr answer
//! `ESC [ 6 ; height ; width t` from that pane's own cell geometry. That geometry is known
//! regardless of when the terminal attached or when kitty_graphics was toggled — which is exactly
//! why this path renders immediately after install, without the "open a new terminal first" dance
//! that biting the global `pane.graphics.info` state causes.

use crate::model::CellSize;
use std::io::Write;
use std::time::{Duration, Instant};

/// XTerm window op: report the cell size in pixels. Herdr answers with a `CSI 6 ; h ; w t` report.
pub const QUERY_CELL_SIZE: &[u8] = b"\x1b[16t";

/// Parses a `CSI 6 ; height ; width t` cell-size report out of a byte buffer.
///
/// The report may be preceded or followed by unrelated bytes, so this scans for the introducer
/// rather than anchoring at the start. Note the wire order is height then width.
pub fn parse_report(buf: &[u8]) -> Option<CellSize> {
    let start = find_subsequence(buf, b"\x1b[6;")?;
    let rest = &buf[start + 4..];
    let semicolon = rest.iter().position(|&b| b == b';')?;
    let terminator = rest.iter().position(|&b| b == b't')?;
    if semicolon >= terminator {
        return None;
    }
    let height: u32 = std::str::from_utf8(&rest[..semicolon]).ok()?.parse().ok()?;
    let width: u32 = std::str::from_utf8(&rest[semicolon + 1..terminator])
        .ok()?
        .parse()
        .ok()?;
    if width == 0 || height == 0 {
        return None;
    }
    Some(CellSize {
        width_px: width,
        height_px: height,
    })
}

/// Writes the query, then pulls bytes from `next_byte` until a report parses or `timeout` elapses.
///
/// `next_byte` returns `None` when no byte is available yet; this lets a caller wire it to a
/// non-blocking or channel-backed stdin without this module knowing about the transport.
pub fn measure_via<W, F>(out: &mut W, mut next_byte: F, timeout: Duration) -> Option<CellSize>
where
    W: Write,
    F: FnMut() -> Option<u8>,
{
    out.write_all(QUERY_CELL_SIZE).ok()?;
    out.flush().ok()?;

    let deadline = Instant::now() + timeout;
    let mut buf = Vec::with_capacity(32);
    while Instant::now() < deadline {
        match next_byte() {
            Some(byte) => {
                buf.push(byte);
                if byte == b't' {
                    if let Some(size) = parse_report(&buf) {
                        return Some(size);
                    }
                }
            }
            None => std::thread::sleep(Duration::from_millis(1)),
        }
    }
    None
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_height_then_width_report() {
        let size = parse_report(b"\x1b[6;40;17t").unwrap();
        assert_eq!(
            size,
            CellSize {
                width_px: 17,
                height_px: 40
            }
        );
    }

    #[test]
    fn finds_report_embedded_in_surrounding_bytes() {
        let size = parse_report(b"junk\x1b[6;20;10ttrailing").unwrap();
        assert_eq!(
            size,
            CellSize {
                width_px: 10,
                height_px: 20
            }
        );
    }

    #[test]
    fn rejects_zero_dimensions_and_garbage() {
        assert!(parse_report(b"\x1b[6;0;10t").is_none());
        assert!(parse_report(b"\x1b[4;432;720t").is_none()); // pixel-area report, not cell size
        assert!(parse_report(b"no report here").is_none());
    }

    #[test]
    fn measure_reads_report_from_byte_stream() {
        let reply = b"\x1b[6;40;17t";
        let mut feed = reply.iter().copied();
        let mut out = Vec::new();
        let size = measure_via(&mut out, || feed.next(), Duration::from_millis(200)).unwrap();
        assert_eq!(
            size,
            CellSize {
                width_px: 17,
                height_px: 40
            }
        );
        assert_eq!(out, QUERY_CELL_SIZE, "the query must actually be written");
    }

    #[test]
    fn measure_times_out_when_no_report_arrives() {
        let mut out = Vec::new();
        let size = measure_via(&mut out, || None, Duration::from_millis(20));
        assert!(size.is_none());
    }
}
