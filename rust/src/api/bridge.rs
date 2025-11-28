extern crate excel_to_xml;

#[flutter_rust_bridge::frb(sync)] // Synchronous mode for simplicity of the demo
pub fn greet(name: String) -> String {
    format!("Hello Rust, {name}!")
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    // Default utilities - feel free to customize
    flutter_rust_bridge::setup_default_user_utils();
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
