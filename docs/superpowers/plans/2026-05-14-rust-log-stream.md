# Rust Log Stream → Flutter UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 `excel_to_xml` 内的 `println!` 改造成 `log` crate 调用,通过 flutter_rust_bridge `StreamSink<String>` 实时推送到 Flutter UI 日志框,同时保留控制台输出供 CLI 模式使用。

**Architecture:** `excel_to_xml` 仅依赖 `log` facade。bridge crate 实现 `log::Log`,在每条记录上同时 `println!` 到 stdout 和 `sink.add()` 到 Flutter 端。`init_app` 装载 logger;`create_log_stream` FRB 函数让 Dart 端注册 sink;`set_log_debug` 切换 Info/Debug 级别。

**Tech Stack:** Rust(`log` 0.4 + flutter_rust_bridge 2.11.1) / Flutter / Dart Stream API。

**Spec:** [`docs/superpowers/specs/2026-05-14-rust-log-stream-design.md`](../specs/2026-05-14-rust-log-stream-design.md)

---

## File Structure

**Create:**
- (none — all changes go to existing files; codegen will regenerate `lib/src/rust/api/bridge.dart` and `rust/src/frb_generated.rs`)

**Modify:**
- `rust/src/excel_to_xml/Cargo.toml` — 加 `log` 依赖
- `rust/src/excel_to_xml/src/find_files.rs` — 1 处 `println!` → `log::debug!`
- `rust/src/excel_to_xml/src/read_excel.rs` — 3 处 `println!` 替换
- `rust/src/excel_to_xml/src/write_xml.rs` — 7 处 `println!` 替换
- `rust_builder/Cargo.toml` — 加 `log` 依赖
- `rust/src/api/bridge.rs` — 新增 `StreamLogger` + 两个 FRB 函数,改造 `init_app`
- `lib/main_viewmodel.dart` — 订阅 log stream,dispose 取消订阅,新增 `setLogDebug`

**Leave alone:**
- `rust/src/excel_to_xml/src/config.rs` — 内部 `println!` 仅在 `#[cfg(test)]` 测试代码里,测试通常需要 stdout 输出,不改
- `rust/src/excel_to_xml/src/main.rs` — 全部 `println!` 是 CLI 交互输出(menu/prompt/结果回显),不是日志
- `lib/main.dart` — UI 不变(暂不暴露调试开关 checkbox)

---

## Task 1: 迁移 excel_to_xml 到 log crate

**Goal:** 替换核心库所有运行时 `println!` 为 `log` 宏,保持库 FRB 无关。

**Files:**
- Modify: `rust/src/excel_to_xml/Cargo.toml`
- Modify: `rust/src/excel_to_xml/src/find_files.rs:39`
- Modify: `rust/src/excel_to_xml/src/read_excel.rs:59,104,107`
- Modify: `rust/src/excel_to_xml/src/write_xml.rs:44,48,56,60,206,236-240,264`

- [ ] **Step 1.1: 添加 log 依赖**

修改 `rust/src/excel_to_xml/Cargo.toml`,在 `[dependencies]` 区块末尾追加一行:

```toml
[package]
name = "excel_to_xml"
version = "0.1.8"
edition = "2021"

[lib]
name = "excel_to_xml"
crate-type = ["cdylib", "rlib"]

[dependencies]
# Excel读取
calamine = "0.32.0"
# JSON解析
# serde = "1.0.215"
serde_json = "1.0.140"
# xml读写
quick-xml = "0.37.3"
# 正则表达式
regex = "1.11.1"
# 日志门面
log = "0.4"
```

- [ ] **Step 1.2: 替换 find_files.rs**

修改 `rust/src/excel_to_xml/src/find_files.rs` 第 39 行附近:

旧:
```rust
                .find_map(|file| {
                    let file_path = file.path();
                    if file_path.is_file() && file_path.to_str()?.ends_with(target) {
                        println!("符合后缀的文件: {}", file_path.display());
                        Some(file_path.to_str()?.to_string())
```

新:
```rust
                .find_map(|file| {
                    let file_path = file.path();
                    if file_path.is_file() && file_path.to_str()?.ends_with(target) {
                        log::debug!("符合后缀的文件: {}", file_path.display());
                        Some(file_path.to_str()?.to_string())
```

- [ ] **Step 1.3: 替换 read_excel.rs**

修改 `rust/src/excel_to_xml/src/read_excel.rs` 三处:

