pub mod change_matrix;
pub mod core;
pub mod evidence_contract;
#[cfg(target_os = "macos")]
mod mac_accessibility;
pub mod mac_control_maturity;
pub mod papercuts;
pub mod project_compass;
pub mod promotion;

pub mod quality;
pub mod release_boundary;
pub mod remediation;
pub mod showcase;
pub mod skill_usage_collector;
pub mod skills;

use tauri::Manager;

#[cfg(desktop)]
const WINDOW_EDGE_SNAP_THRESHOLD: i32 = 16;

#[cfg(desktop)]
fn restore_main_window(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        log::warn!("cannot restore the main window because it is not registered");
        return;
    };

    if let Err(error) = window.show() {
        log::warn!("failed to show the main window: {error}");
    }
    if let Err(error) = window.unminimize() {
        log::warn!("failed to unminimize the main window: {error}");
    }
    if let Err(error) = window.set_focus() {
        log::warn!("failed to focus the main window: {error}");
    }
}

#[cfg(desktop)]
fn snapped_window_position(
    position: tauri::PhysicalPosition<i32>,
    window_size: tauri::PhysicalSize<u32>,
    work_area: &tauri::PhysicalRect<i32, u32>,
) -> Option<tauri::PhysicalPosition<i32>> {
    fn snap_axis(value: i32, start: i32, end: i32) -> i32 {
        if value.abs_diff(start) <= WINDOW_EDGE_SNAP_THRESHOLD as u32 {
            start
        } else if value.abs_diff(end) <= WINDOW_EDGE_SNAP_THRESHOLD as u32 {
            end
        } else {
            value
        }
    }

    let mut snapped = position;
    if window_size.width <= work_area.size.width {
        let right = work_area
            .position
            .x
            .saturating_add(work_area.size.width.saturating_sub(window_size.width) as i32);
        snapped.x = snap_axis(position.x, work_area.position.x, right);
    }
    if window_size.height <= work_area.size.height {
        let bottom = work_area
            .position
            .y
            .saturating_add(work_area.size.height.saturating_sub(window_size.height) as i32);
        snapped.y = snap_axis(position.y, work_area.position.y, bottom);
    }

    (snapped != position).then_some(snapped)
}

#[cfg(desktop)]
fn snap_window_to_work_area(window: &tauri::Window, position: tauri::PhysicalPosition<i32>) {
    if window.is_fullscreen().unwrap_or(false) || window.is_maximized().unwrap_or(false) {
        return;
    }
    let (Ok(window_size), Ok(Some(monitor))) = (window.outer_size(), window.current_monitor())
    else {
        return;
    };
    if let Some(snapped) = snapped_window_position(position, window_size, monitor.work_area()) {
        let _ = window.set_position(snapped);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    // Tauri requires the single-instance plugin to be registered before every
    // other plugin so a duplicate launch exits before it can create a window.
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(
            |app, _arguments, _working_directory| restore_main_window(app),
        ));
    }

    let builder = builder
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(|window, event| {
            #[cfg(desktop)]
            if let tauri::WindowEvent::Moved(position) = event {
                snap_window_to_work_area(window, *position);
            }
        });

    #[cfg(target_os = "macos")]
    let builder = builder.on_page_load(|webview, payload| {
        if payload.event() == tauri::webview::PageLoadEvent::Finished {
            if let Err(error) = mac_accessibility::install(webview.clone()) {
                log::error!("failed to install Mac Control accessibility targets: {error}");
            }
        }
    });

    let app = builder
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            #[cfg(target_os = "macos")]
            if let Some(window) = app.get_webview_window("main") {
                for webview in window.as_ref().window().webviews() {
                    mac_accessibility::install(webview)?;
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            core::get_snapshot,
            core::get_analytics,
            core::get_skills,
            core::refresh_skills,
            core::open_skill_source,
            core::register_root,
            core::refresh,
            core::refresh_quality,
            core::refresh_repository_target_evidence,
            core::refresh_github,
            core::refresh_remediation,
            core::set_remediation_action_status,
            core::check_remediation_handoff,
            core::export_remediation,
            core::set_maturity_audit_root,
            core::open_quality_report,
            core::open_workspace,
            core::prepare_repository,
            core::preflight_action,
            core::mark_condition_expected,
            core::clear_condition_expected,
            core::update_root_settings,
            core::set_repository_lifecycle,
            core::set_repository_target_branch,
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
            promotion::get_promotion_inbox,
            promotion::decide_promotion,
            papercuts::get_papercut_backlog,
            papercuts::create_papercut,
            papercuts::set_papercut_status,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|_app_handle, _event| {
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen { .. } = _event {
            restore_main_window(_app_handle);
        }
    });
}

#[cfg(all(test, desktop))]
mod tests {
    use super::*;

    fn work_area() -> tauri::PhysicalRect<i32, u32> {
        tauri::PhysicalRect {
            position: tauri::PhysicalPosition::new(0, 24),
            size: tauri::PhysicalSize::new(1440, 876),
        }
    }

    #[test]
    fn snaps_each_window_edge_to_the_visible_work_area() {
        let size = tauri::PhysicalSize::new(700, 540);
        assert_eq!(
            snapped_window_position(tauri::PhysicalPosition::new(12, 36), size, &work_area()),
            Some(tauri::PhysicalPosition::new(0, 24))
        );
        assert_eq!(
            snapped_window_position(tauri::PhysicalPosition::new(728, 348), size, &work_area()),
            Some(tauri::PhysicalPosition::new(740, 360))
        );
    }

    #[test]
    fn leaves_a_window_away_from_edges_unchanged() {
        assert_eq!(
            snapped_window_position(
                tauri::PhysicalPosition::new(120, 140),
                tauri::PhysicalSize::new(700, 540),
                &work_area(),
            ),
            None
        );
    }
}
