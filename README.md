# e2xf

e2xf 是一个桌面端 Excel → Android `strings.xml` 转换工具。Flutter 负责配置、文件选择和日志界面，Rust 负责读取 Excel、匹配 Android 语言资源目录并安全更新 XML 文件。

当前发布平台为 macOS、Windows 和 Linux。

## 仓库结构

项目由一个主仓库和一个 Git 子模块组成：

```text
e2xf/
├── lib/                         Flutter 界面与状态管理
├── rust/                        Flutter Rust Bridge 适配层（属于主仓库）
│   └── src/excel_to_xml/        Rust 转换核心（独立 Git 子模块）
├── rust_builder/                Cargokit 原生构建插件
├── integration_test/            Flutter 集成测试
├── test/                        Flutter 单元测试
└── .github/workflows/           构建、验证与发布流程
```

调用链：

```text
Flutter UI → MainViewModel → flutter_rust_bridge → Rust bridge → excel_to_xml → strings.xml
```

`rust/` 整体不是第二个仓库，真正的子模块路径是 `rust/src/excel_to_xml`。

## 环境要求

CI 当前固定使用：

- Flutter 3.41.4（Dart 3.11.1）
- Rust 1.98.0
- flutter_rust_bridge / codegen 2.13.0

还需要目标桌面平台对应的 Flutter 原生构建工具链。

## 获取项目

首次克隆时同时初始化 Rust 子模块：

```bash
git clone --recurse-submodules https://github.com/sweard/e2xf.git
cd e2xf
```

已有工作区可执行：

```bash
git submodule update --init --recursive
```

## 运行

```bash
flutter pub get
flutter run -d macos
```

将 `macos` 替换为 `windows` 或 `linux` 即可运行相应桌面版本。

基本使用流程：

1. 选择包含翻译内容的 `.xlsx` 文件。
2. 选择包含 Android `res` 目录的模块文件夹。
3. 检查或修改 JSON 配置。
4. 选择普通转换或快速转换。
5. 点击“开始转换”，在界面下方查看 Rust 和 Flutter 日志。

普通转换按语言读取 Excel，内存占用较低；快速转换一次读取所有语言，速度更快但占用更多内存。两种模式应产生相同结果。

## 配置说明

默认配置由 Rust 核心提供，主要字段如下：

| 字段 | 作用 |
| --- | --- |
| `sheetName` | Excel 工作表名称；为空时使用第一个工作表 |
| `tagName` | Android string key 所在列的表头 |
| `defaultLang` | 默认语言，用于其他语言空白内容回退 |
| `langMap` | Android 语言目录后缀与 Excel 表头的映射 |
| `disableEscape` | 是否禁用 XML 转义 |
| `escapeOnly` | 指定需要执行的字符替换规则 |
| `reset` | 是否完全重写目标 XML，而不是保留未出现在 Excel 中的标签 |
| `replaceBlankWithDefault` | 翻译为空时是否使用默认语言内容 |
| `regex` | 写入前用于清理文本的正则表达式 |
| `ignoreFolder` | 搜索 `res` 目录时需要忽略的路径片段 |

默认语言对应 `res/values/strings.xml`，其他语言对应 `res/values-<lang>/strings.xml`。Excel 中存在语言列但目标 XML 缺失时，转换会明确失败，不会静默跳过。

## 文件安全

转换不会边读取边覆盖原文件：

1. 先为所有语言生成同目录临时文件。
2. 所有临时文件成功后，为原文件建立备份。
3. Unix 平台使用同目录原子重命名替换；Windows 在删除目标前已完成备份。
4. 任一文件替换失败时，已更新的文件会从备份回滚。
5. 回滚不完整时保留备份，并通过错误信息给出失败文件。

因此调用方应以 Rust 返回的 `Result` 为最终状态，不要仅根据日志文本判断成功。

## 开发与验证

Flutter：

```bash
flutter analyze
flutter test test
```

Rust bridge 与核心：

```bash
cargo fmt --manifest-path rust/Cargo.toml -- --check
cargo fmt --manifest-path rust/src/excel_to_xml/Cargo.toml -- --check

cargo clippy --locked --manifest-path rust/Cargo.toml --all-targets -- -D warnings
cargo clippy --locked --manifest-path rust/src/excel_to_xml/Cargo.toml --all-targets -- -D warnings

cargo test --locked --manifest-path rust/Cargo.toml
cargo test --locked --manifest-path rust/src/excel_to_xml/Cargo.toml
```

Rust 核心测试会动态生成最小 XLSX，覆盖普通/快速模式一致性、缺少语言文件、XML 损坏、错误传播和替换失败回滚。

## 更新 Flutter Rust Bridge

修改 `rust/src/api/bridge.rs` 的公开函数后，重新生成 Rust/Dart 绑定：

```bash
flutter_rust_bridge_codegen generate
```

codegen 版本必须与 `pubspec.yaml` 中的 `flutter_rust_bridge` 版本保持一致。生成后同时提交 `rust/src/frb_generated.rs` 和 `lib/src/rust/` 下的变化。

## 子模块开发

核心逻辑修改应先在 `rust/src/excel_to_xml` 仓库中提交，再回到主仓库提交新的子模块指针：

```bash
cd rust/src/excel_to_xml
git status

cd ../../..
git status
```

克隆、CI 和发布环境都必须使用递归子模块检出，否则 Rust bridge 无法找到 `excel_to_xml`。

## 发布

推送 `v*` 标签会触发 GitHub Actions：

1. 先运行 Flutter analyze/test 和 Rust fmt/clippy/test。
2. 验证通过后并行构建 macOS、Windows、Linux。
3. 生成 DMG、ZIP、tar.gz 并创建 GitHub Release。

发布流程定义在 `.github/workflows/release.yml`。
