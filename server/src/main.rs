use axum::{
    extract::{Multipart, Path, Query, State},
    http::HeaderMap,
    response::Json,
    routing::{get, post},
    Router,
};
use chrono::Utc;
use config::{Config, File};
use rand::{distributions::Alphanumeric, Rng};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};
use uuid::Uuid;

const CONFIG_FILE: &str = "config.toml";
const DB_FILE: &str = "files.db";
const DEFAULT_ROOT_DIR: &str = "./files";
const DEFAULT_PORT: u16 = 3000;
const MAX_STORAGE_FILE_NAME_BYTES: usize = 180;

fn default_root_dir() -> String {
    DEFAULT_ROOT_DIR.to_string()
}

fn default_port() -> u16 {
    DEFAULT_PORT
}

fn validate_storage_file_name(value: &str) -> Result<String, String> {
    let file_name = value.trim();
    if file_name.is_empty() {
        return Err("File name is empty".to_string());
    }

    if file_name.len() > MAX_STORAGE_FILE_NAME_BYTES {
        return Err(format!("File name too long: {}", file_name));
    }

    if file_name.chars().any(|ch| matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || ch.is_control()) {
        return Err(format!("File name contains invalid characters: {}", file_name));
    }

    if file_name == "." || file_name == ".." || file_name.ends_with('.') {
        return Err(format!("File name is invalid: {}", file_name));
    }

    Ok(file_name.to_string())
}

fn validate_relative_path(value: &str) -> Result<String, String> {
    let relative_path = value.trim().trim_start_matches("./");
    if relative_path.is_empty() || relative_path.starts_with('/') || relative_path.contains("..") {
        return Err("Relative path is invalid".to_string());
    }

    for component in relative_path.split('/') {
        validate_storage_file_name(component)?;
    }

    Ok(relative_path.to_string())
}

#[derive(Clone)]
struct AppState {
    root_dir: PathBuf,
    db: Arc<Mutex<Connection>>,
    users: Arc<Vec<UserConfig>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct User {
    id: String,
    name: String,
    storage_path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct FileInfo {
    id: String,
    user_id: String,
    original_name: String,
    stored_name: String,
    relative_path: String,
    size: u64,
    mime_type: Option<String>,
    last_updated: String,
    version: u32,
    is_deleted: bool,
    deleted_at: Option<String>,
}

#[derive(Serialize)]
struct ApiResponse<T: Serialize> {
    success: bool,
    data: Option<T>,
    message: String,
}

#[derive(Deserialize)]
struct FileListQuery {
    #[allow(dead_code)]
    page: Option<u32>,
    #[allow(dead_code)]
    limit: Option<u32>,
}

#[derive(Deserialize)]
struct CreateUserRequest {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    api_key: Option<String>,
    #[allow(dead_code)]
    storage_path: Option<String>,
}

#[derive(Deserialize, Clone)]
struct UserConfig {
    name: String,
    api_key: String,
    storage_path: String,
}

#[derive(Deserialize)]
struct ServerConfig {
    #[serde(default = "default_root_dir")]
    root_dir: String,
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default)]
    users: Vec<UserConfig>,
}

fn generate_api_key() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

fn load_or_create_config() -> ServerConfig {
    let config_path = PathBuf::from(CONFIG_FILE);

    if !config_path.exists() {
        let admin_key = generate_api_key();
        let content = format!(
            r#"[server]
# 文件根目录，所有用户的文件都存放在此目录下
root_dir = "{}"
# 服务监听端口
port = {}

# 用户配置数组，必须命名为 server.users 才能被正确解析
[[server.users]]
name = "admin"
api_key = "{}"
storage_path = "admin"

[[server.users]]
name = "user1"
api_key = "{}"
storage_path = "user1_files"
"#,
            DEFAULT_ROOT_DIR,
            DEFAULT_PORT,
            admin_key,
            generate_api_key()
        );

        std::fs::write(&config_path, content).expect("Failed to create config file");
        info!("Created default config file with multi-user setup");
    }

    let config = Config::builder()
        .add_source(File::from(config_path.clone()))
        .build()
        .expect("Failed to load config");

    let server: ServerConfig = config
        .get::<ServerConfig>("server")
        .expect("Failed to parse server config");

    if server.users.is_empty() {
        panic!("At least one user must be configured in config.toml");
    }

    server
}

