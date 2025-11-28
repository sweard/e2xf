use std::{fs::read_dir, path::Path};

/**
 * 收集目标文件
 * @param path 路径
 * @param target_dir_name 目标文件夹名称
 * @param target 目标文件名称
 */
pub fn collect_target_files(path: &str, target_dir_name: &str, target: &str) -> Vec<String> {
    let path = Path::new(path);
    if !path.is_dir() {
        return Vec::new(); // 当前路径不是文件夹，返回空向量
    }

    // 遍历子目录并收集目标文件
    read_dir(path)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.flatten())
        .filter_map(|entry| {
            let entry_path = entry.path();
            if !entry_path.is_dir() {
                return None; // 不是文件夹，跳过
            }

            let dir_name = entry.file_name().to_str()?.to_owned();
            if !dir_name.contains(target_dir_name) {
                return None; // 文件夹名称不包含目标文件夹名称，跳过
            }

            // 查找符合条件的文件
            read_dir(entry_path)
                .ok()
                .into_iter()
                .flat_map(|files| files.flatten())
                .find_map(|file| {
                    let file_path = file.path();
                    if file_path.is_file() && file_path.to_str()?.ends_with(target) {
                        println!("找到目标文件: {}", file_path.display());
                        Some(file_path.to_str()?.to_string())
                    } else {
                        None
                    }
                })
        })
        .collect()
}

/**
 * 查找目标文件夹
 * @param input 输入路径
 * @param target 目标文件夹名称
 * @param ignore 忽略的文件夹
 */
pub fn find_target_folder(input: &str, target: &str, ignore: &[&str]) -> Option<String> {
    let path = Path::new(input);
    if !path.is_dir() {
        return None; // 当前路径不是文件夹
    }

    // 检查当前路径是否为目标文件夹
    if path.file_name().unwrap_or_default() == target {
        if ignore.iter().any(|&x| input.contains(x)) {
            return None; // 忽略指定的文件夹
        }
        return Some(input.to_string());
    }

    // 遍历子目录递归查找目标文件夹
    read_dir(path)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                find_target_folder(entry_path.to_str()?, target, ignore)
            } else {
                None
            }
        })
        .next()
}