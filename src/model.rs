use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PaneId(pub String);

impl PaneId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl std::fmt::Display for PaneId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Cell-space rectangle in Herdr-global layout coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

/// Host terminal cell size in pixels, the unit that maps grid cells to a pixel image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellSize {
    pub width_px: u32,
    pub height_px: u32,
}

/// Herdr tab layout snapshot returned by `pane.layout`, trimmed to what Panes needs.
#[derive(Debug, Clone, Deserialize)]
pub struct LayoutSnapshot {
    pub panes: Vec<LayoutPane>,
    #[serde(default)]
    pub focused_pane_id: Option<String>,
    #[serde(default)]
    pub zoomed: bool,
}

/// One pane within a layout snapshot.
#[derive(Debug, Clone, Deserialize)]
pub struct LayoutPane {
    pub pane_id: String,
    pub rect: Rect,
    #[serde(default)]
    pub focused: bool,
}

/// A visible pane assigned a jump label, serialized from the action process into the picker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    pub label: char,
    pub pane_id: PaneId,
    pub rect: Rect,
    pub focused: bool,
}

/// The label alphabet: digits first to preserve the muscle memory of a numeric pane picker,
/// then home-row-ish letters once panes outnumber the digits.
pub const LABELS: &[char] = &[
    '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g',
];

/// Assigns labels to visible panes in reading order (top-to-bottom, then left-to-right).
///
/// Reading order keeps a pane's label stable across invocations as long as the layout is stable,
/// which is what makes the labels memorizable.
pub fn assign_labels(layout: &LayoutSnapshot) -> Vec<Target> {
    let mut panes = layout.panes.clone();
    panes.sort_by_key(|p| (p.rect.y, p.rect.x));
    panes
        .into_iter()
        .zip(LABELS.iter())
        .map(|(pane, &label)| Target {
            label,
            pane_id: PaneId::new(pane.pane_id),
            rect: pane.rect,
            focused: pane.focused,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pane(id: &str, x: u16, y: u16, focused: bool) -> LayoutPane {
        LayoutPane {
            pane_id: id.into(),
            rect: Rect {
                x,
                y,
                width: 40,
                height: 20,
            },
            focused,
        }
    }

    #[test]
    fn labels_follow_reading_order_not_snapshot_order() {
        // Snapshot lists them out of visual order; labels must still read top-left first.
        let layout = LayoutSnapshot {
            panes: vec![
                pane("br", 40, 20, false),
                pane("tl", 0, 0, true),
                pane("tr", 40, 0, false),
                pane("bl", 0, 20, false),
            ],
            focused_pane_id: Some("tl".into()),
            zoomed: false,
        };
        let targets = assign_labels(&layout);
        let by_label: Vec<(char, &str)> = targets
            .iter()
            .map(|t| (t.label, t.pane_id.0.as_str()))
            .collect();
        assert_eq!(
            by_label,
            vec![('1', "tl"), ('2', "tr"), ('3', "bl"), ('4', "br")]
        );
    }

    #[test]
    fn labels_are_capped_at_the_alphabet_size() {
        let panes = (0..30)
            .map(|i| pane(&format!("p{i}"), (i % 5) * 40, (i / 5) * 20, false))
            .collect();
        let layout = LayoutSnapshot {
            panes,
            focused_pane_id: None,
            zoomed: false,
        };
        assert_eq!(assign_labels(&layout).len(), LABELS.len());
    }
}
