#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        mpsc::{self, Receiver, SyncSender},
        OnceLock,
    },
    time::Duration,
};

use e2xf::{
    settings::{Settings, SettingsStore},
    state::{apply_sheet_names, validate_config, ConversionRequest, LogBuffer},
    MainWindow,
};
use log::{LevelFilter, Log, Metadata, Record};
use slint::{ComponentHandle, Timer, TimerMode};

enum Event {
    Log(String),
    Sheets(Result<Vec<String>, String>),
    Converted(Result<excel_to_xml::ConversionReport, String>),
}

static EVENTS: OnceLock<SyncSender<Event>> = OnceLock::new();
static LOGGER: UiLogger = UiLogger;
struct UiLogger;

impl Log for UiLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }
    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let line = format!("[{}] {}", record.level(), record.args());
            eprintln!("{line}");
            if let Some(events) = EVENTS.get() {
                let _ = events.try_send(Event::Log(line));
            }
        }
    }
    fn flush(&self) {}
}

struct Controller {
    settings: Settings,
    store: SettingsStore,
    sheets: Vec<String>,
    logs: LogBuffer,
    events: SyncSender<Event>,
    allow_save: bool,
}

impl Controller {
    fn save(&mut self) {
        if !self.allow_save {
            return;
        }
        if let Err(error) = self.store.save(&self.settings) {
            self.logs.append(&error);
        }
    }

    fn sync(&self, ui: &MainWindow) {
        ui.set_config_text(self.settings.config.clone().into());
        ui.set_excel_path(self.settings.excel_path.clone().into());
        ui.set_folder_path(self.settings.xml_folder_path.clone().into());
        ui.set_quick_mode(self.settings.quick_mode);
        ui.set_log_text(self.logs.text().into());
        self.validate(ui);
    }

    fn validate(&self, ui: &MainWindow) {
        ui.set_config_error(
            validate_config(&self.settings.config, &self.sheets)
                .err()
                .unwrap_or_default()
                .into(),
        );
    }

    fn read_sheets(&mut self, ui: &MainWindow) {
        self.sheets.clear();
        ui.set_busy(true);
        ui.set_status_error(false);
        ui.set_status_text("正在读取 Excel 工作表…".into());
        let path = self.settings.excel_path.clone();
        let events = self.events.clone();
        std::thread::spawn(move || {
            let result = worker(|| excel_to_xml::get_sheet_names(&path).map_err(|e| e.to_string()));
            let _ = events.send(Event::Sheets(result));
        });
    }

    fn drain(&mut self, ui: &MainWindow, receiver: &Receiver<Event>) {
        let mut changed = false;
        // Bound work per tick so a verbose conversion cannot starve input/rendering.
        for event in receiver.try_iter().take(256) {
            changed = true;
            match event {
                Event::Log(line) => self.logs.append(&line),
                Event::Sheets(result) => {
                    ui.set_busy(false);
                    let result = result.and_then(|sheets| {
                        self.sheets = sheets;
                        apply_sheet_names(&mut self.settings, &self.sheets)
                    });
                    match result {
                        Ok(()) => {
                            self.logs
                                .append(&format!("Excel 工作表: {}", self.sheets.join(", ")));
                            self.save();
                            ui.set_status_error(false);
                            ui.set_status_text("Excel 已就绪".into());
                        }
                        Err(error) => {
                            self.logs.append(&format!("读取 Excel 失败: {error}"));
                            ui.set_status_error(true);
                            ui.set_status_text(error.clone().into());
                        }
                    }
                    self.sync(ui);
                }
                Event::Converted(result) => {
                    ui.set_busy(false);
                    let (message, failed) = match result {
                        Ok(report) => (
                            format!("转换成功，已更新 {} 个 XML 文件", report.files_updated),
                            false,
                        ),
                        Err(error) => (format!("转换失败: {error}"), true),
                    };
                    self.logs.append(&message);
                    ui.set_status_error(failed);
                    ui.set_status_text(message.into());
                }
            }
        }
        if changed {
            ui.set_log_text(self.logs.text().into());
        }
    }
}

