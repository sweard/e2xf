# Rust 日志流推送到 Flutter UI

**Date:** 2026-05-14
**Status:** Approved
**Scope:** 把 Rust 端的 `println!` 调试输出统一改造为分级日志,通过 flutter_rust_bridge 的 `StreamSink` 推送到 Flutter UI 的日志框。

## 背景

`rust/src/excel_to_xml/` 核心库内多处用 `println!` 打印 Excel 解析进度、错误信息。在 Flutter 桌面 release 包中:

- 控制台被 GUI 应用隐藏,用户看不到这些输出
- 错误信息(如"未找到 res 文件夹")只能通过函数返回值的字符串拿到一条概要,中间过程的 `println!` 全部丢失
- 调试时只能改代码加日志重新构建

`excel_to_xml` 同时是一个独立的 CLI(`main.rs`),控制台输出对 CLI 模式仍有价值,不能直接删除。

## 目标

1. Rust 端使用统一的 `log` 抽象
2. Flutter 调用 Rust 函数期间产生的日志能实时显示在 UI 日志框
3. 不破坏 `excel_to_xml` 独立 crate 的属性 — 不引入对 flutter_rust_bridge 的依赖
4. CLI 模式 (`cargo run`) 仍能在控制台看到日志
5. 默认 Info 级别,可在运行时切到 Debug

## 非目标

- 不引入持久化日志 / 文件落盘
- 不为 logger 加复杂特性(target 过滤、自定义 format、多 sink 配置)
- 不改 UI(暂不加调试开关 checkbox,只暴露 Rust API)

## 架构

```
┌─────────────────────────────┐
│ excel_to_xml (核心库)       │
│   log::info!/warn!/error!   │  仅依赖 log facade,无 FRB
└──────────────┬──────────────┘
               │
        log crate facade
               │
┌──────────────▼──────────────────────────┐
│ rust_lib_e2xf (bridge crate)            │
│  StreamLogger : log::Log                │
│   ├─ println!  (stdout, CLI 兼容)       │
│   └─ sink.add() (若 sink 已注册)        │
│                                         │
│  static SINK: OnceLock<RwLock<Option    │
│              <StreamSink<String>>>>     │
└──────────────┬──────────────────────────┘
               │ FRB
┌──────────────▼──────────────┐
│ Flutter MainViewModel       │
│  api.createLogStream()      │
│   .listen((m) => _log+=m)   │
└─────────────────────────────┘
```

## 组件细节

### 1. `excel_to_xml` (核心库)

**Cargo.toml**: 添加 `log = "0.4"` 到 `[dependencies]`

**println! → log 映射规则**:

| 文件 | 原内容 | 新级别 |
|------|--------|--------|
| `read_excel.rs:59` | "开始解析Excel文件: ..." | `info!` |
| `read_excel.rs:107` | "header_cells: ..." | `debug!` |
| `read_excel.rs:104` | "工作表为空或没有数据" | `warn!` |
| `write_xml.rs:39` | "符合后缀的文件: ..." (find_files.rs) | `debug!` |
| `write_xml.rs:45` | "解析配置时出错: ..." | `error!` |
| `write_xml.rs:48` | "解析配置成功: ..." | `info!` |
| `write_xml.rs:57` | "未找到res文件夹" | `error!` |
| `write_xml.rs:60` | "找到res文件夹: ..." | `info!` |
| `write_xml.rs:206` | "tag_value_map size: ..." | `debug!` |
| `write_xml.rs:236-239` | "更新XML文件失败 ..." | `error!` |
| `write_xml.rs:264` | "正则表达式错误: ..." | `warn!` |

**main.rs**: 交互式 CLI 提示(menu / prompts / 用户输入回显)保持 `println!` — 那是 UI 输出而非日志。`update`/`quick_update` 的成功失败提示在 CLI 模式下仍由 `match` 块的 `println!` 输出,但内部库函数会通过 `log` 输出更详细的过程。

### 2. `rust_lib_e2xf` (bridge crate)

**Cargo.toml**: 添加 `log = "0.4"` 到 `[dependencies]`

**新增 `bridge.rs` 内容**:

```rust
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
        if !self.enabled(record.metadata()) { return; }
        let line = format!("[{}] {}", record.level(), record.args());
        println!("{}", line);  // CLI 兼容
        if let Some(lock) = SINK.get() {
            if let Ok(guard) = lock.read() {
                if let Some(sink) = guard.as_ref() {
                    let _ = sink.add(line);  // 失败静默(stream 已关闭)
                }
            }
        }
    }
    fn flush(&self) {}
}

static LOGGER: StreamLogger = StreamLogger;

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
    let _ = log::set_logger(&LOGGER);
    log::set_max_level(LevelFilter::Info);
}

pub fn create_log_stream(sink: StreamSink<String>) {
    let lock = SINK.get_or_init(|| RwLock::new(None));
    if let Ok(mut guard) = lock.write() {
        *guard = Some(sink);
    }
}

#[flutter_rust_bridge::frb(sync)]
pub fn set_log_debug(enable: bool) {
    log::set_max_level(if enable { LevelFilter::Debug } else { LevelFilter::Info });
}
```

