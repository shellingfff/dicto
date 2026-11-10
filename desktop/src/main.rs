#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod backend;
mod document;

use tauri::Manager;

fn show(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show(app)))
        .plugin(tauri_plugin_dialog::init())
        .manage(backend::Backend::default())
        .invoke_handler(tauri::generate_handler![
            backend::status,
            backend::search,
            backend::lookup,
            backend::get_settings,
            backend::save_settings,
            backend::choose_folder
        ])
        .register_asynchronous_uri_scheme_protocol("dict", |context, request, responder| {
            let app = context.app_handle().clone();
            tauri::async_runtime::spawn_blocking(move || {
                responder.respond(document::respond(
                    app.state::<backend::Backend>().inner(),
                    request.uri(),
                ));
            });
        })
        .setup(|app| {
            use tauri::{
                menu::{Menu, MenuItem},
                tray::TrayIconBuilder,
            };
            let show_item = MenuItem::with_id(app, "show", "Show Dictionary", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Dicto")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            backend::refresh(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("could not start Dicto");
}