第 59 行:
```rust
    println!("开始解析Excel文件: {} config_json: {}", file_path, config_json);
```
改为:
```rust
    log::info!("开始解析Excel文件: {} config_json: {}", file_path, config_json);
```

第 103-105 行:
```rust
    if first_row.is_empty() {
        println!("工作表为空或没有数据");
        return Err(Box::new(ExcelError::InvalidFirstLine));
    }
```
改为:
```rust
    if first_row.is_empty() {
        log::warn!("工作表为空或没有数据");
        return Err(Box::new(ExcelError::InvalidFirstLine));
    }
```

第 107 行:
```rust
    println!("header_cells: {:?}\n", first_row);
```
改为:
```rust
    log::debug!("header_cells: {:?}", first_row);
```

(注:去掉末尾 `\n` — 日志框架自带换行)

- [ ] **Step 1.4: 替换 write_xml.rs**

修改 `rust/src/excel_to_xml/src/write_xml.rs` 七处:

第 43-46 行:
```rust
    if cfg.is_err() {
        println!("解析配置时出错: {:?}", cfg.err());
        return Err("解析配置时出错".into());
    }
```
改为:
```rust
    if cfg.is_err() {
        log::error!("解析配置时出错: {:?}", cfg.err());
        return Err("解析配置时出错".into());
    }
```

第 48 行:
```rust
    println!("解析配置成功: {:?}", parsed_cfg);
```
改为:
```rust
    log::info!("解析配置成功: {:?}", parsed_cfg);
```

第 55-58 行:
```rust
    if forder.is_none() {
        println!("未找到res文件夹");
        return Err("未找到res文件夹".into());
    }
```
改为:
```rust
    if forder.is_none() {
        log::error!("未找到res文件夹");
        return Err("未找到res文件夹".into());
    }
```

第 60 行:
```rust
    println!("找到res文件夹: {}", res_folder);
```
改为:
```rust
    log::info!("找到res文件夹: {}", res_folder);
```

第 206 行:
```rust
    println!("tag_value_map size: {}", tag_value_map.len());
```
改为:
```rust
    log::debug!("tag_value_map size: {}", tag_value_map.len());
```

第 235-241 行:
```rust
        if res.is_err() {
            println!(
                "更新XML文件失败,lang index: {}, err: {:?}",
                index,
                res.err()
            );
        }
```
改为:
```rust
        if res.is_err() {
            log::error!(
                "更新XML文件失败,lang index: {}, err: {:?}",
                index,
                res.err()
            );
        }
```

第 263-266 行:
```rust
            Err(e) => {
                println!("正则表达式错误: {:?}", e);
                None
            }
```
改为:
```rust
            Err(e) => {
                log::warn!("正则表达式错误: {:?}", e);
                None
            }
```

- [ ] **Step 1.5: 验证核心库独立编译**

```bash
cd /Users/sbwoan/FlutterProjects/e2xf/rust/src/excel_to_xml
cargo check
```

Expected: `Finished \`dev\` profile ... target(s) in ...s` 无错误无警告。

- [ ] **Step 1.6: 验证残留 println!**

```bash
cd /Users/sbwoan/FlutterProjects/e2xf
grep -rn "println!" rust/src/excel_to_xml/src/ | grep -v "main.rs" | grep -v "test"
```

Expected: 输出为空(全部已迁移,test 模块和 main.rs 不计)。

- [ ] **Step 1.7: Commit**

```bash
cd /Users/sbwoan/FlutterProjects/e2xf
git add rust/src/excel_to_xml/Cargo.toml rust/src/excel_to_xml/src/find_files.rs rust/src/excel_to_xml/src/read_excel.rs rust/src/excel_to_xml/src/write_xml.rs
git commit -m "$(cat <<'EOF'
refactor(excel_to_xml): migrate println! to log crate facade

Why: enable downstream consumers (Flutter UI via FRB stream, env_logger
in CLI) to capture and route logs without modifying the library.
Library stays FRB-agnostic — only adds the log facade dependency.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: 实现 bridge 端 StreamLogger + FRB 函数

**Goal:** bridge crate 提供 `log::Log` 实现,日志同时进 stdout 和 Flutter sink;暴露 `create_log_stream` 和 `set_log_debug`。

**Files:**
- Modify: `rust_builder/Cargo.toml`
- Modify: `rust/src/api/bridge.rs`

- [ ] **Step 2.1: 添加 log 依赖到 bridge crate**

修改 `rust_builder/Cargo.toml`,在 `[dependencies]` 末尾加一行:

```toml
[package]
name = "rust_lib_e2xf"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "staticlib"]