fn init_db(db_path: &PathBuf) -> Connection {
    let conn = Connection::open(db_path).expect("Failed to open database");

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            storage_path TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS files (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            original_name TEXT NOT NULL,
            stored_name TEXT NOT NULL,
            relative_path TEXT NOT NULL,
            size INTEGER NOT NULL,
            mime_type TEXT,
            last_updated TEXT NOT NULL,
            version INTEGER NOT NULL DEFAULT 1,
            is_deleted INTEGER NOT NULL DEFAULT 0,
            deleted_at TEXT,
            FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_files_user_id ON files(user_id);
        ",
    )
    .expect("Failed to create tables");

    remove_api_keys_from_users_table(&conn);

    let existing_columns: Vec<String> = {
        let mut stmt = conn
            .prepare("PRAGMA table_info(files)")
            .expect("Failed to inspect files table");
        stmt.query_map([], |row| row.get::<_, String>(1))
            .expect("Failed to query files table columns")
            .filter_map(|item| item.ok())
            .collect()
    };

    if !existing_columns.iter().any(|column| column == "is_deleted") {
        conn.execute(
            "ALTER TABLE files ADD COLUMN is_deleted INTEGER NOT NULL DEFAULT 0",
            [],
        )
        .expect("Failed to add is_deleted column");
    }

    if !existing_columns.iter().any(|column| column == "deleted_at") {
        conn.execute("ALTER TABLE files ADD COLUMN deleted_at TEXT", [])
            .expect("Failed to add deleted_at column");
    }

    info!("Database initialized at {:?}", db_path);
    conn
}

