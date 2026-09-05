use crate::herdr::socket::UnixSocketTransport;
use crate::model::{LayoutSnapshot, PaneId};
use anyhow::{bail, Context, Result};
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A pane graphics overlay ready to submit: a pixel image plus where it sits in the pane grid.
#[derive(Debug, Clone, Serialize)]
pub struct Overlay {
    pub pane_id: String,
    pub format: &'static str,
    pub image_width: u32,
    pub image_height: u32,
    pub data_base64: String,
    pub z_index: i32,
    pub placement: Placement,
}

/// Where an overlay sits, in pane grid cells.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Placement {
    pub viewport_col: i32,
    pub viewport_row: i32,
    pub grid_cols: u32,
    pub grid_rows: u32,
}

/// Herdr socket client scoped to the calls Panes makes.
#[derive(Debug, Clone)]
pub struct HerdrClient {
    transport: UnixSocketTransport,
}

impl HerdrClient {
    pub fn new(socket_path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            transport: UnixSocketTransport::new(socket_path),
        }
    }

    /// Fetches the current tab layout, or a specific pane's tab when `pane` is given.
    pub fn pane_layout(&self, pane: Option<&PaneId>) -> Result<LayoutSnapshot> {
        let params = match pane {
            Some(p) => json!({ "pane_id": p.0 }),
            None => json!({ "current": true }),
        };
        let value = self.call("pane.layout", params)?;
        let layout = value
            .get("layout")
            .cloned()
            .context("pane.layout response had no layout")?;
        serde_json::from_value(layout).context("failed to decode layout snapshot")
    }

    /// Paints one overlay onto its pane.
    pub fn graphics_set(&self, overlay: &Overlay) -> Result<()> {
        self.call("pane.graphics.set", serde_json::to_value(overlay)?)
            .map(|_| ())
    }

    /// Removes the primary-layer overlay from a pane. Best-effort by contract at the call site.
    pub fn graphics_clear(&self, pane_id: &str) -> Result<()> {
        self.call("pane.graphics.clear", json!({ "pane_id": pane_id }))
            .map(|_| ())
    }

    /// Focuses a pane by id. Crosses tabs on its own, so no separate tab focus is needed.
    pub fn focus_pane(&self, pane_id: &str) -> Result<()> {
        self.call("pane.focus", json!({ "pane_id": pane_id }))
            .map(|_| ())
    }

    /// Opens a focused popup plugin pane. Done over the socket directly rather than by spawning the
    /// `herdr` CLI, so the wake path never pays a second process cold start.
    pub fn open_popup(
        &self,
        plugin_id: &str,
        entrypoint: &str,
        width: u16,
        height: u16,
        env: serde_json::Map<String, Value>,
    ) -> Result<()> {
        self.call(
            "plugin.pane.open",
            json!({
                "plugin_id": plugin_id,
                "entrypoint": entrypoint,
                "placement": "popup",
                "width": width,
                "height": height,
                "focus": true,
                "env": env,
            }),
        )
        .map(|_| ())
    }

    /// Sends one request, checks the id echo, and unwraps the result or a declared error.
    fn call(&self, method: &str, params: Value) -> Result<Value> {
        let id = format!(
            "panes-{}-{}",
            std::process::id(),
            REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let response = self
            .transport
            .exchange(&json!({ "id": id, "method": method, "params": params }))?;
        decode(response, &id, method)
    }
}

fn decode(response: Value, id: &str, method: &str) -> Result<Value> {
    match response.get("id").and_then(Value::as_str) {
        Some(got) if got == id => {}
        Some(got) => bail!("Herdr response id mismatch on {method}: expected {id}, got {got}"),
        None => bail!("Herdr response to {method} had no id"),
    }
    if let Some(error) = response.get("error") {
        let code = error
            .get("code")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("Herdr request failed");
        bail!("Herdr error on {method}: {code}: {message}");
    }
    response
        .get("result")
        .cloned()
        .with_context(|| format!("Herdr response to {method} had neither result nor error"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;
    use std::thread;

    #[test]
    fn decode_returns_result_on_id_match() {
        let ok = decode(
            json!({"id": "a", "result": {"layout": {}}}),
            "a",
            "pane.layout",
        )
        .unwrap();
        assert!(ok.get("layout").is_some());
    }

    #[test]
    fn decode_surfaces_declared_error_code_and_message() {
        let err = decode(
            json!({"id": "a", "error": {"code": "feature_disabled", "message": "off"}}),
            "a",
            "pane.graphics.set",
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("feature_disabled") && err.contains("off"));
    }

    #[test]
    fn decode_rejects_id_mismatch() {
        assert!(decode(json!({"id": "b", "result": {}}), "a", "pane.focus").is_err());
    }

    #[test]
    fn overlay_serializes_without_owner_field() {
        let overlay = Overlay {
            pane_id: "w:p1".into(),
            format: "png",
            image_width: 10,
            image_height: 20,
            data_base64: "AAAA".into(),
            z_index: 100,
            placement: Placement {
                viewport_col: 1,
                viewport_row: 2,
                grid_cols: 3,
                grid_rows: 4,
            },
        };
        let value = serde_json::to_value(&overlay).unwrap();
        assert!(value.get("owner").is_none());
        assert_eq!(value["placement"]["grid_cols"], 3);
        assert_eq!(value["format"], "png");
    }

    #[test]
    fn open_popup_sends_the_minimal_focused_request() {
        let path = std::env::temp_dir().join(format!("hop-open-popup-test-{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).unwrap();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut line = String::new();
            let mut reader = BufReader::new(stream);
            reader.read_line(&mut line).unwrap();
            let request: Value = serde_json::from_str(&line).unwrap();
            assert_eq!(request["method"], "plugin.pane.open");
            assert_eq!(request["params"]["plugin_id"], "example.hop");
            assert_eq!(request["params"]["entrypoint"], "picker");
            assert_eq!(request["params"]["placement"], "popup");
            assert_eq!(request["params"]["width"], 6);
            assert_eq!(request["params"]["height"], 4);
            assert_eq!(request["params"]["focus"], true);
            assert_eq!(request["params"]["env"]["HOP_TARGETS_JSON"], "[]");

            let id = request["id"].as_str().unwrap();
            writeln!(reader.get_mut(), "{{\"id\":{id:?},\"result\":{{}}}}").unwrap();
        });

        let mut env = serde_json::Map::new();
        env.insert("HOP_TARGETS_JSON".into(), Value::String("[]".into()));
        HerdrClient::new(&path)
            .open_popup("example.hop", "picker", 6, 4, env)
            .unwrap();

        server.join().unwrap();
        std::fs::remove_file(path).unwrap();
    }
}