[dependencies]
flutter_rust_bridge = "=2.11.1"
excel_to_xml = {path = "../rust/src/excel_to_xml"}
log = "0.4"

[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(frb_expand)'] }
```

- [ ] **Step 2.2: 重写 bridge.rs**

完整覆盖 `rust/src/api/bridge.rs`:

```rust
extern crate excel_to_xml;

use std::sync::{OnceLock, RwLock};
use flutter_rust_bridge::StreamSink;
use log::{Log, Metadata, Record, LevelFilter};

static SINK: OnceLock<RwLock<Option<StreamSink<String>>>> = OnceLock::new();

struct StreamLogger;

impl Log for StreamLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = format!("[{}] {}", record.level(), record.args());
        // 1) stdout 兼容 CLI / 调试场景
        println!("{}", line);
        // 2) 推到 Flutter (若 sink 已注册)
        if let Some(lock) = SINK.get() {
            if let Ok(guard) = lock.read() {
                if let Some(sink) = guard.as_ref() {
                    let _ = sink.add(line);
                }
            }
        }
    }

    fn flush(&self) {}
}

static LOGGER: StreamLogger = StreamLogger;

#[flutter_rust_bridge::frb(sync)] // Synchronous mode for simplicity of the demo
pub fn greet(name: String) -> String {
    format!("Hello Rust, {name}!")
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    // Default utilities - feel free to customize
    flutter_rust_bridge::setup_default_user_utils();
    let _ = log::set_logger(&LOGGER);
    log::set_max_level(LevelFilter::Info);
}

/// 注册 Flutter 端的 log stream sink。每次调用会覆盖上次的 sink。
pub fn create_log_stream(sink: StreamSink<String>) {
    let lock = SINK.get_or_init(|| RwLock::new(None));
    if let Ok(mut guard) = lock.write() {
        *guard = Some(sink);
    }
}

/// 切换日志级别:开 = Debug,关 = Info。
#[flutter_rust_bridge::frb(sync)]
pub fn set_log_debug(enable: bool) {
    log::set_max_level(if enable {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    });
}

#[flutter_rust_bridge::frb(sync)]
pub fn get_default_cfg() -> String {
    excel_to_xml::get_default_cfg_json()
}

#[flutter_rust_bridge::frb(sync)]
pub fn get_sheet_names(file_path: &str) -> Vec<String> {
    excel_to_xml::get_sheet_names(file_path)
}

pub fn update(cfg_json: &str, excel_path: &str, xml_dir_path: &str) -> String {
    excel_to_xml::update(cfg_json, excel_path, xml_dir_path)
}

pub fn quick_update(cfg_json: &str, excel_path: &str, xml_dir_path: &str) -> String {
    excel_to_xml::quick_update(cfg_json, excel_path, xml_dir_path)
}
```

- [ ] **Step 2.3: 验证 bridge crate 编译**

```bash
cd /Users/sbwoan/FlutterProjects/e2xf/rust_builder
cargo check
```

Expected: `Finished` 无错误。可能有"未使用 frb_generated.rs 中尚未存在的符号"类警告,这是预期的 — codegen 还没跑。

- [ ] **Step 2.4: Commit**

```bash
cd /Users/sbwoan/FlutterProjects/e2xf
git add rust_builder/Cargo.toml rust/src/api/bridge.rs
git commit -m "$(cat <<'EOF'
feat(bridge): add StreamLogger forwarding logs to Flutter via FRB

Implements log::Log to dual-write each record: println! for CLI/console
and StreamSink<String>::add for the Flutter UI subscriber. Adds
create_log_stream(sink) for registration and set_log_debug(bool) for
runtime level control. init_app installs the logger at LevelFilter::Info.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: 重新生成 flutter_rust_bridge 绑定

**Goal:** 让 Dart 端能调用新增的 `createLogStream` 和 `setLogDebug`。

**Files:**
- Regenerate: `lib/src/rust/api/bridge.dart`
- Regenerate: `lib/src/rust/frb_generated.dart`
- Regenerate: `rust/src/frb_generated.rs`
- Possibly regenerate: `lib/src/rust/frb_generated.io.dart`, `lib/src/rust/frb_generated.web.dart` (依工具版本)

- [ ] **Step 3.1: 跑 codegen**

```bash
cd /Users/sbwoan/FlutterProjects/e2xf
flutter_rust_bridge_codegen generate
```

Expected: 看到 "Done!" 或类似成功信息。无 ERROR 输出。