fn remove_api_keys_from_users_table(conn: &Connection) {
    let user_columns: Vec<String> = {
        let mut stmt = conn
            .prepare("PRAGMA table_info(users)")
            .expect("Failed to inspect users table");
        stmt.query_map([], |row| row.get::<_, String>(1))
            .expect("Failed to query users table columns")
            .filter_map(|item| item.ok())
            .collect()
    };

    if !user_columns.iter().any(|column| column == "api_key") {
        return;
    }

    conn.execute_batch(
        "
        PRAGMA foreign_keys = OFF;
        DROP INDEX IF EXISTS idx_users_api_key;
        CREATE TABLE users_without_api_key (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            storage_path TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        INSERT INTO users_without_api_key (id, name, storage_path, created_at)
            SELECT id, name, storage_path, created_at FROM users;
        DROP TABLE users;
        ALTER TABLE users_without_api_key RENAME TO users;
        PRAGMA foreign_keys = ON;
        ",
    )
    .expect("Failed to remove api_key from users table");

    info!("Removed api_key column from users table; API keys are now loaded from config.toml only");
}

fn sync_users_from_config(conn: &Connection, users: &[UserConfig], root_dir: &PathBuf) {
    let now = Utc::now().to_rfc3339();

    for user_config in users {
        let user_id = format!("{}", user_config.name);

        match conn.query_row(
            "SELECT id FROM users WHERE name = ?1",
            params![user_config.name],
            |row| row.get::<_, String>(0),
        ) {
            Ok(_) => {
                conn.execute(
                    "UPDATE users SET storage_path = ?1 WHERE name = ?2",
                    params![user_config.storage_path, user_config.name],
                )
                .ok();
                let user_dir = root_dir.join(&user_config.storage_path);
                if !user_dir.exists() {
                    fs::create_dir_all(&user_dir)
                        .expect(&format!("Failed to create user directory: {:?}", user_dir));
                }
                info!("Updated user config for: {}", user_config.name);
            }
            Err(_) => {
                // 用户不存在，创建新用户
                conn.execute(
                    "INSERT INTO users (id, name, storage_path, created_at) VALUES (?1, ?2, ?3, ?4)",
                    params![user_id, user_config.name, user_config.storage_path, now],
                ).ok();

                // 创建用户专属目录
                let user_dir = root_dir.join(&user_config.storage_path);
                if !user_dir.exists() {
                    fs::create_dir_all(&user_dir)
                        .expect(&format!("Failed to create user directory: {:?}", user_dir));
                }
                info!(
                    "Created user '{}' with storage path: {}",
                    user_config.name, user_config.storage_path
                );
            }
        }
    }
}

fn authenticate_user(users: &[UserConfig], api_key: &str) -> Option<User> {
    users
        .iter()
        .find(|user| user.api_key == api_key)
        .map(|user| User {
            id: user.name.clone(),
            name: user.name.clone(),
            storage_path: user.storage_path.clone(),
        })
}

fn get_user_storage_dir(root_dir: &PathBuf, storage_path: &str) -> PathBuf {
    root_dir.join(storage_path)
}

fn get_api_key(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

async fn get_user_from_key(state: &AppState, api_key: &str) -> Option<User> {
    authenticate_user(&state.users, api_key)
}

async fn upload_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Json<ApiResponse<FileInfo>> {
    let api_key = match get_api_key(&headers) {
        Some(key) => key,
        None => {
            return Json(ApiResponse {
                success: false,
                data: None,
                message: "Missing API key".to_string(),
            })
        }
    };

    let user = match get_user_from_key(&state, &api_key).await {
        Some(u) => u,
        None => {
            return Json(ApiResponse {
                success: false,
                data: None,
                message: "Invalid API key".to_string(),
            })
        }
    };

    let storage_dir = get_user_storage_dir(&state.root_dir, &user.storage_path);
    if let Err(e) = tokio::fs::create_dir_all(&storage_dir).await {
        error!("Failed to create user storage directory: {}", e);
        return Json(ApiResponse {
            success: false,
            data: None,
            message: format!("Failed to prepare user storage directory: {}", e),
        });
    }

    let mut relative_path_override: Option<String> = None;
    let mut last_updated_override: Option<String> = None;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();
        if name == "relative_path" {
            if let Ok(value) = field.text().await {
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    match validate_relative_path(trimmed) {
                        Ok(relative_path) => relative_path_override = Some(relative_path),
                        Err(message) => {
                            return Json(ApiResponse {
                                success: false,
                                data: None,
                                message,
                            });
                        }
                    }
                }
            }
            continue;
        }
        if name == "last_updated" {
            if let Ok(value) = field.text().await {
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    last_updated_override = Some(trimmed.to_string());
                }
            }
            continue;
        }

        if name != "file" {
            continue;
        }

        let file_name = match validate_storage_file_name(field.file_name().unwrap_or("unknown")) {
            Ok(file_name) => file_name,
            Err(message) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    message,
                });
            }
        };
        let content_type = field.content_type().map(|s| s.to_string());

        let data = match field.bytes().await {
            Ok(data) => data,
            Err(e) => {
                error!("Failed to read field: {}", e);
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    message: "Failed to read file data".to_string(),
                });
            }
        };

        let file_size = data.len() as u64;
        let relative_path = relative_path_override.clone().unwrap_or_else(|| file_name.clone());
        let stored_name = format!("{}/{}", user.storage_path, relative_path);
        let file_path = state.root_dir.join(&stored_name);
        if let Some(parent) = file_path.parent() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                error!("Failed to create file directory: {}", e);
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    message: format!("Failed to prepare file directory: {}", e),
                });
            }
        }

        if let Err(e) = tokio::fs::write(&file_path, &data).await {
            error!("Failed to write file: {}", e);
            return Json(ApiResponse {
                success: false,
                data: None,
                message: format!("Failed to save file: {}", e),
            });
        }

        let file_id = Uuid::new_v4().to_string();
        let now = last_updated_override
            .clone()
            .unwrap_or_else(|| Utc::now().to_rfc3339());
        let file_info = FileInfo {
            id: file_id.clone(),
            user_id: user.id.clone(),
            original_name: file_name.clone(),
            stored_name: stored_name.clone(),
            relative_path: relative_path.clone(),
            size: file_size,
            mime_type: content_type.clone(),
            last_updated: now.clone(),
            version: 1,
            is_deleted: false,
            deleted_at: None,
        };

        let db = state.db.clone();
        let user_id = user.id.clone();
        let file_id_clone = file_id.clone();
        let file_name_clone = file_name.clone();
        let stored_name_clone = stored_name.clone();
        let relative_path_clone = relative_path.clone();
        let mime_type_clone = content_type.clone();
        let now_clone = now.clone();

        tokio::task::spawn_blocking(move || {
            let conn = db.blocking_lock();
            let _ = conn.execute(
                "INSERT INTO files (id, user_id, original_name, stored_name, relative_path, size, mime_type, last_updated, version, is_deleted, deleted_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, NULL)",
                params![file_id_clone, user_id, file_name_clone, stored_name_clone, relative_path_clone, file_size, mime_type_clone, now_clone, 1u32],
            );
        }).await.unwrap();

        info!(
            "User {} uploaded file: {} ({})",
            user.name, file_name, file_size
        );

        return Json(ApiResponse {
            success: true,
            data: Some(file_info),
            message: "File uploaded successfully".to_string(),
        });
    }

    Json(ApiResponse {
        success: false,
        data: None,
        message: "No file field found in multipart form".to_string(),
    })
}

