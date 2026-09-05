# Herdr Hop

**English** · [简体中文](README.zh-CN.md) · [日本語](README.ja.md)

Herdr Hop is an independent Rust reimplementation of [Herdr EasyMotion](https://github.com/elliotekj/herdr-easymotion), the open-source plugin created by [Elliot Jackson](https://github.com/elliotekj). It preserves the original plugin's direct pane-jump workflow: invoke the action, read the large label drawn over each visible pane, then press that label to focus the pane.

![Herdr Hop labeling two visible panes for direct keyboard selection](assets/herdr-hop-demo.png)

Labels follow visual reading order—top to bottom, then left to right—and use `1` through `9`, followed by `a` through `g`. A stable layout therefore keeps stable, learnable labels.

## Terminal support

Herdr Hop has narrower terminal requirements than Herdr itself. Herdr's normal text UI works in many terminals, but this plugin draws its labels with Herdr's experimental [Kitty graphics pane API](https://herdr.dev/docs/socket-api/#experimental-pane-graphics). It has no text-only or Sixel fallback.

| Host terminal | Support |
|---|---|
| [Ghostty](https://ghostty.org) | Supported and tested. |
| [kitty](https://sw.kovidgoyal.net/kitty/), [WezTerm](https://wezterm.org) | Theoretically supported: Herdr's graphics path explicitly recognizes these Kitty-graphics terminals, but this project has not tested them. |
| Other terminals implementing the Kitty graphics protocol | May work, but are not currently validated by this project. |
| iTerm2, Terminal.app, Windows Terminal, and terminals without a usable Kitty graphics path | Not supported. Herdr itself may still work, but Herdr Hop's pane labels will not render. |

The outer terminal must provide the graphics capability. When running through another multiplexer or transport, such as tmux or SSH, every layer must preserve the Kitty graphics protocol. A direct Herdr session in the tested Ghostty terminal is recommended; kitty and WezTerm are theoretically supported but have not been tested by this project.

Native Windows is not currently supported by the plugin manifest. Herdr also documents that Windows Terminal does not expose the Kitty graphics path it needs. Linux and macOS are supported; WSL combinations remain dependent on the Windows host terminal and are not currently tested here.

Enable the required Herdr feature in `~/.config/herdr/config.toml` (merge this into an existing `[experimental]` table if you already have one):

```toml
[experimental]
kitty_graphics = true
```

Reload the running server after editing the file:

```bash
herdr server reload-config
```

Once graphics are enabled, Herdr Hop measures cell geometry through its own picker PTY. It does not require opening a new terminal or reattaching merely to make newly installed labels render.

## Requirements

- Herdr 0.8.2 or newer
- Linux or macOS
- A compatible Kitty-graphics terminal, as described above
- Rust and Cargo (installation currently builds the plugin from source)

## Install

Install from GitHub:

```bash
herdr plugin install youguanxinqing/herdr-hop
```

To install a specific branch, tag, or commit, pass `--ref`. To use a local checkout, build it before linking because `herdr plugin link` does not run build commands:

```bash
./scripts/build.sh
herdr plugin link .
```

Confirm that Herdr registered the action:

```bash
herdr plugin action list --plugin youguanxinqing.herdr-hop
```

## Keybinding

Add a `plugin_action` binding to your Herdr config, then reload it with `herdr server reload-config`:

```toml
[[keys.command]]
key = "prefix+q"
type = "plugin_action"
command = "youguanxinqing.herdr-hop.jump"
description = "hop to a visible pane"
```

`prefix+q` is only an example. Choose any binding that does not conflict with your existing Herdr or outer-terminal shortcuts.

## Usage

1. Open a Herdr tab containing at least two visible panes.
2. Press the configured keybinding.
3. Press the label shown over the pane you want to focus. Letter labels are case-insensitive.

Press `Esc` or `Ctrl-C` to cancel. Any unrelated key is ignored, and the picker closes automatically after 20 seconds without a selection. Labels are removed before focus changes, so they are not left behind on the previous tab.

The plugin labels up to 16 panes: `1`–`9`, then `a`–`g`. It intentionally does nothing when the current tab is zoomed or has fewer than two panes. A small blank popup may briefly appear while choosing; it exists only to capture the keystroke, while the actual labels are drawn over the target panes.

## Troubleshooting

If the picker opens but no labels appear:

1. Confirm that `[experimental].kitty_graphics = true` is active and that the server config was reloaded.
2. Confirm that the outer terminal is the tested Ghostty terminal. If you use untested kitty, WezTerm, or another terminal, confirm that it provides a working Kitty graphics path.
3. Remove tmux, SSH, or other intermediate terminal layers while testing.
4. Check the plugin command log:

   ```bash
   herdr plugin log list --plugin youguanxinqing.herdr-hop
   ```

## Uninstall

```bash
herdr plugin uninstall youguanxinqing.herdr-hop
```

For a local checkout registered with `plugin link`, use:

```bash
herdr plugin unlink youguanxinqing.herdr-hop
```

## Development

Run the required formatting, test, and lint checks:

```bash
just verify
```

Build and stage the release binary expected by `herdr-plugin.toml`:

```bash
just build
```

## Lineage and credits

Herdr Hop was inspired by [elliotekj/herdr-easymotion](https://github.com/elliotekj/herdr-easymotion) and reimplements its pane-jump workflow in Rust. Thanks to [Elliot Jackson](https://github.com/elliotekj) for publishing the original project as open source.

## License

Herdr Hop's Rust implementation is released under the [MIT License](LICENSE). The referenced Herdr EasyMotion project is released separately under Apache License 2.0.
