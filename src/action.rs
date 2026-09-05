//! The `jump` action: read the layout, label the panes, and open the picker popup.
//!
//! This process has no visible surface of its own — it exists only to snapshot the layout and hand
//! it to the popup, which owns the keyboard and does the drawing.

use crate::herdr::{HerdrClient, HerdrContext};
use crate::model::assign_labels;
use anyhow::{Context, Result};
use serde_json::{Map, Value};

/// Plugin id, matched against `herdr-plugin.toml`.
const PLUGIN_ID: &str = "youguanxinqing.herdr-hop";
/// Popup entrypoint id, matched against the `[[panes]]` block in `herdr-plugin.toml`.
const PICKER_ENTRYPOINT: &str = "picker";

pub fn run() -> Result<()> {
    let context = HerdrContext::from_env();
    let socket = context
        .socket_path
        .clone()
        .context("HERDR_SOCKET_PATH is missing; Herdr Hop requires Herdr 0.8.2 or newer")?;
    let client = HerdrClient::new(socket);

    let layout = client.pane_layout(context.target_pane().as_ref())?;

    // A zoomed tab exposes only the focused pane, and a single pane has nowhere to jump.
    if layout.zoomed || layout.panes.len() < 2 {
        return Ok(());
    }

    let targets = assign_labels(&layout);
    let targets_json = serde_json::to_string(&targets).context("failed to serialize targets")?;

    let mut env = Map::new();
    env.insert("HOP_TARGETS_JSON".to_string(), Value::String(targets_json));

    // The popup exists only to own the keyboard; nothing is drawn inside it. Ask for the 6x4
    // minimum Herdr clamps to anyway, so the unavoidable bordered box stays as small as it can be.
    // Use the existing socket connection directly: spawning a second `herdr` process here adds
    // cold-start latency to the keybinding's critical path.
    client
        .open_popup(PLUGIN_ID, PICKER_ENTRYPOINT, 6, 4, env)
        .context("failed to launch Herdr Hop picker popup")
}
