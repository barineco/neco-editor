use std::fs;
use std::path::PathBuf;

#[tauri::command]
fn neco_read_file(path: String) -> Result<String, String> {
    fs::read_to_string(PathBuf::from(path)).map_err(|e| e.to_string())
}

#[tauri::command]
fn neco_write_file(path: String, contents: String) -> Result<(), String> {
    fs::write(PathBuf::from(path), contents).map_err(|e| e.to_string())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            neco_read_file,
            neco_write_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running neco-editor-gui");
}