async fn download_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(file_id): axum::extract::Path<String>,
) -> Json<ApiResponse<Vec<u8>>> {
    let api_key = match get_api_key(&headers) {
        Some(key) => key,
        None => {
            return Json(ApiResponse {
                success: false,
                data: None,
                message: "Missing API key".to_string(),
            })
        }
    };

    let user = match get_user_from_key(&state, &api_key).await {
        Some(u) => u,
        None => {
            return Json(ApiResponse {
                success: false,
                data: None,
                message: "Invalid API key".to_string(),
            })
        }
    };

    let db = state.db.clone();
    let file_id_clone = file_id.clone();
    let user_id_clone = user.id.clone();

    let res = tokio::task::spawn_blocking(move || {
        let conn = db.blocking_lock();
        let mut stmt = conn
            .prepare("SELECT stored_name FROM files WHERE id = ?1 AND user_id = ?2 AND is_deleted = 0")
            .map_err(|e| e.to_string())?;
        let stored_name: String = stmt
            .query_row(params![file_id_clone, user_id_clone], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        Ok::<_, String>(stored_name)
    })
    .await;

    match res {
        Ok(Ok(stored_name)) => {
            let file_path = state.root_dir.join(&stored_name);
            match tokio::fs::read(&file_path).await {
                Ok(data) => Json(ApiResponse {
                    success: true,
                    data: Some(data),
                    message: "File downloaded".to_string(),
                }),
                Err(_) => Json(ApiResponse {
                    success: false,
                    data: None,
                    message: "File read error".to_string(),
                }),
            }
        }
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: "File not found or access denied".to_string(),
        }),
    }
}