fn worker<T>(
    action: impl FnOnce() -> Result<T, String> + std::panic::UnwindSafe,
) -> Result<T, String> {
    std::panic::catch_unwind(action)
        .unwrap_or_else(|_| Err("后台任务异常退出，请查看日志并重新尝试".into()))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (sender, receiver) = mpsc::sync_channel(4096);
    let _ = EVENTS.set(sender.clone());
    log::set_logger(&LOGGER).map_err(|e| e.to_string())?;
    log::set_max_level(LevelFilter::Info);
    let mut args = std::env::args_os().skip(1);
    let (path, import_legacy) = match args.next() {
        Some(flag) if flag == "--settings" => (
            args.next()
                .map(std::path::PathBuf::from)
                .ok_or("--settings requires a path")?,
            false,
        ),
        Some(_) => return Err("Usage: e2xf [--settings PATH]".into()),
        None => (SettingsStore::default_path()?, true),
    };
    if args.next().is_some() {
        return Err("Usage: e2xf [--settings PATH]".into());
    }
    let store = SettingsStore::new(path);
    let mut logs = LogBuffer::default();
    logs.append("e2xf 已启动（Slint + Rust）");
    let (mut settings, allow_save) = match store.load() {
        Ok(settings) => (settings, true),
        Err(error) => {
            logs.append(&error);
            logs.append("自动保存已暂停；编辑配置后恢复保存，原配置文件暂时保留");
            (Settings::default(), false)
        }
    };
    if import_legacy {
        match store.import_flutter() {
            Ok(Some(imported)) => {
                logs.append("已读取旧 Flutter 配置和文件路径");
                settings = imported;
            }
            Ok(None) => {}
            Err(error) => {
                logs.append(&format!("旧配置读取失败: {error}"));
            }
        }
    }
    let controller = Rc::new(RefCell::new(Controller {
        settings,
        store,
        sheets: vec![],
        logs,
        events: sender,
        allow_save,
    }));
    let ui = MainWindow::new()?;
    controller.borrow().sync(&ui);

    let (save_timer, _poll_timer) = wire_ui(&ui, controller.clone(), receiver);
    if !controller.borrow().settings.excel_path.is_empty() {
        controller.borrow_mut().read_sheets(&ui);
    }
    ui.run()?;
    save_timer.stop();
    controller.borrow_mut().save();
    Ok(())
}

fn wire_ui(
    ui: &MainWindow,
    controller: Rc<RefCell<Controller>>,
    receiver: Receiver<Event>,
) -> (Rc<Timer>, Timer) {
    let save_timer = Rc::new(Timer::default());
    let state = controller.clone();
    let weak = ui.as_weak();
    let timer = save_timer.clone();
    ui.on_config_edited(move |text| {
        {
            let mut state = state.borrow_mut();
            state.settings.config = text.to_string();
            state.allow_save = true;
        }
        if let Some(ui) = weak.upgrade() {
            state.borrow().validate(&ui);
        }
        let state = state.clone();
        let weak = weak.clone();
        timer.start(
            TimerMode::SingleShot,
            Duration::from_millis(500),
            move || {
                let mut state = state.borrow_mut();
                state.save();
                if let Some(ui) = weak.upgrade() {
                    ui.set_log_text(state.logs.text().into());
                }
            },
        );
    });

    let state = controller.clone();
    let weak = ui.as_weak();
    ui.on_select_excel(move || {
        let Some(ui) = weak.upgrade() else { return };
        if ui.get_busy() {
            return;
        }
        let mut dialog = rfd::FileDialog::new()
            .set_title("选择 Excel 文件")
            .add_filter("Excel", &["xlsx", "xls"]);
        dialog = dialog.set_parent(&ui.window().window_handle());
        if let Some(path) = dialog.pick_file() {
            let mut state = state.borrow_mut();
            state.settings.excel_path = path.to_string_lossy().into_owned();
            state.save();
            state.sync(&ui);
            state.read_sheets(&ui);
        }
    });

    let state = controller.clone();
    let weak = ui.as_weak();
    ui.on_select_folder(move || {
        let Some(ui) = weak.upgrade() else { return };
        if ui.get_busy() {
            return;
        }
        let mut dialog = rfd::FileDialog::new().set_title("选择模块文件夹");
        dialog = dialog.set_parent(&ui.window().window_handle());
        if let Some(path) = dialog.pick_folder() {
            let mut state = state.borrow_mut();
            state.settings.xml_folder_path = path.to_string_lossy().into_owned();
            state
                .logs
                .append(&format!("已选择模块文件夹: {}", path.display()));
            state.save();
            state.sync(&ui);
        }
    });

    let state = controller.clone();
    ui.on_quick_mode_changed(move |enabled| {
        let mut state = state.borrow_mut();
        state.settings.quick_mode = enabled;
        state.save();
    });

    let state = controller.clone();
    let weak = ui.as_weak();
    ui.on_clear_log(move || {
        state.borrow_mut().logs.clear();
        if let Some(ui) = weak.upgrade() {
            ui.set_log_text("".into());
        }
    });

    let state = controller.clone();
    let weak = ui.as_weak();
    ui.on_convert(move || {
        let Some(ui) = weak.upgrade() else { return };
        if ui.get_busy() {
            return;
        }
        let mut state = state.borrow_mut();
        match ConversionRequest::new(&state.settings, &state.sheets) {
            Ok(request) => {
                state.save();
                state.logs.append("开始转换…");
                ui.set_busy(true);
                ui.set_status_error(false);
                ui.set_status_text("正在转换，请稍候…".into());
                let events = state.events.clone();
                std::thread::spawn(move || {
                    let _ = events.send(Event::Converted(worker(|| request.run())));
                });
            }
            Err(error) => {
                state.logs.append(&error);
                ui.set_status_error(true);
                ui.set_status_text(error.into());
            }
        }
        ui.set_log_text(state.logs.text().into());
    });

    let state = controller.clone();
    let weak = ui.as_weak();
    ui.window().on_close_requested(move || {
        if let Some(ui) = weak.upgrade() {
            if ui.get_busy() {
                ui.set_status_text("任务尚未完成，请完成后再关闭窗口".into());
                return slint::CloseRequestResponse::KeepWindowShown;
            }
        }
        state.borrow_mut().save();
        slint::CloseRequestResponse::HideWindow
    });

    let state = controller.clone();
    let weak = ui.as_weak();
    let poll_timer = Timer::default();
    poll_timer.start(TimerMode::Repeated, Duration::from_millis(50), move || {
        if let Some(ui) = weak.upgrade() {
            state.borrow_mut().drain(&ui, &receiver);
        }
    });
    (save_timer, poll_timer)
}