如果命令找不到,先安装:`cargo install flutter_rust_bridge_codegen --version 2.11.1`(版本对齐 `pubspec.yaml`)。

- [ ] **Step 3.2: 验证 Dart 绑定包含新函数**

```bash
cd /Users/sbwoan/FlutterProjects/e2xf
grep -E "createLogStream|setLogDebug" lib/src/rust/api/bridge.dart
```

Expected: 至少匹配两行,看到 `Stream<String> createLogStream()` 和 `void setLogDebug({required bool enable})` 之类签名。

- [ ] **Step 3.3: 验证 flutter analyze 通过**

```bash
cd /Users/sbwoan/FlutterProjects/e2xf
flutter analyze lib/
```

Expected: `No issues found!`。

- [ ] **Step 3.4: Commit 生成产物**

```bash
cd /Users/sbwoan/FlutterProjects/e2xf
git add lib/src/rust/ rust/src/frb_generated.rs
git commit -m "$(cat <<'EOF'
chore(codegen): regenerate FRB bindings for log stream API

Adds createLogStream() and setLogDebug() to the Dart bridge.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Flutter 端订阅 log stream

**Goal:** `MainViewModel` 在初始化时订阅 Rust 日志流,把每条消息追加到 UI 日志框;dispose 时取消订阅;暴露 `setLogDebug` 方法。

**Files:**
- Modify: `lib/main_viewmodel.dart:1-12,63-66,289-308`

- [ ] **Step 4.1: 添加 StreamSubscription 字段**

修改 `lib/main_viewmodel.dart` 顶部 import 区(第 1-10 行已有 `dart:async`,无需新增 import)。

在 `MainViewModel` 类内 `_resolvedXmlFolder` 字段下方追加 `_logSubscription` 字段。定位到第 19-23 行附近:

旧:
```dart
  final _secureBookmarks = SecureBookmarks();

  // 存储已解析的 security-scoped 资源,以便 dispose 时释放
  FileSystemEntity? _resolvedExcelFile;
  FileSystemEntity? _resolvedXmlFolder;
```

新:
```dart
  final _secureBookmarks = SecureBookmarks();

  // 存储已解析的 security-scoped 资源,以便 dispose 时释放
  FileSystemEntity? _resolvedExcelFile;
  FileSystemEntity? _resolvedXmlFolder;

  // Rust 日志流订阅
  StreamSubscription<String>? _logSubscription;
```

- [ ] **Step 4.2: 在 init() 订阅 stream**

修改 `init()` 方法(第 63-65 行):

旧:
```dart
  void init() {
    _loadPreferences();
  }
```

新:
```dart
  void init() {
    _logSubscription = lib.createLogStream().listen((message) {
      updateLog(message);
    });
    _loadPreferences();
  }
