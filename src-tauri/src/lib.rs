pub mod core;
pub mod quality;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            core::get_snapshot,
            core::register_root,
            core::refresh,
            core::refresh_github,
            core::set_maturity_audit_root,
            core::open_quality_report,
            core::open_workspace,
            core::prepare_repository,
            core::preflight_action,
            core::mark_condition_expected,
            core::clear_condition_expected,
            core::update_root_settings,
            core::set_repository_lifecycle,
            core::set_release_rule,
            core::set_release_recipe,
            core::set_release_version,
            core::set_ai_permission,
            core::preview_ai_summary,
            core::set_retention_days,
            core::upsert_product,
            core::delete_product,
            core::upsert_group,
            core::delete_group,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
