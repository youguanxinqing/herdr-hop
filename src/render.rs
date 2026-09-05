//! Turning a label char into a pane graphics overlay: a big centered glyph as a PNG, plus the
//! grid placement that positions it over the pane.

use crate::herdr::{Overlay, Placement};
use crate::model::{CellSize, Rect, Target};
use base64::Engine;
use std::io::Write;

/// Overlay stacking order; well above pane content so labels are never hidden behind text.
const LABEL_Z_INDEX: i32 = 1000;

/// 5x7 bitmaps for every label in [`crate::model::LABELS`]. `1` is an on pixel.
const GLYPHS: &[(char, [&str; 7])] = &[
    (
        '1',
        [
            "00100", "01100", "00100", "00100", "00100", "00100", "01110",
        ],
    ),
    (
        '2',
        [
            "01110", "10001", "00001", "00010", "00100", "01000", "11111",
        ],
    ),
    (
        '3',
        [
            "11110", "00001", "00001", "01110", "00001", "00001", "11110",
        ],
    ),
    (
        '4',
        [
            "00010", "00110", "01010", "10010", "11111", "00010", "00010",
        ],
    ),
    (
        '5',
        [
            "11111", "10000", "10000", "11110", "00001", "00001", "11110",
        ],
    ),
    (
        '6',
        [
            "01110", "10000", "10000", "11110", "10001", "10001", "01110",
        ],
    ),
    (
        '7',
        [
            "11111", "00001", "00010", "00100", "01000", "01000", "01000",
        ],
    ),
    (
        '8',
        [
            "01110", "10001", "10001", "01110", "10001", "10001", "01110",
        ],
    ),
    (
        '9',
        [
            "01110", "10001", "10001", "01111", "00001", "00001", "01110",
        ],
    ),
    (
        'a',
        [
            "01110", "10001", "10001", "11111", "10001", "10001", "10001",
        ],
    ),
    (
        'b',
        [
            "11110", "10001", "10001", "11110", "10001", "10001", "11110",
        ],
    ),
    (
        'c',
        [
            "01111", "10000", "10000", "10000", "10000", "10000", "01111",
        ],
    ),
    (
        'd',
        [
            "11110", "10001", "10001", "10001", "10001", "10001", "11110",
        ],
    ),
    (
        'e',
        [
            "11111", "10000", "10000", "11110", "10000", "10000", "11111",
        ],
    ),
    (
        'f',
        [
            "11111", "10000", "10000", "11110", "10000", "10000", "10000",
        ],
    ),
    (
        'g',
        [
            "01111", "10000", "10000", "10011", "10001", "10001", "01110",
        ],
    ),
];

/// Distinct bright colors so adjacent panes' labels never blur together. `[r, g, b, a]`.
const COLORS: &[[u8; 4]] = &[
    [255, 92, 92, 245],
    [64, 156, 255, 245],
    [245, 191, 79, 245],
    [80, 220, 190, 245],
    [220, 130, 245, 245],
    [130, 220, 120, 245],
];

fn glyph_for(label: char) -> &'static [&'static str; 7] {
    GLYPHS
        .iter()
        .find(|(c, _)| *c == label)
        .map(|(_, bitmap)| bitmap)
        .unwrap_or(&GLYPHS[0].1)
}

/// Builds the overlay for one labeled pane, `index` selecting its color.
pub fn overlay_for(target: &Target, cell: CellSize, index: usize) -> Overlay {
    let plan = plan_placement(target.rect, cell);
    let color = COLORS[index % COLORS.len()];
    let png = encode_png(&draw_label(
        target.label,
        plan.image_width,
        plan.image_height,
        color,
    ));
    Overlay {
        pane_id: target.pane_id.0.clone(),
        format: "png",
        image_width: plan.image_width,
        image_height: plan.image_height,
        data_base64: base64::engine::general_purpose::STANDARD.encode(png),
        z_index: LABEL_Z_INDEX,
        placement: plan.placement,
    }
}

struct Plan {
    placement: Placement,
    image_width: u32,
    image_height: u32,
}

/// A label occupies a centered block of the pane, sized to a fraction of it and clamped so it stays
/// bold on huge panes and legible on tiny ones.
fn plan_placement(rect: Rect, cell: CellSize) -> Plan {
    let pane_cols = rect.width.max(1) as u32;
    let pane_rows = rect.height.max(1) as u32;
    let grid_rows = ((pane_rows * 68 / 100).max(5)).min(pane_rows).min(20);
    let grid_cols = ((grid_rows * 72 / 100).max(4)).min(pane_cols).min(16);
    let viewport_col = ((pane_cols.saturating_sub(grid_cols)) / 2) as i32;
    let viewport_row = ((pane_rows.saturating_sub(grid_rows)) / 2) as i32;
    Plan {
        placement: Placement {
            viewport_col,
            viewport_row,
            grid_cols,
            grid_rows,
        },
        image_width: grid_cols * cell.width_px,
        image_height: grid_rows * cell.height_px,
    }
}