```

- [ ] **Step 4.3: 添加 setLogDebug 方法**

在 `update()` 方法之后、`updateLog()` 之前(第 279 行附近)插入新方法:

旧片段(`update()` 结束处):
```dart
    } finally {
      _isLoading.value = false;
    }
  }

  void updateLog(String message) {
```

新片段:
```dart
    } finally {
      _isLoading.value = false;
    }
  }

  /// 切换 Rust 端日志级别: true = Debug,false = Info(默认)。
  void setLogDebug(bool enable) {
    lib.setLogDebug(enable: enable);
  }

  void updateLog(String message) {
```

- [ ] **Step 4.4: dispose 取消订阅**

修改 `dispose()` 方法(第 289-308 行)。在 `_debounceTimer?.cancel();` 之后追加 `_logSubscription?.cancel();`:

旧:
```dart
  void dispose() async {
    _debounceTimer?.cancel(); // 清理防抖定时器

    // 释放 macOS security-scoped 资源
```

新:
```dart
  void dispose() async {
    _debounceTimer?.cancel(); // 清理防抖定时器
    await _logSubscription?.cancel(); // 取消日志流订阅

    // 释放 macOS security-scoped 资源
```

- [ ] **Step 4.5: 验证 flutter analyze**

```bash
cd /Users/sbwoan/FlutterProjects/e2xf
flutter analyze lib/main_viewmodel.dart
```

Expected: `No issues found!`。

- [ ] **Step 4.6: Commit**

```bash
cd /Users/sbwoan/FlutterProjects/e2xf
git add lib/main_viewmodel.dart
git commit -m "$(cat <<'EOF'
feat(ui): subscribe to Rust log stream and surface in UI log box

MainViewModel now opens a StreamSubscription against createLogStream()
in init() and forwards each record to updateLog(). dispose() cancels
the subscription. Adds setLogDebug() to flip the Rust-side level
filter at runtime.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: 端到端冒烟测试

**Goal:** 启动应用,确认日志真的从 Rust 流到 UI;失败时回滚或修复。

**Files:** 无修改(纯验证)。

- [ ] **Step 5.1: 启动 Flutter desktop**

```bash
cd /Users/sbwoan/FlutterProjects/e2xf
flutter run -d macos
```

(Linux 用户用 `-d linux`,Windows 用 `-d windows`。)

Expected: 应用窗口出现,UI 日志框显示初始 cache 信息(原有 `Application initialized.` + cache 内容)。**注意**: 这一步如果是首次构建会很慢(Rust 全量编译),允许 1-3 分钟。

- [ ] **Step 5.2: 触发一次成功转换路径**

操作:
1. 配置文本框已有默认 JSON
2. 点"选择 Excel 文件" → 选一个有效 xlsx
3. 点"选择模块文件夹" → 选一个含 `res/values*/strings.xml` 的 Android 模块根目录
4. 点"开始转换"

Expected: UI 日志框出现至少以下几条(顺序可能变):
- `[INFO] 开始解析Excel文件: /...`
- `[INFO] 解析配置成功: ParsedCfg { ... }`
- `[INFO] 找到res文件夹: /...`
- 最终原有的 `update success` / `quick update success`

控制台同时也应有完全相同的 `[INFO] ...` 行。

- [ ] **Step 5.3: 触发一次错误路径**

操作: 选一个**不**含 `res/` 的文件夹再点转换。

Expected: UI 日志框出现 `[ERROR] 未找到res文件夹`,不再像改造前那样只有函数返回值的 `更新失败:...` 一条。

- [ ] **Step 5.4: 验证 excel_to_xml 仍可独立构建**

```bash
cd /Users/sbwoan/FlutterProjects/e2xf/rust/src/excel_to_xml
cargo build --release
```

Expected: 构建成功,无 `flutter_rust_bridge` 依赖出现在 `Cargo.lock` 的 `excel_to_xml` 相关链路上。

- [ ] **Step 5.5: 标记验收完成**

更新 spec 文件 `docs/superpowers/specs/2026-05-14-rust-log-stream-design.md` 末尾的"验收标准",把六条 `[ ]` 全改 `[x]`(只在前述步骤都通过时)。

Commit:
```bash
cd /Users/sbwoan/FlutterProjects/e2xf
git add docs/superpowers/specs/2026-05-14-rust-log-stream-design.md
git commit -m "$(cat <<'EOF'
docs: mark log-stream design acceptance criteria as complete

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## 失败回退策略

任何 Task 验证失败时:

| 失败位置 | 回退命令 |
|----------|----------|
| Task 1 cargo check 报错 | `git restore rust/src/excel_to_xml/` 重做 Step 1.x |
| Task 2 bridge.rs 编译错 | `git restore rust_builder/Cargo.toml rust/src/api/bridge.rs` 重做 Step 2.x |
| Task 3 codegen 失败 | 检查 `flutter_rust_bridge_codegen --version` 与 pubspec 一致;`git restore lib/src/rust rust/src/frb_generated.rs` 后修因再跑 |
| Task 4 analyze 报错 | `git restore lib/main_viewmodel.dart` 重做 Step 4.x |
| Task 5 UI 没看到日志 | 检查 `init_app` 是否被调用(打断点或加临时 println);确认 `createLogStream` 在 `init()` 同步调用而非异步 await 之后 |

---

## 自检结果

- **Spec 覆盖**: spec 的 6 项验收标准全部映射到 Task 5 的 Steps;架构图中 4 层(excel_to_xml / log facade / bridge / Flutter)分别对应 Task 1 / 隐式 / Task 2 / Task 4。✓
- **占位符扫描**: 全文无 TBD/TODO,代码块均为完整可粘贴。✓
- **类型/签名一致性**: `create_log_stream(sink: StreamSink<String>)` → Dart `Stream<String> createLogStream()`,`set_log_debug(enable: bool)` → `setLogDebug({required bool enable})`(`#[frb(sync)]` 决定是否 Future)。Task 4 调用 `lib.createLogStream()` / `lib.setLogDebug(enable: enable)` 与之匹配。✓
- **Scope**: 单一目标(日志通路),5 个任务线性依赖,合并到一个 plan 合理。✓
