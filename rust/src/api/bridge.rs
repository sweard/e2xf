extern crate excel_to_xml;

use std::sync::{OnceLock, RwLock};
use crate::frb_generated::StreamSink;
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
