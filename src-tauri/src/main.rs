#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::path::Path;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tauri_plugin_dialog::DialogExt;
use std::io::Write;
use dirs::config_dir;
use tauri::{AppHandle, Manager};
use chrono::Local;
use tokio::sync::oneshot;

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
    // ログファイルのパスを決定
    #[cfg(debug_assertions)]
    let mut log_file = Some(std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("C:/python/file-organizer-2025-06-15/file-organizer.log")
        .map_err(|e| format!("ログファイルの作成/オープンに失敗しました: {}", e))?);
    #[cfg(not(debug_assertions))]
    let mut log_file: Option<std::fs::File> = None;
    
    for rule in config.rules {
        let source_path = Path::new(&rule.source_folder);
        let dest_path = Path::new(&rule.destination_folder);
        if let Some(ref mut log_file) = log_file {
            writeln!(log_file, "--- ルール: {} (パターン: {}) ---", rule.name, rule.pattern).ok();
        }
        if !source_path.exists() {
            results.push(format!("警告: ソースフォルダが存在しません: {}", rule.source_folder));
            if let Some(ref mut log_file) = log_file {
                writeln!(log_file, "警告: ソースフォルダが存在しません: {}", rule.source_folder).ok();
            }
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
            let is_file = entry.file_type().map_err(|e| format!("ファイルタイプの取得に失敗しました: {}", e))?.is_file();
            if is_file {
                let matched = regex.is_match(&file_name_str);
                if let Some(ref mut log_file) = log_file {
                    writeln!(log_file, "ファイル: {} → マッチ: {}", file_name_str, matched).ok();
                }
                if matched {
                    let source_file = entry.path();
                    let dest_file = dest_path.join(&file_name);
                    match fs::copy(&source_file, &dest_file) {
                        Ok(_) => {
                            match fs::remove_file(&source_file) {
                                Ok(_) => {
                                    moved_count += 1;
                                    results.push(format!("コピー+削除で移動: {} -> {}", source_file.display(), dest_file.display()));
                                    if let Some(ref mut log_file) = log_file {
                                        writeln!(log_file, "コピー+削除で移動: {} -> {}", source_file.display(), dest_file.display()).ok();
                                    }
                                }
                                Err(e2) => {
                                    results.push(format!("コピー後の削除失敗 {}: {}", source_file.display(), e2));
                                    if let Some(ref mut log_file) = log_file {
                                        writeln!(log_file, "コピー後の削除失敗 {}: {}", source_file.display(), e2).ok();
                                    }
                                }
                            }
                        }
                        Err(e2) => {
                            results.push(format!("移動失敗 {}: {}", source_file.display(), e2));
                            if let Some(ref mut log_file) = log_file {
                                writeln!(log_file, "copy失敗 {}: {}", source_file.display(), e2).ok();
                            }
                        }
                    }
                }
            }
        }
        results.push(format!("ルール '{}': {}個のファイルを移動しました", rule.name, moved_count));
        if let Some(ref mut log_file) = log_file {
            writeln!(log_file, "ルール '{}': {}個のファイルを移動しました", rule.name, moved_count).ok();
        }
    }
    Ok(results)
}

async fn backup_rules_handler(app_handle: &AppHandle) -> Result<String, String> {
    let last_config_path_str = get_last_config_path()?
        .ok_or("最後に使用した設定ファイルが見つかりません。まずは一度ルールを読み込んで実行してください。")?;
    let source_path = Path::new(&last_config_path_str);

    if !source_path.exists() {
        return Err(format!("バックアップ元のファイルが見つかりません: {}", last_config_path_str));
    }
    
    let (tx, rx) = oneshot::channel();
    app_handle.dialog().file().pick_folder(move |path| {
        let _ = tx.send(path);
    });

    let dest_folder_option = rx.await.map_err(|e| e.to_string())?;

    if let Some(dest_folder) = dest_folder_option {
        if let Some(dest_folder_path) = dest_folder.as_path() {
            let now = Local::now();
            let timestamp = now.format("%Y年%m月%d日%H時%M分%S秒").to_string();
            
            let original_filename = source_path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("rules.yaml");

            let backup_filename = format!("{}.backup({})", original_filename, timestamp);
            
            let dest_path = dest_folder_path.join(backup_filename);

            fs::copy(&source_path, &dest_path)
                .map_err(|e| format!("バックアップに失敗しました: {}", e))?;
            
            Ok(format!("バックアップが完了しました。\n保存先: {}", dest_path.to_string_lossy()))
        } else {
            Err("無効なフォルダパスが選択されました。".to_string())
        }
    } else {
        Ok("バックアップ処理はキャンセルされました。".to_string())
    }
}

