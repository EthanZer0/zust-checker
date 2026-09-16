//! ZUST Checker — Tauri 后端入口

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;

pub use commands::*;
pub use state::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    let app_state = state::AppState::new();
    let sniper_state = state::SniperState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .manage(app_state)
        .manage(sniper_state)
        .invoke_handler(tauri::generate_handler![
            commands::check_session,
            commands::login,
            commands::refresh_captcha,
            commands::send_reauth_code,
            commands::verify_reauth_code,
            commands::logout,
            commands::refresh_dashboard,
            commands::get_grades,
            commands::get_schedule,
            commands::get_exams,
            commands::get_course_tabs,
            commands::get_course_list,
            commands::get_course_detail,
            commands::get_selected_courses,
            commands::enroll_single,
            commands::sniper_start,
            commands::sniper_stop,
            commands::sniper_status,
            commands::sniper_tick,
            commands::load_credentials,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
