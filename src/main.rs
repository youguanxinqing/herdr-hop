use anyhow::Result;

fn main() -> Result<()> {
    // For `pick`, hide the cursor before anything else runs, clap included. Herdr settles the
    // host cursor into a newly focused popup after a ~20ms quiet window; whether this write beats
    // that window is the difference between an empty box and a block glyph flashing in it.
    if std::env::args().nth(1).as_deref() == Some("pick") {
        use std::io::Write;
        let mut out = std::io::stdout();
        let _ = out.write_all(b"\x1b[?25l");
        let _ = out.flush();
    }
    herdr_hop::cli::run()
}
