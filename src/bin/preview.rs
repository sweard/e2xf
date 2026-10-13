use std::{path::Path, rc::Rc};

use e2xf::MainWindow;
use slint::{
    platform::{
        software_renderer::{MinimalSoftwareWindow, RepaintBufferType},
        Platform, WindowAdapter,
    },
    ComponentHandle, Rgb8Pixel,
};

struct PreviewPlatform(Rc<MinimalSoftwareWindow>);
impl Platform for PreviewPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.0.clone())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = args
        .next()
        .ok_or("Usage: e2xf-preview OUTPUT.png [idle|busy|error|success|about] [WIDTHxHEIGHT]")?;
    let mode = args.next().unwrap_or_else(|| "idle".into());
    let size = args.next().unwrap_or_else(|| "920x840".into());
    let (width, height) = size.split_once('x').ok_or("Expected WIDTHxHEIGHT")?;
    let (width, height) = (width.parse::<u32>()?, height.parse::<u32>()?);
    let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    slint::platform::set_platform(Box::new(PreviewPlatform(window.clone())))?;
    let ui = MainWindow::new()?;
    ui.set_config_text(excel_to_xml::get_default_cfg_json().into());
    ui.set_excel_path("/Users/example/Documents/翻译文案.xlsx".into());
    ui.set_folder_path("/Users/example/StudioProjects/SwitchBot/android/switchbot-app".into());
    ui.set_log_text("e2xf 已启动（Slint + Rust）\n[INFO] 已读取 Excel 工作表: 翻译文案\n[INFO] 默认语言: en\n[INFO] 目标模块: switchbot-app\n中文、英文和其他语言的转换日志会显示在这里。".into());
    match mode.as_str() {
        "idle" | "about" => {}
        "busy" => {
            ui.set_busy(true);
            ui.set_status_text("正在转换，请稍候…".into());
        }
        "error" => {
            ui.set_config_error("Excel 中没有对应的工作表: 无效工作表".into());
            ui.set_status_error(true);
            ui.set_status_text("配置检查失败，请修正后重试".into());
        }
        "success" => {
            ui.set_status_text("转换成功，已更新 10 个 XML 文件".into());
        }
        _ => return Err("Unknown preview state".into()),
    }
    window.set_size(slint::PhysicalSize::new(width, height));
    ui.show()?;
    slint::platform::update_timers_and_animations();
    if mode == "about" {
        let position = slint::LogicalPosition::new(width as f32 - 44.0, 32.0);
        let button = slint::platform::PointerEventButton::Left;
        window.dispatch_event(slint::platform::WindowEvent::PointerPressed { position, button });
        window.dispatch_event(slint::platform::WindowEvent::PointerReleased { position, button });
    }
    window.request_redraw();
    let mut pixels = vec![Rgb8Pixel::default(); width as usize * height as usize];
    window.draw_if_needed(|renderer| {
        renderer.render(&mut pixels, width as usize);
    });
    let bytes: Vec<u8> = pixels
        .iter()
        .flat_map(|pixel| [pixel.r, pixel.g, pixel.b])
        .collect();
    let mut encoder = png::Encoder::new(
        std::io::BufWriter::new(std::fs::File::create(Path::new(&output))?),
        width,
        height,
    );
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&bytes)?;
    ui.hide()?;
    println!("{output}");
    Ok(())
}
