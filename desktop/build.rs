fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "status",
            "search",
            "lookup",
            "get_settings",
            "save_settings",
            "choose_folder",
        ]),
    ))
    .expect("failed to generate Tauri application permissions");
}