#[cfg(test)]
#[path = "../tests/support/mod.rs"]
mod fixtures;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_ui_callbacks_convert_and_restore_ready_state() {
        use slint::platform::{
            software_renderer::{MinimalSoftwareWindow, RepaintBufferType},
            Platform, WindowAdapter,
        };
        struct TestPlatform(Rc<MinimalSoftwareWindow>);
        impl Platform for TestPlatform {
            fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
                Ok(self.0.clone())
            }
        }
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        slint::platform::set_platform(Box::new(TestPlatform(window))).unwrap();
        let root = tempfile::tempdir().unwrap();
        let settings = fixtures::settings(root.path(), true);
        let (events, receiver) = mpsc::sync_channel(128);
        let controller = Rc::new(RefCell::new(Controller {
            settings,
            store: SettingsStore::new(root.path().join("settings.json")),
            sheets: vec![],
            logs: LogBuffer::default(),
            events,
            allow_save: true,
        }));
        let ui = MainWindow::new().unwrap();
        controller.borrow().sync(&ui);
        let (_save_timer, _poll_timer) = wire_ui(&ui, controller.clone(), receiver);
        let complete = || {
            let deadline = std::time::Instant::now() + Duration::from_secs(5);
            while ui.get_busy() {
                assert!(
                    std::time::Instant::now() < deadline,
                    "UI worker did not finish"
                );
                slint::platform::update_timers_and_animations();
                std::thread::sleep(Duration::from_millis(10));
            }
        };
        controller.borrow_mut().read_sheets(&ui);
        complete();
        ui.invoke_convert();
        assert!(ui.get_busy());
        complete();
        assert!(!ui.get_status_error());
        assert!(ui.get_status_text().contains("转换成功"));
        assert!(
            std::fs::read_to_string(root.path().join("module/res/values-zh/strings.xml"))
                .unwrap()
                .contains("你好")
        );
        ui.invoke_quick_mode_changed(false);
        assert!(!controller.borrow().settings.quick_mode);
        ui.invoke_convert();
        complete();
        assert!(ui.get_status_text().contains("转换成功"));
        std::fs::write(
            root.path().join("module/res/values-zh/strings.xml"),
            "<resources><string>",
        )
        .unwrap();
        ui.invoke_convert();
        complete();
        assert!(ui.get_status_error());
        assert!(ui.get_status_text().contains("转换失败"));
        ui.invoke_config_edited("{".into());
        assert!(ui.get_config_error().contains("JSON"));
        ui.invoke_clear_log();
        assert_eq!(ui.get_log_text(), "");
    }

    #[test]
    fn failed_load_does_not_overwrite_original_on_close() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        std::fs::write(&path, "broken").unwrap();
        let (events, _receiver) = mpsc::sync_channel(16);
        let mut controller = Controller {
            settings: Settings::default(),
            store: SettingsStore::new(path.clone()),
            sheets: vec![],
            logs: LogBuffer::default(),
            events,
            allow_save: false,
        };
        controller.save();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "broken");
    }

    #[test]
    fn worker_panic_is_an_error_instead_of_a_stuck_loading_state() {
        assert!(worker::<()>(|| panic!("fixture panic")).is_err());
    }
}
