mod mail;
mod notes;
mod writing;

const MAIN_WINDOW_LABEL: &str = "main";
const DEBUG_LOG_WINDOW_LABEL: &str = "debug-log-window";
const DEBUG_LOG_FILE_NAME: &str = "debug.log";
const SMALL_SCREEN_WIDTH: f64 = 1440.0;
const SMALL_SCREEN_HEIGHT: f64 = 900.0;
const LARGE_SCREEN_WIDTH: f64 = 2400.0;
const LARGE_SCREEN_HEIGHT: f64 = 1350.0;
const DEFAULT_SCREEN_RATIO: f64 = 0.80;
const SMALL_SCREEN_RATIO: f64 = 0.86;
const LARGE_SCREEN_RATIO: f64 = 0.76;
const MIN_WINDOW_WIDTH: f64 = 960.0;
const MIN_WINDOW_HEIGHT: f64 = 620.0;
const MAX_WINDOW_WIDTH: f64 = 1680.0;
const MAX_WINDOW_HEIGHT: f64 = 1050.0;
const SCREEN_MARGIN: f64 = 32.0;

#[tauri::command]
fn open_debug_devtools(_window: tauri::WebviewWindow) {
    #[cfg(debug_assertions)]
    {
        if !_window.is_devtools_open() {
            _window.open_devtools();
        }
    }
}

fn debug_log_file_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    use tauri::Manager;

    let dir = app.path().app_log_dir().map_err(|error| error.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir.join(DEBUG_LOG_FILE_NAME))
}

fn target_window_ratio(width: f64, height: f64) -> f64 {
    if width <= SMALL_SCREEN_WIDTH || height <= SMALL_SCREEN_HEIGHT {
        SMALL_SCREEN_RATIO
    } else if width >= LARGE_SCREEN_WIDTH && height >= LARGE_SCREEN_HEIGHT {
        LARGE_SCREEN_RATIO
    } else {
        DEFAULT_SCREEN_RATIO
    }
}

fn clamp_window_dimension(value: f64, min: f64, max: f64, available: f64) -> f64 {
    let upper_bound = (available - SCREEN_MARGIN).max(available * DEFAULT_SCREEN_RATIO);
    value.clamp(min.min(upper_bound), max.min(upper_bound)).round()
}

fn initial_window_size(monitor: &tauri::Monitor) -> (f64, f64) {
    let scale_factor = monitor.scale_factor();
    let work_area = monitor.work_area();
    let available_width = work_area.size.width as f64 / scale_factor;
    let available_height = work_area.size.height as f64 / scale_factor;
    let ratio = target_window_ratio(available_width, available_height);

    (
        clamp_window_dimension(
            available_width * ratio,
            MIN_WINDOW_WIDTH,
            MAX_WINDOW_WIDTH,
            available_width,
        ),
        clamp_window_dimension(
            available_height * ratio,
            MIN_WINDOW_HEIGHT,
            MAX_WINDOW_HEIGHT,
            available_height,
        ),
    )
}

fn fit_min_window_size(width: f64, height: f64) -> (f64, f64) {
    (
        MIN_WINDOW_WIDTH.min(width).round(),
        MIN_WINDOW_HEIGHT.min(height).round(),
    )
}

fn apply_initial_window_size(app: &tauri::App) -> tauri::Result<()> {
    use tauri::{LogicalSize, Manager};

    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let monitor = match window.current_monitor()? {
            Some(monitor) => Some(monitor),
            None => window.primary_monitor()?,
        };

        if let Some(monitor) = monitor {
            let (width, height) = initial_window_size(&monitor);
            let (min_width, min_height) = fit_min_window_size(width, height);

            window.set_min_size(Some(LogicalSize::new(min_width, min_height)))?;
            window.set_size(LogicalSize::new(width, height))?;
            window.center()?;
        }
    }

    Ok(())
}

