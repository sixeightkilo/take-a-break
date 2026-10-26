#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod player;
mod ui_handler;
mod utils;

use log::debug;
// use tauri::Manager;
use tokio::sync::mpsc;

fn main() {
    utils::setup_logger();
    let log_path = utils::get_log_file_name();
    debug!("Logging to {}", log_path);

    debug!("Starting up.");
    // 1. Create channel OUTSIDE
    let (tx, rx) = mpsc::channel::<String>(10);

    tauri::Builder::default()
        // 2. Register state
        .manage(tx.clone())
        .setup(|app| {
            // --- Devtools (optional) ---
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }

            // --- Background UI handler ---
            let handle = app.handle().clone();
            let ui = ui_handler::UiHandler::new(rx, handle);
            ui.run();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![command, is_debug_build])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
async fn command(payload: String, tx: tauri::State<'_, mpsc::Sender<String>>) -> Result<(), ()> {
    tx.send(payload).await.map_err(|_| ())
}

#[tauri::command]
fn is_debug_build() -> bool {
    cfg!(feature = "debug")
}
