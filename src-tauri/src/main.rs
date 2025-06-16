#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::path::Path;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tauri_plugin_dialog::DialogExt;

#[derive(Debug, Deserialize, Serialize)]
struct OrganizeRule {
    name: String,
    source_folder: String,
    pattern: String,
    destination_folder: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    rules: Vec<OrganizeRule>,
}

#[tauri::command]
async fn load_config(config_path: String) -> Result<Config, String> {
    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("設定ファイルの読み込みに失敗しました: {}", e))?;
    
    let config: Config = serde_yaml::from_str(&content)
        .map_err(|e| format!("YAML解析に失敗しました: {}", e))?;
    
    Ok(config)
}

#[tauri::command]
async fn organize_files(config_path: String) -> Result<Vec<String>, String> {
    let config = load_config(config_path).await?;
    let mut results = Vec::new();
    
    for rule in config.rules {
        let source_path = Path::new(&rule.source_folder);
        let dest_path = Path::new(&rule.destination_folder);
        
        if !source_path.exists() {
            results.push(format!("警告: ソースフォルダが存在しません: {}", rule.source_folder));
            continue;
        }
        
        // 宛先フォルダを作成
        if !dest_path.exists() {
            fs::create_dir_all(dest_path)
                .map_err(|e| format!("宛先フォルダの作成に失敗しました: {}", e))?;
        }
        
        let regex = Regex::new(&rule.pattern)
            .map_err(|e| format!("正規表現が無効です ({}): {}", rule.pattern, e))?;
        
        let entries = fs::read_dir(source_path)
            .map_err(|e| format!("フォルダの読み取りに失敗しました: {}", e))?;
        
        let mut moved_count = 0;
        
        for entry in entries {
            let entry = entry.map_err(|e| format!("ファイルエントリの読み取りに失敗しました: {}", e))?;
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();
            
            if entry.file_type().map_err(|e| format!("ファイルタイプの取得に失敗しました: {}", e))?.is_file() {
                if regex.is_match(&file_name_str) {
                    let source_file = entry.path();
                    let dest_file = dest_path.join(&file_name);
                    
                    match fs::rename(&source_file, &dest_file) {
                        Ok(_) => {
                            moved_count += 1;
                            results.push(format!("移動: {} -> {}", 
                                source_file.display(), 
                                dest_file.display()));
                        }
                        Err(e) => {
                            results.push(format!("移動失敗 {}: {}", source_file.display(), e));
                        }
                    }
                }
            }
        }
        
        results.push(format!("ルール '{}': {}個のファイルを移動しました", rule.name, moved_count));
    }
    
    Ok(results)
}

// ファイル・フォルダ選択機能は一旦無効化
#[tauri::command]
async fn select_folder() -> Result<String, String> {
    Err("フォルダ選択機能は現在無効です。手動でパスを入力してください。".to_string())
}

#[tauri::command]
async fn select_file(app_handle: tauri::AppHandle) -> Result<String, String> {
    let file_path = app_handle.dialog()
        .file()
        .add_filter("YAML", &["yaml", "yml"])
        .blocking_pick_file();
    
    match file_path {
        Some(path) => {
            match path.as_path() {
                Some(p) => Ok(p.to_string_lossy().into_owned()),
                None => Err("パスの取得に失敗しました".to_string())
            }
        },
        None => Err("ファイル選択がキャンセルされました".to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            load_config,
            organize_files,
            select_folder,
            select_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() {
    run();
}