#[tauri::command]
async fn open_debug_log_window(app: tauri::AppHandle) -> Result<(), String> {
    let path = debug_log_file_path(&app)?;
    if !path.exists() {
        std::fs::write(&path, "").map_err(|error| error.to_string())?;
    }

    let app_for_window = app.clone();
    let (sender, receiver) = std::sync::mpsc::channel();

    app.run_on_main_thread(move || {
        let result = (|| -> Result<(), String> {
            use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

            if let Some(window) = app_for_window.get_webview_window(DEBUG_LOG_WINDOW_LABEL) {
                window.show().map_err(|error| error.to_string())?;
                window.set_focus().map_err(|error| error.to_string())?;
                return Ok(());
            }

            let window = WebviewWindowBuilder::new(
                &app_for_window,
                DEBUG_LOG_WINDOW_LABEL,
                WebviewUrl::App("debug-log.html".into()),
            )
            .title("Debug Logs")
            .inner_size(920.0, 680.0)
            .min_inner_size(720.0, 520.0)
            .resizable(true)
            .center()
            .build()
            .map_err(|error| error.to_string())?;

            window.set_focus().map_err(|error| error.to_string())
        })();

        let _ = sender.send(result);
    })
    .map_err(|error| error.to_string())?;

    receiver
        .recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|_| "打开日志窗口超时，请重启应用后再试".to_string())?
}

#[tauri::command]
async fn close_debug_log_window(app: tauri::AppHandle) -> Result<(), String> {
    let app_for_window = app.clone();
    let (sender, receiver) = std::sync::mpsc::channel();

    app.run_on_main_thread(move || {
        let result = (|| -> Result<(), String> {
            use tauri::Manager;

            if let Some(window) = app_for_window.get_webview_window(DEBUG_LOG_WINDOW_LABEL) {
                window.close().map_err(|error| error.to_string())?;
            }

            Ok(())
        })();

        let _ = sender.send(result);
    })
    .map_err(|error| error.to_string())?;

    receiver
        .recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|_| "关闭日志窗口超时，请重启应用后再试".to_string())?
}

#[tauri::command]
fn append_debug_log(app: tauri::AppHandle, entry: serde_json::Value) -> Result<(), String> {
    use std::io::Write;

    let path = debug_log_file_path(&app)?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    let line = serde_json::to_string(&entry).map_err(|error| error.to_string())?;
    writeln!(file, "{line}").map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            apply_initial_window_size(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            open_debug_devtools,
            open_debug_log_window,
            close_debug_log_window,
            append_debug_log,
            mail::get_mail_accounts,
            mail::save_mail_account,
            mail::delete_mail_account,
            mail::fetch_mail_messages,
            mail::sync_mail_messages,
            mail::fetch_mail_detail,
            mail::send_mail,
            notes::get_note_workspace_settings,
            notes::get_note_sync_settings,
            notes::get_debug_settings,
            notes::update_note_workspace_settings,
            notes::update_note_sync_settings,
            notes::update_debug_settings,
            notes::get_backup_settings,
            notes::get_ai_providers,
            notes::update_backup_settings,
            notes::save_ai_provider,
            notes::delete_ai_provider,
            notes::get_image_hosts,
            notes::save_image_host,
            notes::delete_image_host,
            notes::list_backups,
            notes::read_backup,
            notes::restore_backup,
            notes::create_directory,
            notes::create_note,
            notes::delete_entry,
            notes::sync_notes,
            notes::list_notes,
            notes::read_note,
            notes::rename_entry,
            notes::move_entry,
            notes::save_note,
            notes::save_note_image,
            writing::list_writing_projects,
            writing::create_writing_project,
            writing::get_writing_project,
            writing::save_writing_project,
            writing::save_writing_image,
            writing::delete_writing_project,
            writing::add_writing_material,
            writing::update_writing_material,
            writing::delete_writing_material,
            writing::publish_writing_project
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
