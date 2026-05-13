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

#[derive(Clone)]
struct AppState {
    root_dir: PathBuf,
    db: Arc<Mutex<Connection>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct User {
    id: String,
    name: String,
    api_key: String,
    storage_path: String,
    created_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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
    name: String,
    api_key: Option<String>,
    storage_path: Option<String>,
}

#[derive(Deserialize)]
struct UserConfig {
    name: String,
    api_key: String,
    storage_path: String,
}

#[derive(Deserialize)]
struct ServerConfig {
    root_dir: String,
    port: u16,
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
root_dir = "./files"
# 服务监听端口
port = 3000

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
            api_key TEXT UNIQUE NOT NULL,
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
            FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_files_user_id ON files(user_id);
        CREATE INDEX IF NOT EXISTS idx_users_api_key ON users(api_key);
        ",
    )
    .expect("Failed to create tables");

    info!("Database initialized at {:?}", db_path);
    conn
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
                // 用户已存在，更新 api_key 和 storage_path
                conn.execute(
                    "UPDATE users SET api_key = ?1, storage_path = ?2 WHERE name = ?3",
                    params![
                        user_config.api_key,
                        user_config.storage_path,
                        user_config.name
                    ],
                )
                .ok();
                info!("Updated user config for: {}", user_config.name);
            }
            Err(_) => {
                // 用户不存在，创建新用户
                conn.execute(
                    "INSERT INTO users (id, name, api_key, storage_path, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![user_id, user_config.name, user_config.api_key, user_config.storage_path, now],
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

fn authenticate_user(conn: &Connection, api_key: &str) -> Option<User> {
    conn.query_row(
        "SELECT id, name, api_key, storage_path, created_at FROM users WHERE api_key = ?1",
        params![api_key],
        |row| {
            Ok(User {
                id: row.get(0)?,
                name: row.get(1)?,
                api_key: row.get(2)?,
                storage_path: row.get(3)?,
                created_at: row.get(4)?,
            })
        },
    )
    .ok()
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
    let db = state.db.clone();
    let api_key_clone = api_key.to_string();
    tokio::task::spawn_blocking(move || {
        let conn = db.blocking_lock();
        authenticate_user(&conn, &api_key_clone)
    })
    .await
    .unwrap()
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

    let mut relative_path_override: Option<String> = None;
    let mut last_updated_override: Option<String> = None;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();
        if name == "relative_path" {
            if let Ok(value) = field.text().await {
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    relative_path_override = Some(trimmed.replace('\\', "/"));
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

        let file_name = field.file_name().unwrap_or("unknown").to_string();
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
        let file_uuid = Uuid::new_v4();
        let stored_name = format!("{}/{}_{}", user.storage_path, file_uuid, file_name);
        let file_path = state.root_dir.join(&stored_name);

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
        let relative_path = relative_path_override
            .clone()
            .unwrap_or_else(|| stored_name.clone());

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
                "INSERT INTO files (id, user_id, original_name, stored_name, relative_path, size, mime_type, last_updated, version) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
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

    let storage_dir = get_user_storage_dir(&state.root_dir, &user.storage_path);
    let db = state.db.clone();
    let file_id_clone = file_id.clone();
    let user_id_clone = user.id.clone();

    let res = tokio::task::spawn_blocking(move || {
        let conn = db.blocking_lock();
        let mut stmt = conn
            .prepare("SELECT stored_name FROM files WHERE id = ?1 AND user_id = ?2")
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
            "SELECT id, user_id, original_name, stored_name, relative_path, size, mime_type, last_updated, version FROM files WHERE user_id = ?1",
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
    let db = state.db.clone();
    let user_id = user.id.clone();
    let file_id_clone = file_id.clone();

    let stored_name_result: Option<String> = tokio::task::spawn_blocking(move || {
        let conn = db.blocking_lock();
        conn.query_row(
            "SELECT stored_name FROM files WHERE id = ?1 AND user_id = ?2",
            params![file_id_clone, user_id],
            |row| row.get::<_, String>(0),
        )
        .ok()
    })
    .await
    .unwrap();

    let stored_name = match stored_name_result {
        Some(name) => name,
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
                    relative_path_override = Some(trimmed.replace('\\', "/"));
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

        let old_path = state.root_dir.join(&stored_name);
        let _ = tokio::fs::remove_file(&old_path).await;

        let file_uuid = Uuid::new_v4();
        let new_stored_name = format!(
            "{}/{}_{}",
            user.storage_path,
            file_uuid,
            stored_name.split('/').last().unwrap_or(&stored_name)
        );
        let file_path = state.root_dir.join(&new_stored_name);

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
        let relative_path = relative_path_override
            .clone()
            .unwrap_or_else(|| new_stored_name.clone());
        let file_size = data.len() as u64;

        let db_clone = state.db.clone();
        let file_id_for_update = file_id.clone();
        let user_id_for_update = user.id.clone();

        let _ = tokio::task::spawn_blocking(move || {
            let conn = db_clone.blocking_lock();
            let _ = conn.execute(
                "UPDATE files SET stored_name = ?1, relative_path = ?2, size = ?3, mime_type = ?4, last_updated = ?5, version = version + 1 WHERE id = ?6 AND user_id = ?7",
                params![new_stored_name, relative_path, file_size, content_type, now, file_id_for_update, user_id_for_update],
            );
        }).await;

        let db_for_select = state.db.clone();
        let file_id_for_select = file_id.clone();
        let user_id_for_select = user.id.clone();

        let updated_info = tokio::task::spawn_blocking(move || {
            let conn = db_for_select.blocking_lock();
            conn.query_row(
                "SELECT id, user_id, original_name, stored_name, relative_path, size, mime_type, last_updated, version FROM files WHERE id = ?1 AND user_id = ?2",
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

    let storage_dir = get_user_storage_dir(&state.root_dir, &user.storage_path);
    let db = state.db.clone();
    let file_id_clone = file_id.clone();
    let user_id_clone = user.id.clone();

    let stored_name: Option<String> = tokio::task::spawn_blocking(move || {
        let conn = db.blocking_lock();
        conn.query_row(
            "SELECT stored_name FROM files WHERE id = ?1 AND user_id = ?2",
            params![file_id_clone, user_id_clone],
            |row| row.get(0),
        )
        .ok()
    })
    .await
    .unwrap();

    match stored_name {
        Some(name) => {
            let file_path = state.root_dir.join(&name);
            if let Err(e) = tokio::fs::remove_file(&file_path).await {
                error!("Failed to delete file from disk: {}", e);
            }

            let db_clone = state.db.clone();
            let file_id_for_delete = file_id.clone();
            let user_id_for_delete = user.id.clone();
            let _ = tokio::task::spawn_blocking(move || {
                let conn = db_clone.blocking_lock();
                let _ = conn.execute(
                    "DELETE FROM files WHERE id = ?1 AND user_id = ?2",
                    params![file_id_for_delete, user_id_for_delete],
                );
            })
            .await;

            info!("User {} deleted file: {}", user.name, name);
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
    Json(req): Json<CreateUserRequest>,
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

    let user_api_key = req.api_key.unwrap_or_else(|| generate_api_key());
    let storage_path = req
        .storage_path
        .unwrap_or_else(|| format!("user_{}", req.name));
    let user_id = req.name.clone();
    let created_at = Utc::now().to_rfc3339();

    let db = state.db.clone();
    let user_id_clone = user_id.clone();
    let req_name = req.name.clone();
    let storage_path_clone = storage_path.clone();
    let root_dir = state.root_dir.clone();

    let result = tokio::task::spawn_blocking(move || {
        let conn = db.blocking_lock();

        match conn.execute(
            "INSERT OR REPLACE INTO users (id, name, api_key, storage_path, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![user_id_clone, req_name, user_api_key, storage_path_clone, created_at],
        ) {
            Ok(_) => {
                let user_dir = get_user_storage_dir(&root_dir, &storage_path_clone);
                if !user_dir.exists() {
                    fs::create_dir_all(&user_dir).expect("Failed to create user directory");
                }

                Ok(User {
                    id: user_id_clone.clone(),
                    name: req_name.clone(),
                    api_key: user_api_key.clone(),
                    storage_path: storage_path_clone.clone(),
                    created_at: created_at.clone(),
                })
            }
            Err(e) => Err(e.to_string()),
        }
    }).await;

    match result {
        Ok(Ok(user)) => Json(ApiResponse {
            success: true,
            data: Some(user),
            message: "User created successfully".to_string(),
        }),
        Ok(Err(e)) => Json(ApiResponse {
            success: false,
            data: None,
            message: format!("Failed to create user: {}", e),
        }),
        Err(_) => Json(ApiResponse {
            success: false,
            data: None,
            message: "Internal error".to_string(),
        }),
    }
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

    let db = state.db.clone();
    let result = tokio::task::spawn_blocking(move || {
        let conn = db.blocking_lock();
        let mut stmt = conn
            .prepare("SELECT id, name, api_key, storage_path, created_at FROM users")
            .map_err(|e| e.to_string())?;

        let users: Vec<User> = stmt
            .query_map([], |row| {
                Ok(User {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    api_key: row.get(2)?,
                    storage_path: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        Ok::<_, String>(users)
    })
    .await;

    match result {
        Ok(Ok(users)) => Json(ApiResponse {
            success: true,
            data: Some(users),
            message: "Users listed successfully".to_string(),
        }),
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: "Failed to list users".to_string(),
        }),
    }
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