// ファイル・フォルダ選択機能は一旦無効化
#[tauri::command]
async fn select_folder() -> Result<String, String> {
    Err("フォルダ選択機能は現在無効です。手動でパスを入力してください。".to_string())
}

#[tauri::command]
async fn select_file(app_handle: tauri::AppHandle) -> Result<String, String> {
    let (tx, rx) = oneshot::channel();
    app_handle.dialog()
        .file()
        .add_filter("YAML", &["yaml", "yml"])
        .pick_file(move |file_path| {
            let _ = tx.send(file_path);
        });
    
    let file_path_option = rx.await.map_err(|e| e.to_string())?;
    
    match file_path_option {
        Some(path) => {
            if let Some(p) = path.as_path() {
                Ok(p.to_string_lossy().into_owned())
            } else {
                Err("パスの変換に失敗しました".to_string())
            }
        }
        None => Err("ファイル選択がキャンセルされました".to_string())
    }
}

#[tauri::command]
async fn save_last_config_path(_app_handle: tauri::AppHandle, config_path: String) -> Result<(), String> {
    let config_dir = config_dir()
        .ok_or_else(|| "設定ディレクトリの取得に失敗しました".to_string())?
        .join("file-organizer");
    fs::create_dir_all(&config_dir)
        .map_err(|e| format!("設定ディレクトリの作成に失敗しました: {}", e))?;
    let config_file = config_dir.join("last_config.txt");
    let mut file = fs::File::create(config_file)
        .map_err(|e| format!("設定ファイルの作成に失敗しました: {}", e))?;
    file.write_all(config_path.as_bytes())
        .map_err(|e| format!("設定の保存に失敗しました: {}", e))?;
    Ok(())
}

fn get_last_config_path() -> Result<Option<String>, String> {
    let config_dir = config_dir()
        .ok_or_else(|| "設定ディレクトリの取得に失敗しました".to_string())?
        .join("file-organizer");
    let config_file = config_dir.join("last_config.txt");
    if !config_file.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(config_file)
        .map_err(|e| format!("設定ファイルの読み込みに失敗しました: {}", e))?;
    Ok(Some(content))
}

#[tauri::command]
async fn load_last_config_path(_app_handle: tauri::AppHandle) -> Result<Option<String>, String> {
    get_last_config_path()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle();
            let backup_item =
                tauri::menu::MenuItem::with_id(handle, "backup_rules", "ルールのバックアップ", true, None::<&str>)?;
            let menu = tauri::menu::Menu::with_items(handle, &[&backup_item])?;
            app.set_menu(menu)?;
            Ok(())
        })
        .on_menu_event(|window, event| {
            if event.id() == "backup_rules" {
                let window = window.clone();
                tauri::async_runtime::spawn(async move {
                    let app_handle = window.app_handle();
                    let result = backup_rules_handler(app_handle).await;
                    let dialog = app_handle.dialog();
                    match result {
                        Ok(message) => {
                            dialog.message(message).show(|_| {});
                        }
                        Err(e) => {
                            dialog.message(e).show(|_| {});
                        }
                    }
                });
            }
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            load_config,
            organize_files,
            select_folder,
            select_file,
            save_last_config_path,
            load_last_config_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() {
    run();
}