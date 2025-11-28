use std::{
    collections::{HashMap, HashSet},
    fs::{remove_file, rename, File},
    io::{BufReader, BufWriter},
};

use crate::{config::ParsedCfg, find_files, read_excel};
use calamine::{open_workbook, Reader, Xlsx};
use quick_xml::{
    events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event},
    Writer,
};
use regex::Regex;

const XML_NAME: &str = "name";
const XML_B_NAME: &[u8] = b"name";
// 格式化相关常量
const XML_NEWLINE: &str = "\n";
const XML_INDENT: &str = "\n    ";
const XML_STRING: &str = "string";
const XML_B_STRING: &[u8] = b"string";
const XML_RESOURCES: &str = "resources";
const XML_B_RESOURCES: &[u8] = b"resources";

#[derive(Default)]
struct PathIndex {
    path: String,
    index: u32,
    lang: String,
}

/// 获取解析后的数据
/// - excel_path: Excel文件路径
/// - parsed_cfg: 解析后的配置
/// - paths: 找到的XML文件路径列表
fn get_parsed_data<'a>(
    cfg_json: &str,
    excel_path: &'a str,
    xml_dir_path: &str,
) -> Option<(&'a str, ParsedCfg, Vec<String>)> {
    let cfg = read_excel::parse_cfg_with_excel(excel_path, cfg_json);
    if cfg.is_err() {
        println!("解析配置时出错: {:?}", cfg.err());
        return None;
    }
    let parsed_cfg = cfg.unwrap();
    println!("解析配置成功: {:?}", parsed_cfg);
    let ignore_folders: Vec<&str> = parsed_cfg.ignore_folder.iter().map(|s| s.as_str()).collect();
    let forder = find_files::find_target_folder(
        &xml_dir_path,
        "res",
        &ignore_folders,
    );
    if forder.is_none() {
        println!("未找到res文件夹");
        return None;
    }
    let res_folder = forder.unwrap();
    println!("找到res文件夹: {}", res_folder);
    let paths = find_files::collect_target_files(&res_folder, "values", "strings.xml");
    return Some((excel_path, parsed_cfg, paths));
}

/// 准备需要写入的数据
pub fn update(
    cfg_json: &str,
    excel_path: &str,
    xml_dir_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let (file_path, parsed_cfg, paths) = match get_parsed_data(cfg_json, excel_path, xml_dir_path) {
        Some((file_path, parsed_cfg, paths)) => (file_path, parsed_cfg, paths),
        None => return Ok(()),
    };
    let tag_index = parsed_cfg.tag_index;
    // 预先打开Excel文件，只打开一次
    let mut workbook: Xlsx<_> = open_workbook(file_path)?;

    // 获取sheet名称（只需获取一次）
    let sheet_name = if parsed_cfg.sheet_name.is_empty() {
        workbook
            .sheet_names()
            .get(0)
            .cloned()
            .ok_or("No sheets found")?
    } else {
        parsed_cfg.sheet_name.clone()
    };

    // 预分配一个HashMap，循环内复用
    let mut tag_value_map = HashMap::with_capacity(5000);

    let mut default_valug_map: HashMap<String, String> = HashMap::new();
    let default_lang = &parsed_cfg.default_lang;
    let replace_blank_with_default = parsed_cfg.replace_blank_with_default;

    let mut lang_index_map = parsed_cfg.lang_index_map.clone();
    // 将默认语言移到第一个位置
    if let Some(pos) = lang_index_map
        .iter()
        .position(|(lang, _)| &lang == &default_lang)
    {
        // 将默认语言换到第一个位置
        lang_index_map.swap(0, pos);
    }

    // 遍历语言索引映射
    for (lang, index) in &lang_index_map {
        let is_default_lang = lang == default_lang;
        let end_point = if is_default_lang {
            "values/strings.xml".to_string()
        } else {
            format!("values-{}/strings.xml", lang)
        };

        // 查找匹配的XML文件路径
        let path = match paths.iter().find(|path| path.ends_with(&end_point)) {
            Some(path) => path,
            None => continue, // 没找到对应语言的文件，跳过这个语言
        };

        // 清空map，准备复用
        tag_value_map.clear();

        // 构建标签值映射 - 复用workbook、sheet_name和tag_value_map
        let lang_index = *index;
        read_excel::process_excel_single_lang(
            &mut workbook,
            &sheet_name,
            tag_index,
            lang_index,
            &mut tag_value_map,
        )?;
        // 如果是默认语言，将tag_value_map的内容复制到default_valug_map
        // 以便后续处理空值
        if is_default_lang && replace_blank_with_default {
            // 处理默认语言,使用空default_valug_map
            update_xml_file(path, &tag_value_map, &default_valug_map, &parsed_cfg)?;
            default_valug_map = tag_value_map.clone();
        } else {
            // 处理XML文件
            update_xml_file(path, &tag_value_map, &default_valug_map, &parsed_cfg)?;
        }
    }
    tag_value_map.clear(); // 清空map
    tag_value_map.shrink_to_fit(); // 释放内存
    Ok(())
}

