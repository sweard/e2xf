use calamine::Reader;

mod config;
mod read_excel;
mod write_xml;
mod find_files;

/// 获取Excel文件中的工作表名称列表
pub fn get_sheet_names(file_path: &str) -> Vec<String> {
    let workbook = read_excel::open_excel_workbook(file_path);
    match workbook {
        Ok(workbook)=> workbook.sheet_names(),
        Err(_) => vec![],
    }
   
}

/// 获取默认配置JSON字符串
pub fn get_default_cfg_json() -> String {
    config::CFG_JSON.to_string()
}

/// 更新XML文件
pub fn update(cfg_json: &str, excel_path: &str, xml_dir_path: &str) -> String {
    match write_xml::update(&cfg_json, &excel_path, &xml_dir_path) {
        Ok(_) => "update success".to_string(),
        Err(e) => "更新失败:".to_owned() + &format!("{:?}", e),
    }
}

/// 快速更新XML文件
pub fn quick_update(cfg_json: &str, excel_path: &str, xml_dir_path: &str) -> String {
    match write_xml::quick_update(&cfg_json, &excel_path, &xml_dir_path) {
        Ok(_) => "quick update success".to_string(),
        Err(e) => "更新失败:".to_owned() + &format!("{:?}", e),
    }
}