async fn list_files(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(_query): Query<FileListQuery>,
) -> Json<ApiResponse<Vec<FileInfo>>> {
    let api_key = match get_api_key(&headers) {
        Some(key) => key,
        None => {
            return Json(ApiResponse {
                success: false,
                data: None,
                message: "Missing API key".to_string(),
            })
        }
    };

    let user = match get_user_from_key(&state, &api_key).await {
        Some(u) => u,
        None => {
            return Json(ApiResponse {
                success: false,
                data: None,
                message: "Invalid API key".to_string(),
            })
        }
    };

    let db = state.db.clone();
    let user_id = user.id.clone();
    let result = tokio::task::spawn_blocking(move || {
        let conn = db.blocking_lock();
        let mut stmt = conn.prepare(
            "SELECT id, user_id, original_name, stored_name, relative_path, size, mime_type, last_updated, version, is_deleted, deleted_at FROM files WHERE user_id = ?1",
        ).map_err(|e| e.to_string())?;

        let files: Vec<FileInfo> = stmt.query_map(params![user_id], |row| {
            Ok(FileInfo {
                id: row.get(0)?,
                user_id: row.get(1)?,
                original_name: row.get(2)?,
                stored_name: row.get(3)?,
                relative_path: row.get(4)?,
                size: row.get(5)?,
                mime_type: row.get(6)?,
                last_updated: row.get(7)?,
                version: row.get(8)?,
                is_deleted: row.get::<_, i64>(9)? != 0,
                deleted_at: row.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

        Ok::<_, String>(files)
    }).await;

    match result {
        Ok(Ok(files)) => {
            let count = files.len();
            Json(ApiResponse {
                success: true,
                data: Some(files),
                message: format!("Found {} files", count),
            })
        }
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: "Failed to list files".to_string(),
        }),
    }
}

async fn update_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(file_id): Path<String>,
    mut multipart: Multipart,
) -> Json<ApiResponse<FileInfo>> {
    let api_key = match get_api_key(&headers) {
        Some(key) => key,
        None => {
            return Json(ApiResponse {
                success: false,
                data: None,
                message: "Missing API key".to_string(),
            })
        }
    };

    let user = match get_user_from_key(&state, &api_key).await {
        Some(u) => u,
        None => {
            return Json(ApiResponse {
                success: false,
                data: None,
                message: "Invalid API key".to_string(),
            })
        }
    };

    let storage_dir = get_user_storage_dir(&state.root_dir, &user.storage_path);
    if let Err(e) = tokio::fs::create_dir_all(&storage_dir).await {
        error!("Failed to create user storage directory: {}", e);
        return Json(ApiResponse {
            success: false,
            data: None,
            message: format!("Failed to prepare user storage directory: {}", e),
        });
    }
    let db = state.db.clone();
    let user_id = user.id.clone();
    let file_id_clone = file_id.clone();

    let stored_file_result: Option<(String, String, String)> = tokio::task::spawn_blocking(move || {
        let conn = db.blocking_lock();
        conn.query_row(
            "SELECT stored_name, original_name, relative_path FROM files WHERE id = ?1 AND user_id = ?2 AND is_deleted = 0",
            params![file_id_clone, user_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .ok()
    })
    .await
    .unwrap();

    let (stored_name, original_name, previous_relative_path) = match stored_file_result {
        Some(file) => file,
        None => {
            return Json(ApiResponse {
                success: false,
                data: None,
                message: "File not found or access denied".to_string(),
            });
        }
    };

    let mut relative_path_override: Option<String> = None;
    let mut last_updated_override: Option<String> = None;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();
        if name == "relative_path" {
            if let Ok(value) = field.text().await {
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    match validate_relative_path(trimmed) {
                        Ok(relative_path) => relative_path_override = Some(relative_path),
                        Err(message) => {
                            return Json(ApiResponse {
                                success: false,
                                data: None,
                                message,
                            });
                        }
                    }
                }
            }
            continue;
        }
        if name == "last_updated" {
            if let Ok(value) = field.text().await {
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    last_updated_override = Some(trimmed.to_string());
                }
            }
            continue;
        }

        if name != "file" {
            continue;
        }

        let file_name = match validate_storage_file_name(field.file_name().unwrap_or(&original_name)) {
            Ok(file_name) => file_name,
            Err(message) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    message,
                });
            }
        };
        let content_type = field.content_type().map(|s| s.to_string());

        let data = match field.bytes().await {
            Ok(data) => data,
            Err(e) => {
                error!("Failed to read field: {}", e);
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    message: "Failed to read file data".to_string(),
                });
            }
        };

        let relative_path = relative_path_override
            .clone()
            .unwrap_or_else(|| previous_relative_path.clone());
        let new_stored_name = format!("{}/{}", user.storage_path, relative_path);
        let file_path = state.root_dir.join(&new_stored_name);
        if stored_name != new_stored_name {
            let old_path = state.root_dir.join(&stored_name);
            let _ = tokio::fs::remove_file(&old_path).await;
        }
        if let Some(parent) = file_path.parent() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                error!("Failed to create file directory: {}", e);
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    message: format!("Failed to prepare file directory: {}", e),
                });
            }
        }

        if let Err(e) = tokio::fs::write(&file_path, &data).await {
            error!("Failed to write file: {}", e);
            return Json(ApiResponse {
                success: false,
                data: None,
                message: format!("Failed to save file: {}", e),
            });
        }

        let now = last_updated_override
            .clone()
            .unwrap_or_else(|| Utc::now().to_rfc3339());
        let file_size = data.len() as u64;

        let db_clone = state.db.clone();
        let file_id_for_update = file_id.clone();
        let user_id_for_update = user.id.clone();

        let _ = tokio::task::spawn_blocking(move || {
            let conn = db_clone.blocking_lock();
            let _ = conn.execute(
                "UPDATE files SET original_name = ?8, stored_name = ?1, relative_path = ?2, size = ?3, mime_type = ?4, last_updated = ?5, version = version + 1, is_deleted = 0, deleted_at = NULL WHERE id = ?6 AND user_id = ?7",
                params![new_stored_name, relative_path, file_size, content_type, now, file_id_for_update, user_id_for_update, file_name],
            );
        }).await;

        let db_for_select = state.db.clone();
        let file_id_for_select = file_id.clone();
        let user_id_for_select = user.id.clone();

        let updated_info = tokio::task::spawn_blocking(move || {
            let conn = db_for_select.blocking_lock();
            conn.query_row(
                "SELECT id, user_id, original_name, stored_name, relative_path, size, mime_type, last_updated, version, is_deleted, deleted_at FROM files WHERE id = ?1 AND user_id = ?2",
                params![file_id_for_select, user_id_for_select],
                |row| {
                    Ok(FileInfo {
                        id: row.get(0)?,
                        user_id: row.get(1)?,
                        original_name: row.get(2)?,
                        stored_name: row.get(3)?,
                        relative_path: row.get(4)?,
                        size: row.get(5)?,
                        mime_type: row.get(6)?,
                        last_updated: row.get(7)?,
                        version: row.get(8)?,
                        is_deleted: row.get::<_, i64>(9)? != 0,
                        deleted_at: row.get(10)?,
                    })
                },
            ).ok()
        }).await.unwrap();

        match updated_info {
            Some(info) => {
                info!(
                    "User {} updated file: {} (version {})",
                    user.name, file_id, info.version
                );
                return Json(ApiResponse {
                    success: true,
                    data: Some(info),
                    message: "File updated successfully".to_string(),
                });
            }
            None => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    message: "Failed to fetch updated record".to_string(),
                })
            }
        }
    }

    Json(ApiResponse {
        success: false,
        data: None,
        message: "No file field found in multipart form".to_string(),
    })
}