/// 快速更新，占用更多内存
pub fn quick_update(
    cfg_json: &str,
    excel_path: &str,
    xml_dir_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let (file_path, parsed_cfg, paths) = match get_parsed_data(cfg_json, excel_path, xml_dir_path) {
        Some((file_path, parsed_cfg, paths)) => (file_path, parsed_cfg, paths),
        None => return Ok(()),
    };
    let tag_index = parsed_cfg.tag_index;

    // 预先打开Excel文件，只打开一次
    let mut workbook: Xlsx<_> = open_workbook(file_path)?;

    // 获取sheet名称（只需获取一次）
    let sheet_name = if parsed_cfg.sheet_name.is_empty() {
        workbook
            .sheet_names()
            .get(0)
            .cloned()
            .ok_or("No sheets found")?
    } else {
        parsed_cfg.sheet_name.clone()
    };

    // 预分配一个HashMap，循环内复用
    let mut tag_value_map: HashMap<String, HashMap<u32, String>> = HashMap::with_capacity(5000);
    let mut path_index_vec: Vec<PathIndex> = Vec::with_capacity(paths.len());
    let default_lang = &parsed_cfg.default_lang;
    let replace_blank_with_default = parsed_cfg.replace_blank_with_default;
    let mut default_valug_map: HashMap<String, String> = HashMap::new();
    // 遍历语言索引映射
    for (lang, index) in &parsed_cfg.lang_index_map {
        let is_default_lang = lang == default_lang;
        let end_point = if is_default_lang {
            "values/strings.xml".to_string()
        } else {
            format!("values-{}/strings.xml", lang)
        };

        // 查找匹配的XML文件路径
        let path = match paths.iter().find(|path| path.ends_with(&end_point)) {
            Some(path) => path,
            None => continue, // 没找到对应语言的文件，跳过这个语言
        };
        let path_index = PathIndex {
            path: path.to_string(),
            index: *index,
            lang: lang.to_string(),
        };
        path_index_vec.push(path_index);
    }
    let lang_index_vec: Vec<u32> = path_index_vec.iter().map(|p| p.index).collect();

    read_excel::process_excel_multi_lang(
        &mut workbook,
        &sheet_name,
        tag_index,
        lang_index_vec,
        &mut tag_value_map,
    )?;
    println!("tag_value_map size: {}", tag_value_map.len());
    if let Some(pos) = path_index_vec.iter().position(|p| &p.lang == default_lang) {
        // 默认语言在生成时，已被移动到index 0
        path_index_vec.swap(0, pos);
    }

    // 处理XML文件
    path_index_vec.iter().for_each(|p| {
        let index = p.index;
        let path = &p.path;
        let lang = &p.lang;
        let mut write_map: HashMap<String, String> = HashMap::new();
        tag_value_map.iter().for_each(|(tag, map)| {
            write_map.insert(tag.to_string(), map.get(&index).unwrap().to_string());
        });

        let is_default_lang = lang == default_lang;
        // 如果是默认语言，将tag_value_map的内容复制到default_valug_map
        // 以便后续处理空值
        let res = if is_default_lang && replace_blank_with_default {
            // 处理默认语言,使用空default_valug_map
            let res = update_xml_file(path, &write_map, &default_valug_map, &parsed_cfg);
            // 处理完后，默认语言的值会被复制到default_valug_map
            default_valug_map = write_map.clone();
            res
        } else {
            update_xml_file(path, &write_map, &default_valug_map, &parsed_cfg)
        };
        write_map.clear(); // 清空map
        if res.is_err() {
            println!(
                "更新XML文件失败,lang index: {}, err: {:?}",
                index,
                res.err()
            );
        }
    });
    tag_value_map.clear(); // 清空map
    tag_value_map.shrink_to_fit(); // 释放内存

    Ok(())
}

