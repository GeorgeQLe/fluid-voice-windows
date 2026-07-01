mod audio;
mod commands;
mod enhancement;
mod history;
mod hotkey;
mod model_catalog;
mod secrets;
mod settings;
mod state;
mod text_insertion;
mod transcription;

use commands::{
    cancel_dictation, cancel_model_download, clear_history, clear_model_cache, delete_api_key,
    download_model, finish_dictation, get_app_status, get_recording_snapshot, get_settings,
    has_api_key, insert_text, list_history, list_input_devices, list_models, save_settings,
    set_api_key, start_dictation, validate_model_cache,
};
use state::AppState;
use std::path::PathBuf;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_app_status,
            get_settings,
            save_settings,
            list_input_devices,
            start_dictation,
            get_recording_snapshot,
            cancel_dictation,
            finish_dictation,
            list_models,
            validate_model_cache,
            download_model,
            cancel_model_download,
            clear_model_cache,
            insert_text,
            list_history,
            clear_history,
            set_api_key,
            has_api_key,
            delete_api_key
        ])
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| fallback_data_dir());
            let state =
                AppState::new(data_dir, app.handle().clone()).map_err(anyhow::Error::msg)?;
            app.manage(state);
            setup_tray(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running FluidVoice");
}

fn setup_tray(app: &mut tauri::App) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "Show FluidVoice", true, None::<&str>)?;
    let hide_item = MenuItem::with_id(app, "hide", "Hide", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show_item, &hide_item, &separator, &quit_item])?;

    let builder = TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "hide" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    let builder = if let Some(icon) = app.default_window_icon() {
        builder.icon(icon.clone())
    } else {
        builder
    };

    builder.build(app)?;
    Ok(())
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn fallback_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("FluidVoice")
}
