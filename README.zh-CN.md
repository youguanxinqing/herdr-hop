# Herdr Hop

[English](README.md) · **简体中文** · [日本語](README.ja.md)

Herdr Hop 是开源插件 [Herdr EasyMotion](https://github.com/elliotekj/herdr-easymotion) 的独立 Rust 实现。原项目由 [Elliot Jackson](https://github.com/elliotekj) 创建，本项目保留了它的窗格直达交互：触发动作后，每个可见窗格上会显示一个大号标签；按下对应标签，即可直接聚焦目标窗格。

标签按视觉阅读顺序分配——从上到下、从左到右——依次使用 `1` 到 `9`，然后是 `a` 到 `g`。只要布局不变，标签也会保持稳定，便于形成肌肉记忆。

## 终端支持

Herdr Hop 对终端的要求比 Herdr 本身更严格。Herdr 的普通文本界面可以在许多终端中运行，但本插件通过 Herdr 的实验性 [Kitty Graphics 窗格 API](https://herdr.dev/docs/socket-api/#experimental-pane-graphics) 绘制标签，不提供纯文本或 Sixel 降级方案。

| 宿主终端 | 支持情况 |
|---|---|
| [Ghostty](https://ghostty.org) | 支持，已测试。 |
| [kitty](https://sw.kovidgoyal.net/kitty/)、[WezTerm](https://wezterm.org) | 理论上支持：Herdr 的图形路径明确识别这两种 Kitty Graphics 终端，但本项目尚未测试。 |
| 其他实现 Kitty Graphics Protocol 的终端 | 可能可用，但本项目目前没有验证。 |
| iTerm2、Terminal.app、Windows Terminal，以及无法提供 Kitty Graphics 路径的终端 | 不支持。Herdr 本身可能仍能运行，但 Herdr Hop 的窗格标签不会显示。 |

图形能力必须由最外层终端提供。如果 Herdr 运行在 tmux、SSH 或其他中间层中，每一层都必须正确透传 Kitty Graphics Protocol。建议直接在已测试的 Ghostty 中运行 Herdr；kitty 和 WezTerm 理论上支持，但本项目尚未测试。

插件清单目前不支持原生 Windows。Herdr 官方文档也明确说明 Windows Terminal 无法提供其所需的 Kitty Graphics 路径。Linux 和 macOS 受支持；WSL 组合仍取决于 Windows 侧的宿主终端，本项目目前未对其进行测试。

请在 `~/.config/herdr/config.toml` 中启用 Herdr 的图形功能。如果文件中已经存在 `[experimental]` 表，请将配置合并进去，不要重复声明该表：

```toml
[experimental]
kitty_graphics = true
```

修改后重新加载正在运行的 Herdr 服务：

```bash
herdr server reload-config
```

图形功能生效后，Herdr Hop 会通过选择器自身的 PTY 测量字符单元尺寸。仅仅为了让新安装的标签正确显示，无需重新打开终端或重新附加会话。

## 环境要求

- Herdr 0.8.2 或更高版本
- Linux 或 macOS
- 上述兼容 Kitty Graphics 的终端
- Rust 和 Cargo（当前安装过程需要从源码构建插件）

## 安装

从 GitHub 安装：

```bash
herdr plugin install youguanxinqing/herdr-hop
```

如需安装指定分支、标签或提交，请传入 `--ref`。使用本地源码时，需要先构建再链接，因为 `herdr plugin link` 不会执行构建命令：

```bash
./scripts/build.sh
herdr plugin link .
```

确认 Herdr 已注册插件动作：

```bash
herdr plugin action list --plugin youguanxinqing.herdr-hop
```

## 快捷键

在 Herdr 配置中添加一个 `plugin_action` 绑定，然后执行 `herdr server reload-config`：

```toml
[[keys.command]]
key = "prefix+q"
type = "plugin_action"
command = "youguanxinqing.herdr-hop.jump"
description = "hop to a visible pane"
```

`prefix+q` 只是示例。请根据自己的配置选择不会与 Herdr 或外层终端快捷键冲突的按键。

## 使用方法

1. 打开一个至少包含两个可见窗格的 Herdr 标签页。
2. 按下配置好的快捷键。
3. 按下目标窗格上显示的标签。字母标签不区分大小写。

按 `Esc` 或 `Ctrl-C` 可以取消。其他无关按键会被忽略；如果 20 秒内没有作出选择，选择器会自动关闭。插件会先清除所有标签，再切换焦点，因此不会在原标签页留下残余图层。

插件最多为 16 个窗格分配标签：`1`–`9`，然后是 `a`–`g`。当前标签页处于缩放状态，或可见窗格少于两个时，插件会直接退出。选择期间可能会短暂出现一个空白小弹窗；它只负责接收一次按键，实际标签绘制在目标窗格上。

## 故障排查

如果选择器已经打开，但没有显示标签：

1. 确认 `[experimental].kitty_graphics = true` 已经生效，并且重新加载过服务配置。
2. 确认外层终端是已测试的 Ghostty；如使用尚未测试的 kitty、WezTerm 或其他终端，请确认它能正常提供 Kitty Graphics 路径。
3. 测试时暂时移除 tmux、SSH 或其他中间终端层。
4. 查看插件命令日志：

   ```bash
   herdr plugin log list --plugin youguanxinqing.herdr-hop
   ```

## 卸载

```bash
herdr plugin uninstall youguanxinqing.herdr-hop
```

如果本地源码是通过 `plugin link` 注册的，请执行：

```bash
herdr plugin unlink youguanxinqing.herdr-hop
```

## 开发

运行项目要求的格式、测试和静态检查：

```bash
just verify
```

构建发布版本，并将二进制文件放到 `herdr-plugin.toml` 约定的位置：

```bash
just build
```

## 项目来源与致谢

Herdr Hop 参考了 [elliotekj/herdr-easymotion](https://github.com/elliotekj/herdr-easymotion) 的思路，并使用 Rust 重新实现了它的窗格跳转工作流。感谢 [Elliot Jackson](https://github.com/elliotekj) 将原项目开源。

## 许可证

Herdr Hop 的 Rust 实现采用 [MIT 许可证](LICENSE)。作为参考的 Herdr EasyMotion 项目独立采用 Apache License 2.0。