/// 更新XML文件
fn update_xml_file(
    path: &str,
    tag_value_map: &HashMap<String, String>,
    default_valug_map: &HashMap<String, String>,
    parsed_cfg: &ParsedCfg,
) -> Result<(), Box<dyn std::error::Error>> {
    // 创建临时文件路径
    let temp_path = format!("{}.temp", path);
    // 正则表达式
    let regex_str = &parsed_cfg.regex;
    let regex = if !is_blank(regex_str) {
        match Regex::new(regex_str) {
            Ok(regex) => Some(regex),
            Err(e) => {
                println!("正则表达式错误: {:?}", e);
                None
            }
        }
    } else {
        None
    };

    // 打开原始XML文件和临时文件
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut xml_reader = quick_xml::Reader::from_reader(reader);

    let output_file = File::create(&temp_path)?;
    let writer = BufWriter::new(output_file);
    let mut xml_writer = Writer::new(writer);

    let mut buf = Vec::with_capacity(4 * 1024);
    let mut current_tag_name = None;
    let mut updated_tags = HashSet::new();

    let replace_blank_with_default = parsed_cfg.replace_blank_with_default;
    let disable_escape = parsed_cfg.disable_escape;
    let escape_only = &parsed_cfg.escape_only;
    if parsed_cfg.reset {
        // 重写XML文件
        // 1. 写入XML声明
        xml_writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))?;

        // 2. 添加换行
        xml_writer.write_event(Event::Text(BytesText::new(XML_NEWLINE)))?;

        // 3. 打开resources标签
        xml_writer.write_event(Event::Start(BytesStart::new(XML_RESOURCES)))?;

        // 4. 按照tag_value_map写入所有标签
        for (tag, value) in tag_value_map {
            // 添加换行和缩进
            xml_writer.write_event(Event::Text(BytesText::new(XML_INDENT)))?;

            // 创建string标签
            let mut elem = BytesStart::new(XML_STRING);
            elem.push_attribute((XML_NAME, tag.as_str()));
            xml_writer.write_event(Event::Start(elem))?;
            let write_value =
                get_write_value(tag, value, default_valug_map, replace_blank_with_default);
            write_text(
                disable_escape,
                &mut xml_writer,
                write_value,
                escape_only,
                &regex,
            )?;
            xml_writer.write_event(Event::End(BytesEnd::new(XML_STRING)))?;
        }

        // 5. 添加最后的换行
        xml_writer.write_event(Event::Text(BytesText::new(XML_NEWLINE)))?;

        // 6. 关闭resources标签
        xml_writer.write_event(Event::End(BytesEnd::new(XML_RESOURCES)))?;
    } else {
        // 更新XML文件
        loop {
            match xml_reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    if e.name().as_ref() == XML_B_STRING {
                        // 提取name属性
                        for attr in e.attributes() {
                            if let Ok(attr) = attr {
                                if attr.key.as_ref() == XML_B_NAME {
                                    if let Ok(tag_name) = std::str::from_utf8(attr.value.as_ref()) {
                                        if tag_value_map.contains_key(tag_name) {
                                            current_tag_name = Some(tag_name.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    xml_writer.write_event(Event::Start(e.to_owned()))?;
                }

                Ok(Event::End(ref e)) => {
                    if e.name().as_ref() == XML_B_RESOURCES {
                        // 在关闭resources标签前添加缺失的标签
                        add_missing_tags(
                            &mut xml_writer,
                            tag_value_map,
                            default_valug_map,
                            &updated_tags,
                            parsed_cfg,
                            &regex,
                        )?;
                    }
                    xml_writer.write_event(Event::End(e.to_owned()))?;
                }

                Ok(Event::Text(e)) => {
                    // 提前处理没有当前标签的情况
                    if current_tag_name.is_none() {
                        xml_writer.write_event(Event::Text(e.to_owned()))?;
                        continue;
                    }

                    let tag_name = current_tag_name.as_ref().unwrap();
                    updated_tags.insert(tag_name.to_string());

                    // 更新文本内容
                    match tag_value_map.get(tag_name) {
                        Some(value) if !value.is_empty() => {
                            let write_value = get_write_value(
                                tag_name,
                                value,
                                default_valug_map,
                                replace_blank_with_default,
                            );
                            write_text(
                                disable_escape,
                                &mut xml_writer,
                                write_value,
                                escape_only,
                                &regex,
                            )?;
                        }
                        _ => {
                            xml_writer.write_event(Event::Text(e.to_owned()))?;
                        }
                    }
                    current_tag_name = None;
                }

                Ok(Event::Eof) => break,
                Ok(e) => xml_writer.write_event(e)?,
                Err(e) => return Err(e.into()),
            }
            buf.clear();
        }
    }
    drop(xml_writer);
    drop(xml_reader);
    // 替换原文件
    remove_file(path)?;
    rename(temp_path, path)?;

    Ok(())
}

/// 添加缺失的标签
fn add_missing_tags(
    xml_writer: &mut Writer<BufWriter<File>>,
    tag_value_map: &HashMap<String, String>,
    default_valug_map: &HashMap<String, String>,
    updated_tags: &HashSet<String>,
    parsed_cfg: &ParsedCfg,
    regex: &Option<Regex>,
) -> Result<(), Box<dyn std::error::Error>> {
    let disable_escape = parsed_cfg.disable_escape;
    let escape_only = &parsed_cfg.escape_only;
    let mut missing_tag_added = false;
    for (tag, value) in tag_value_map {
        if !updated_tags.contains(tag) {
            if !missing_tag_added {
                // 结束时，需要添加换行和缩进
                missing_tag_added = true;
            }
            // 为每个新标签添加缩进
            xml_writer.write_event(Event::Text(BytesText::new(XML_INDENT)))?;

            // 创建新标签
            let mut elem = BytesStart::new(XML_STRING);
            elem.push_attribute((XML_NAME, tag.as_str()));

            xml_writer.write_event(Event::Start(elem))?;
            let write_value = get_write_value(
                tag,
                value,
                default_valug_map,
                parsed_cfg.replace_blank_with_default,
            );
            write_text(disable_escape, xml_writer, write_value, escape_only, regex)?;
            xml_writer.write_event(Event::End(BytesEnd::new(XML_STRING)))?;
        }
    }
    if missing_tag_added {
        // 如果写入了新tag，添加换行和缩进
        xml_writer.write_event(Event::Text(BytesText::new(XML_NEWLINE)))?;
    }
    Ok(())
}

fn write_text(
    disable_escape: bool,
    xml_writer: &mut Writer<BufWriter<File>>,
    value: &str,
    escape_only: &Vec<(String, String)>,
    regex: &Option<Regex>,
) -> Result<(), Box<dyn std::error::Error>> {
    let value = if let Some(regex) = regex {
        // 使用正则表达式替换
        regex.replace_all(value, "").to_string()
    } else {
        value.to_string()
    };
    if disable_escape {
        // 直接写入文本内容，不会再自动转义
        xml_writer.write_event(Event::Text(BytesText::from_escaped(value)))?;
    } else {
        if escape_only.is_empty() {
            // 转义所有内容
            xml_writer.write_event(Event::Text(BytesText::new(&value)))?;
        } else {
            // 只转义指定的内容
            let mut escaped_value = value.to_string();
            for (key, val) in escape_only {
                escaped_value = escaped_value.replace(key, val);
            }
            xml_writer.write_event(Event::Text(BytesText::from_escaped(&escaped_value)))?;
        }
    }
    Ok(())
}

fn is_blank(value: &str) -> bool {
    value.trim().is_empty()
}

fn get_write_value<'a>(
    tag: &String,
    value: &'a String,
    default_valug_map: &'a HashMap<String, String>,
    replace_blank_with_default: bool,
) -> &'a String {
    if is_blank(value) && !default_valug_map.is_empty() && replace_blank_with_default {
        // 如果值为空，尝试使用默认语言的值替换
        match default_valug_map.get(tag) {
            Some(default_value) => default_value,
            None => value,
        }
    } else {
        value
    }
}