async fn delete_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(file_id): Path<String>,
) -> Json<ApiResponse<()>> {
    let api_key = match get_api_key(&headers) {
        Some(key) => key,
        None => {
            return Json(ApiResponse {
                success: false,
                data: None,
                message: "Missing API key".to_string(),
            })
        }
    };

    let user = match get_user_from_key(&state, &api_key).await {
        Some(u) => u,
        None => {
            return Json(ApiResponse {
                success: false,
                data: None,
                message: "Invalid API key".to_string(),
            })
        }
    };

    let db = state.db.clone();
    let file_id_clone = file_id.clone();
    let user_id_clone = user.id.clone();

    let file_record: Option<(String, String, String)> = tokio::task::spawn_blocking(move || {
        let conn = db.blocking_lock();
        conn.query_row(
            "SELECT stored_name, relative_path, original_name FROM files WHERE id = ?1 AND user_id = ?2",
            params![file_id_clone, user_id_clone],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .ok()
    })
    .await
    .unwrap();

    match file_record {
        Some((name, _relative_path, original_name)) => {
            let file_path = state.root_dir.join(&name);
            if let Err(e) = tokio::fs::remove_file(&file_path).await {
                error!("Failed to delete file from disk: {}", e);
            }

            let db_clone = state.db.clone();
            let file_id_for_delete = file_id.clone();
            let user_id_for_delete = user.id.clone();
            let now = Utc::now().to_rfc3339();
            let _ = tokio::task::spawn_blocking(move || {
                let conn = db_clone.blocking_lock();
                let _ = conn.execute(
                    "UPDATE files SET is_deleted = 1, deleted_at = ?1, last_updated = ?1, size = 0, version = version + 1 WHERE id = ?2 AND user_id = ?3",
                    params![now, file_id_for_delete, user_id_for_delete],
                );
            })
            .await;

            info!("User {} marked file deleted: {}", user.name, original_name);
        }
        None => {
            return Json(ApiResponse {
                success: false,
                data: None,
                message: "File not found or access denied".to_string(),
            });
        }
    }

    Json(ApiResponse {
        success: true,
        data: None,
        message: "File deleted successfully".to_string(),
    })
}

async fn create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(_req): Json<CreateUserRequest>,
) -> Json<ApiResponse<User>> {
    let api_key = match get_api_key(&headers) {
        Some(key) => key,
        None => {
            return Json(ApiResponse {
                success: false,
                data: None,
                message: "Missing API key".to_string(),
            })
        }
    };

    let is_admin = get_user_from_key(&state, &api_key)
        .await
        .map(|u| u.name == "admin" || u.id == "admin")
        .unwrap_or(false);

    if !is_admin {
        return Json(ApiResponse {
            success: false,
            data: None,
            message: "Only admin can create users".to_string(),
        });
    }

    Json(ApiResponse {
        success: false,
        data: None,
        message: "Users and API keys are managed in config.toml; edit [[server.users]] and restart the server".to_string(),
    })
}

