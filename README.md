# e2xf

Excel → Android `strings.xml` 桌面转换工具，支持 macOS、Windows 和 Linux。

`slint` 分支将界面迁移为 **Slint 1.18.1 + Rust**。Slint 编译为原生界面，通过 Rust 直接调用 `excel_to_xml` 核心，不需要 Flutter 引擎、Flutter Rust Bridge、Dart codegen 或 Cargokit。

## 运行与构建

```bash
git clone --recurse-submodules https://github.com/sweard/e2xf.git
cd e2xf
git switch slint
cargo run --locked
```

需要 Rust 1.98.0。Linux 需要 X11 或 Wayland、Fontconfig、libxkbcommon，文件选择需要 GTK/GNOME/KDE 的 XDG Desktop Portal 或 Zenity。Ubuntu 开发依赖：

```bash
sudo apt install pkg-config libfontconfig1-dev libxkbcommon-dev libxkbcommon-x11-dev libx11-dev libx11-xcb-dev libxcursor-dev libxi-dev libdbus-1-dev
```

构建发布二进制：

```bash
cargo build --locked --release --bin e2xf
```

macOS App 打包示例：

```bash
MACOSX_DEPLOYMENT_TARGET=12.0 cargo build --locked --release --bin e2xf --target aarch64-apple-darwin
bash scripts/package-macos.sh aarch64-apple-darwin
```

输出为 `dist/macos/arm64/e2xf.app`；Intel 使用 `x86_64-apple-darwin`。发布工作流生成两个 DMG，以及 Windows ZIP 和 Linux tar.gz。

## 使用

1. 选择 `.xlsx` 文件；当前转换核心不支持旧版 `.xls` 格式。
2. 选择 Android 主资源目录（例如模块的 `src/main/res`），或只包含一个目标 `res` 的上层目录。多 source set 模块应直接选择 `src/main/res`，避免误选其他资源目录。
3. 编辑 JSON 配置；空 `sheetName` 自动使用第一个工作表。
4. 选择普通转换或快速转换，点击“开始转换”。

普通转换逐语言读取，快速转换一次读取全部语言。转换在后台线程执行，日志保留最新 1000 行，成功后显示更新的 XML 文件数。错误以 Rust 返回值为准。转换过程中禁止修改输入和关闭窗口，避免中断文件替换。

配置编辑后延迟 500 毫秒保存，退出前再次保存。配置文件位于系统用户配置目录下的 `e2xf/settings.json`，通过同目录临时文件替换。首次启动尝试读取旧 Flutter SharedPreferences（包括 macOS 原应用容器）；保存的配置和路径会迁移，旧数据保留。无法访问缓存路径时重新选择文件即可。可通过 `e2xf --settings PATH` 使用独立配置文件。

macOS 新应用作为普通桌面程序发布，不启用 App Sandbox，文件访问不再依赖 Flutter 的 security-scoped bookmark。当前发布包使用 ad-hoc 签名，未配置 Developer ID 签名和公证，不能直接用于 Mac App Store。

## 架构

```text
Slint UI → Rust controller → excel_to_xml → strings.xml

ui/main.slint                声明式界面
src/main.rs                  文件选择、后台线程、日志与 UI 回调
src/settings.rs              原子配置保存与旧偏好导入
src/state.rs                 配置校验、工作表选择、转换输入快照
rust/src/excel_to_xml/       独立 Git 子模块，包含 XML 实体文本重复修复
packaging/ + scripts/        桌面发行包
.github/workflows/          Rust 检查、界面截图、三平台构建与发布
```

旧 Flutter 代码（`lib/`、`rust/` 下的 FRB 适配层、`rust_builder/` 和平台 runner）暂留用于比较。它们不参与新的 Cargo 构建或发行包。

## 配置

默认 JSON 由 Rust 核心提供：

| 字段 | 作用 |
| --- | --- |
| `sheetName` | 工作表；为空时选第一个 |
| `tagName` | Android string key 所在列的表头 |
| `defaultLang` | 默认语言和空翻译回退语言 |
| `langMap` | Android 语言目录后缀与 Excel 表头映射 |
| `disableEscape` / `escapeOnly` | XML 字符转义规则 |
| `reset` | 是否完全重写目标 XML |
| `replaceBlankWithDefault` | 空翻译是否回退默认语言 |
| `regex` | 写入前清理文本的正则 |
| `ignoreFolder` | 搜索 `res` 时忽略的路径片段 |

文件更新继续使用原核心的临时文件、备份、原子替换与失败回滚机制。

## 验证

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --features preview -- -D warnings
cargo test --locked --features preview
cargo test --locked --manifest-path rust/src/excel_to_xml/Cargo.toml
```

无需显示器的 UI 预览：

```bash
cargo run --locked --features preview --bin e2xf-preview -- /tmp/e2xf.png idle
# 状态：idle / busy / error / success / about；末尾可指定 660x660 等尺寸
```

实际 UI 回调另有无显示器集成测试，覆盖工作表读取、两种转换模式、转换失败后的状态恢复、配置错误及日志清空。Slint 可选 MCP 调试模式暂未启用：本机 Cargo 索引缓存尚未提供其所需的 `pbjson` / `pbjson-build` 0.9 和 `prost` / `prost-build` / `prost-types` 0.14；这些版本已在 crates.io 发布，需要刷新本机索引后再验证，不影响默认应用构建或运行。

## Slint

界面的“关于”入口包含 `AboutSlint` 控件，提供 Slint attribution。Slint 可按 [Royalty-free Desktop, Mobile, and Web Applications License 2.0](https://github.com/slint-ui/slint/blob/v1.18.1/LICENSES/LicenseRef-Slint-Royalty-free-2.0.md) 用于本桌面应用；本次迁移不更改项目自身许可证。
