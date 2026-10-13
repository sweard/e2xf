use e2xf::settings::Settings;
use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};
use zip::{write::SimpleFileOptions, ZipWriter};

pub const ORIGINAL: &str = "<resources><string name=\"hello\">Old</string></resources>";

fn workbook(path: &Path) {
    let mut zip = ZipWriter::new(File::create(path).unwrap());
    let entries = [
        (
            "[Content_Types].xml",
            r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>"#,
        ),
        (
            "_rels/.rels",
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Translations" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Android tag</t></is></c><c r="B1" t="inlineStr"><is><t>English</t></is></c><c r="C1" t="inlineStr"><is><t>中文</t></is></c></row><row r="2"><c r="A2" t="inlineStr"><is><t>hello</t></is></c><c r="B2" t="inlineStr"><is><t>Hello</t></is></c><c r="C2" t="inlineStr"><is><t>你好</t></is></c></row></sheetData></worksheet>"#,
        ),
    ];
    for (name, data) in entries {
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(data.as_bytes()).unwrap();
    }
    zip.finish().unwrap();
}

pub fn settings(root: &Path, quick_mode: bool) -> Settings {
    let excel = root.join("翻译.xlsx");
    workbook(&excel);
    for folder in ["values", "values-zh"] {
        let path = root.join("module/res").join(folder);
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("strings.xml"), ORIGINAL).unwrap();
    }
    Settings {
        config: serde_json::json!({"sheetName": "Translations", "tagName": "Android tag", "defaultLang": "en", "langMap": {"en": "English", "zh": "中文"}, "escapeOnly": {}, "regex": "", "ignoreFolder": []}).to_string(),
        excel_path: excel.to_string_lossy().into_owned(),
        xml_folder_path: root.join("module").to_string_lossy().into_owned(),
        quick_mode,
    }
}
