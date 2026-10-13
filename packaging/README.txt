e2xf (Slint + Rust)

Windows: 解压后运行 e2xf.exe。
macOS: 打开对应 arm64 / x86_64 的 DMG，将 e2xf.app 拖入 Applications。
Linux: 解压后运行 ./e2xf。需要 X11 或 Wayland 桌面、Fontconfig、libxkbcommon，以及支持文件选择的 XDG Desktop Portal（GTK/GNOME/KDE）或 Zenity。

选择 Excel 和 Android 模块文件夹，检查 JSON 配置后点击“开始转换”。
普通转换和快速转换使用相同的 Rust 核心；转换成功后显示更新的 XML 数量。
旧 Flutter 工程保留用于对照，新应用不需要 Flutter 运行时或 Flutter Rust Bridge。