/// Draws the glyph, scaled to fill and centered, into a fresh transparent RGBA buffer.
fn draw_label(label: char, width: u32, height: u32, color: [u8; 4]) -> Rgba {
    let mut image = Rgba::new(width, height);
    let glyph = glyph_for(label);
    let (glyph_cols, glyph_rows) = (5u32, 7u32);
    let scale = ((width / glyph_cols).min(height / glyph_rows)).max(1);
    let glyph_w = glyph_cols * scale;
    let glyph_h = glyph_rows * scale;
    let origin_x = (width.saturating_sub(glyph_w)) / 2;
    let origin_y = (height.saturating_sub(glyph_h)) / 2;
    for (row, line) in glyph.iter().enumerate() {
        for (col, pixel) in line.bytes().enumerate() {
            if pixel == b'1' {
                image.fill_rect(
                    origin_x + col as u32 * scale,
                    origin_y + row as u32 * scale,
                    scale,
                    scale,
                    color,
                );
            }
        }
    }
    image
}

/// A row-major RGBA pixel buffer.
struct Rgba {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl Rgba {
    fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: vec![0; (width * height * 4) as usize],
        }
    }

    fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, color: [u8; 4]) {
        for row in y..(y + h).min(self.height) {
            for col in x..(x + w).min(self.width) {
                let offset = ((row * self.width + col) * 4) as usize;
                self.data[offset..offset + 4].copy_from_slice(&color);
            }
        }
    }
}

/// Encodes an RGBA buffer as a PNG (8-bit, color type 6), zlib-compressing the scanlines.
fn encode_png(image: &Rgba) -> Vec<u8> {
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&image.width.to_be_bytes());
    ihdr.extend_from_slice(&image.height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);

    let stride = (image.width * 4) as usize;
    let mut raw = Vec::with_capacity((stride + 1) * image.height as usize);
    for row in image.data.chunks_exact(stride) {
        raw.push(0); // filter type: none
        raw.extend_from_slice(row);
    }

    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(&raw).expect("zlib write to Vec");
    let idat = encoder.finish().expect("zlib finish");

    let mut png = vec![0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
    write_chunk(&mut png, b"IHDR", &ihdr);
    write_chunk(&mut png, b"IDAT", &idat);
    write_chunk(&mut png, b"IEND", &[]);
    png
}

fn write_chunk(png: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    png.extend_from_slice(&(data.len() as u32).to_be_bytes());
    png.extend_from_slice(kind);
    png.extend_from_slice(data);
    let mut crc = flate2::Crc::new();
    crc.update(kind);
    crc.update(data);
    png.extend_from_slice(&crc.sum().to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::PaneId;

    fn target(label: char, w: u16, h: u16) -> Target {
        Target {
            label,
            pane_id: PaneId::new("w:p1"),
            rect: Rect {
                x: 0,
                y: 0,
                width: w,
                height: h,
            },
            focused: false,
        }
    }

    #[test]
    fn placement_centers_the_label_block_in_the_pane() {
        let plan = plan_placement(
            Rect {
                x: 0,
                y: 0,
                width: 80,
                height: 24,
            },
            CellSize {
                width_px: 17,
                height_px: 40,
            },
        );
        // symmetric margins on each axis
        assert_eq!(
            plan.placement.viewport_col as u32 * 2 + plan.placement.grid_cols,
            80 - (80 - plan.placement.grid_cols) % 2
        );
        assert_eq!(plan.image_width, plan.placement.grid_cols * 17);
        assert_eq!(plan.image_height, plan.placement.grid_rows * 40);
    }

    #[test]
    fn placement_never_exceeds_a_small_pane() {
        let plan = plan_placement(
            Rect {
                x: 0,
                y: 0,
                width: 6,
                height: 5,
            },
            CellSize {
                width_px: 10,
                height_px: 20,
            },
        );
        assert!(plan.placement.grid_cols <= 6);
        assert!(plan.placement.grid_rows <= 5);
        assert!(plan.placement.viewport_col >= 0 && plan.placement.viewport_row >= 0);
    }

    #[test]
    fn overlay_png_is_valid_and_within_the_size_budget() {
        let overlay = overlay_for(
            &target('1', 200, 60),
            CellSize {
                width_px: 17,
                height_px: 40,
            },
            0,
        );
        let png = base64::engine::general_purpose::STANDARD
            .decode(&overlay.data_base64)
            .unwrap();
        assert_eq!(&png[..8], &[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
        assert!(png.len() < 512 * 1024, "PNG must fit Herdr's set limit");
        assert_eq!(overlay.format, "png");
        assert_eq!(overlay.z_index, LABEL_Z_INDEX);
    }

    #[test]
    fn every_label_has_a_glyph() {
        for &label in crate::model::LABELS {
            assert!(
                GLYPHS.iter().any(|(c, _)| *c == label),
                "missing glyph for label {label}"
            );
        }
    }

    #[test]
    fn drawn_glyph_actually_marks_pixels() {
        let image = draw_label('8', 100, 140, [255, 0, 0, 255]);
        assert!(
            image.data.chunks(4).any(|p| p == [255, 0, 0, 255]),
            "a drawn glyph must set at least one colored pixel"
        );
    }
}
