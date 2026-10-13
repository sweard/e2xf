use std::{cell::Cell, rc::Rc, time::Duration};

use e2xf::MainWindow;
use slint::{
    platform::{
        software_renderer::{MinimalSoftwareWindow, RepaintBufferType},
        Platform, PointerEventButton, WindowAdapter, WindowEvent,
    },
    ComponentHandle, LogicalPosition, PhysicalSize, Rgb8Pixel,
};

struct TestPlatform {
    window: Rc<MinimalSoftwareWindow>,
    now: Rc<Cell<Duration>>,
}

impl Platform for TestPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.window.clone())
    }

    fn duration_since_start(&self) -> Duration {
        self.now.get()
    }
}

fn render(
    window: &MinimalSoftwareWindow,
    now: &Cell<Duration>,
    width: u32,
    height: u32,
    name: &str,
) -> Vec<u8> {
    let mut pixels = vec![Rgb8Pixel::default(); (width * height) as usize];
    // Settle deferred property changes, layout and scrollbar animations.
    for _ in 0..4 {
        now.set(now.get() + Duration::from_millis(250));
        slint::platform::update_timers_and_animations();
        window.request_redraw();
        window.draw_if_needed(|renderer| {
            renderer.render(&mut pixels, width as usize);
        });
    }
    let bytes: Vec<u8> = pixels.iter().flat_map(|p| [p.r, p.g, p.b]).collect();
    #[cfg(feature = "preview")]
    if let Some(directory) = std::env::var_os("E2XF_SCROLL_SCREENSHOT_DIR") {
        let file = std::fs::File::create(
            std::path::Path::new(&directory).join(format!("{name}-{width}x{height}.png")),
        )
        .unwrap();
        let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&bytes)
            .unwrap();
    }
    #[cfg(not(feature = "preview"))]
    let _ = name;

    // Compare only log text, excluding the border and the animated scrollbar.
    (height - 70..height - 24)
        .flat_map(|y| {
            let start = ((y * width + 24) * 3) as usize;
            let end = ((y * width + width - 42) * 3) as usize;
            bytes[start..end].iter().copied()
        })
        .collect()
}

fn scroll(window: &MinimalSoftwareWindow, width: u32, height: u32, delta_y: f32) {
    window.dispatch_event(WindowEvent::PointerScrolled {
        position: LogicalPosition::new(width as f32 / 2.0, height as f32 - 50.0),
        delta_x: 0.0,
        delta_y,
    });
}

#[test]
fn long_logs_follow_updates_and_allow_manual_scrolling() {
    let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    let now = Rc::new(Cell::new(Duration::ZERO));
    slint::platform::set_platform(Box::new(TestPlatform {
        window: window.clone(),
        now: now.clone(),
    }))
    .unwrap();
    let ui = MainWindow::new().unwrap();
    ui.set_config_text(excel_to_xml::get_default_cfg_json().into());
    ui.show().unwrap();
    let history = (1..=200)
        .map(|line| {
            format!(
                "[INFO] 日志 {line:04}: {}",
                "这是一条较长的转换日志，用于验证自动换行和滚动。".repeat(5)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    for (width, height) in [(920, 840), (660, 660)] {
        window.set_size(PhysicalSize::new(width, height));
        ui.set_log_text("".into());
        let empty = render(&window, &now, width, height, "empty");
        ui.set_log_text(history.clone().into());
        let before = render(&window, &now, width, height, "history");
        let first_update = format!("{history}\n最新日志 0001：转换完成。");
        ui.set_log_text(first_update.clone().into());
        let latest = render(&window, &now, width, height, "latest");
        assert!(before != latest, "new logs must be visible at the bottom");

        scroll(&window, width, height, 120.0);
        let manual = render(&window, &now, width, height, "manual-wheel");
        assert!(latest != manual, "mouse wheel must scroll to older logs");
        assert!(
            manual == render(&window, &now, width, height, "manual-stable"),
            "manual scrolling must not snap back on redraw"
        );

        // An equally tall buffer (as when the 1000-line limit is reached) must follow too.
        let same_height_update = format!("{history}\n最新日志 0002：转换完成。");
        ui.set_log_text(same_height_update.clone().into());
        let same_height = render(&window, &now, width, height, "same-height-update");
        scroll(&window, width, height, -100_000.0);
        assert!(
            same_height == render(&window, &now, width, height, "same-height-bottom"),
            "a log update must follow even when content height does not change"
        );

        scroll(&window, width, height, 120.0);
        render(&window, &now, width, height, "before-append");
        ui.set_log_text(format!("{same_height_update}\n最新日志 0003：已保存所有文件。").into());
        let appended = render(&window, &now, width, height, "appended");
        scroll(&window, width, height, -100_000.0);
        assert!(
            appended == render(&window, &now, width, height, "appended-bottom"),
            "appending after manual scrolling must reveal the latest log"
        );

        let thumb = LogicalPosition::new(width as f32 - 31.0, height as f32 - 48.0);
        let older = LogicalPosition::new(thumb.x, thumb.y - 30.0);
        window.dispatch_event(WindowEvent::PointerPressed {
            position: thumb,
            button: PointerEventButton::Left,
        });
        window.dispatch_event(WindowEvent::PointerMoved { position: older });
        window.dispatch_event(WindowEvent::PointerReleased {
            position: older,
            button: PointerEventButton::Left,
        });
        assert!(
            appended != render(&window, &now, width, height, "manual-scrollbar"),
            "dragging the scrollbar must reveal older logs"
        );
        window.dispatch_event(WindowEvent::PointerExited);
        ui.set_log_text("".into());
        assert!(empty == render(&window, &now, width, height, "cleared"));
        ui.set_log_text(first_update.into());
        assert!(latest == render(&window, &now, width, height, "after-clear"));

        scroll(&window, width, height, 120.0);
        render(&window, &now, width, height, "before-resize");
        let (width, height) = (width + 40, height + 40);
        window.set_size(PhysicalSize::new(width, height));
        let resized = render(&window, &now, width, height, "resized");
        scroll(&window, width, height, -100_000.0);
        assert!(
            resized == render(&window, &now, width, height, "resized-bottom"),
            "resizing and rewrapping must keep the latest log visible"
        );
    }
    ui.hide().unwrap();
}