async fn list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<ApiResponse<Vec<User>>> {
    let api_key = match get_api_key(&headers) {
        Some(key) => key,
        None => {
            return Json(ApiResponse {
                success: false,
                data: None,
                message: "Missing API key".to_string(),
            })
        }
    };

    let is_admin = get_user_from_key(&state, &api_key)
        .await
        .map(|u| u.name == "admin" || u.id == "admin")
        .unwrap_or(false);

    if !is_admin {
        return Json(ApiResponse {
            success: false,
            data: None,
            message: "Only admin can list users".to_string(),
        });
    }

    let users = state
        .users
        .iter()
        .map(|user| User {
            id: user.name.clone(),
            name: user.name.clone(),
            storage_path: user.storage_path.clone(),
        })
        .collect();

    Json(ApiResponse {
        success: true,
        data: Some(users),
        message: "Users listed from config.toml".to_string(),
    })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    let server_config = load_or_create_config();

    let root_dir = PathBuf::from(&server_config.root_dir);
    if !root_dir.exists() {
        tokio::fs::create_dir_all(&root_dir).await.unwrap();
    }

    let db_path = PathBuf::from(DB_FILE);
    let conn = init_db(&db_path);

    // 从配置文件同步用户到数据库，并创建对应的文件夹
    sync_users_from_config(&conn, &server_config.users, &root_dir);

    info!("Root directory: {:?}", root_dir);
    info!("Server port: {}", server_config.port);
    info!("Configured users: {}", server_config.users.len());

    let state = AppState {
        root_dir,
        db: Arc::new(Mutex::new(conn)),
        users: Arc::new(server_config.users.clone()),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/users/create", post(create_user))
        .route("/api/users/list", get(list_users))
        .route("/api/files/upload", post(upload_file))
        .route("/api/files/download/{id}", get(download_file))
        .route("/api/files/list", get(list_files))
        .route("/api/files/update/{id}", post(update_file))
        .route("/api/files/delete/{id}", post(delete_file))
        .layer(cors)
        .with_state(state);

    let addr = format!("0.0.0.0:{}", server_config.port);
    info!("Server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