**保留**: `greet`, `get_default_cfg`, `get_sheet_names`, `update`, `quick_update` 不变。

### 3. Flutter 端 (`lib/main_viewmodel.dart`)

在 `init()` 内 `_loadPreferences()` 之前订阅 stream:

```dart
void init() {
  _logSubscription = lib.createLogStream().listen((message) {
    updateLog(message);
  });
  _loadPreferences();
}
```

新字段: `StreamSubscription<String>? _logSubscription;`

`dispose()` 中取消订阅: `_logSubscription?.cancel();`

新方法: `void setLogDebug(bool enable) => lib.setLogDebug(enable: enable);`(供未来 UI 调用)

### 4. flutter_rust_bridge codegen

bridge.rs 修改后需重新生成绑定:
```
flutter_rust_bridge_codegen generate
```
会更新 `lib/src/rust/api/bridge.dart` 和 `rust/src/frb_generated.rs`。

## 数据流

1. 应用启动 → `RustLib.init()` 触发 `init_app` → 安装 logger,默认 Info
2. `MainApp.initState()` → `viewModel.init()` → 订阅 `createLogStream` → sink 注册到全局 SINK
3. 用户点击"开始转换" → `update`/`quick_update` 内部 `log::info!()` → `StreamLogger::log()` → 同时 `println!` 和 `sink.add()` → Flutter listener 收到 → `updateLog()` → ValueNotifier 通知 UI 重建日志框

## 错误处理

- `set_logger` 失败(已设置过): 忽略,主进程内不会重复初始化
- `SINK` 写锁中毒: 静默忽略,日志会降级为只写 stdout
- `sink.add` 失败(channel closed,如 stream 被取消): 静默忽略
- 应用退出 / hot restart: 旧 sink 失效,Flutter 重新订阅会覆盖全局 sink

## 测试策略

- 手动验证:
  1. `flutter run` 后 UI 日志框初始仍显示 "Application initialized." + cache 信息
  2. 选 Excel + 模块文件夹 → 点击转换 → UI 应实时出现 `[INFO] 开始解析Excel文件: ...` `[INFO] 找到res文件夹: ...` 等
  3. 错误场景(选错路径)→ `[ERROR] 未找到res文件夹` 出现在 UI
  4. CLI 模式 `cargo run -p excel_to_xml` 仍能在控制台看到日志(同样格式)
- 不新增单元测试 — logger 实现简单,改造主体是字符串字面量替换

## 风险与回退

- **风险**: `OnceLock` + `RwLock` 锁开销 — 转换过程中日志量小(预计 < 100 行/任务),可忽略
- **风险**: Flutter 端 `_log` 字符串无限累加内存增长 — 现有代码已有此问题,不在本次改造范围内
- **回退**: 如果出问题,把 `excel_to_xml` 的 `log::*` 全局替换回 `println!`,删除 bridge.rs 新增内容。`log` crate 是 facade,不强依赖。

## 文件改动清单

1. `rust/src/excel_to_xml/Cargo.toml` — 加 `log = "0.4"`
2. `rust/src/excel_to_xml/src/config.rs` — println! 替换(test 模块除外)
3. `rust/src/excel_to_xml/src/read_excel.rs` — println! 替换
4. `rust/src/excel_to_xml/src/write_xml.rs` — println! 替换
5. `rust/src/excel_to_xml/src/find_files.rs` — println! 替换
6. `rust/src/excel_to_xml/src/main.rs` — **不修改**。该文件全部是交互式 CLI 输出(menu/prompt/结果回显),本来就该是 `println!`,与日志无关。
7. `rust_builder/Cargo.toml` — 加 `log = "0.4"`
8. `rust/src/api/bridge.rs` — 新增 logger + 两个 FRB 函数
9. `lib/main_viewmodel.dart` — 订阅 stream + dispose 取消 + setLogDebug 方法
10. **再生成**: `lib/src/rust/api/bridge.dart`, `rust/src/frb_generated.rs` (由 codegen 产生)

**CLI 模式说明**: 单独 `cargo run` 时,bridge.rs 的 `init_app` 不会被调用,核心库内的 `log::*!` 调用没有 logger 接收,会被默认丢弃。`main.rs` 在 match 分支已用 `println!` 打印关键结果(更新成功/失败 + 耗时),CLI 用户能看到的信息与改造前一致。如未来需要 CLI 也看到细粒度日志,再加 `env_logger` 即可 — 不在本次范围。

## 验收标准

- [ ] Rust `cargo check` 通过
- [ ] flutter_rust_bridge codegen 无错误
- [ ] Flutter `flutter run` 启动 — UI 日志框正常显示
- [ ] 转换 Excel → UI 日志框看到 `[INFO]` 等前缀的实时日志
- [ ] 错误场景在 UI 显示 `[ERROR]` 信息
- [ ] `excel_to_xml` 仍可独立 `cargo build` 不依赖 FRB
