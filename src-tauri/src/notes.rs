use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use uuid::Uuid;

const DEFAULT_BACKUP_RETENTION_DAYS: u64 = 7;
const NOTE_INDEX_FILE: &str = ".note-index.json";
const NOTE_CONTENT_DIR: &str = ".note-files";
const NOTE_ASSET_DIR: &str = ".note-assets";
const NOTE_DELETED_DIR: &str = ".note-deleted";
const NOTE_SYNC_SERVER_URL_KEY: &str = "noteSyncServerUrl";
const NOTE_SYNC_API_KEY_KEY: &str = "noteSyncApiKey";
const NOTE_SYNC_LAST_SYNCED_AT_KEY: &str = "noteSyncLastSyncedAt";
const DEFAULT_NOTE_SYNC_SERVER_URL: &str = "http://localhost:3000";
const DEFAULT_NOTE_SYNC_API_KEY: &str = "TRpLxzmSyMxVHAINVdj0sdqFKcIN8vSF";
const DEBUG_MODE_ENABLED_KEY: &str = "debugModeEnabled";
const MAX_GENERATED_SYNC_FILE_NAME_BYTES: usize = 120;
const MAX_SYNC_FILE_NAME_BYTES: usize = 180;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteTreeNode {
    name: String,
    path: String,
    node_type: String,
    children: Option<Vec<NoteTreeNode>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteDocument {
    pub(crate) path: String,
    pub(crate) title: String,
    pub(crate) content: String,
    pub(crate) updated_at: Option<String>,
    pub(crate) last_backup_at: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteWorkspace {
    root_path: String,
    tree: Vec<NoteTreeNode>,
    backup_root_path: String,
    backup_retention_days: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteSaveResult {
    updated_at: Option<String>,
    last_backup_at: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteImageAsset {
    file_path: String,
    markdown_path: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NoteSyncProgress {
    stage: String,
    current: usize,
    total: usize,
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteSyncResult {
    checked_at: String,
    last_synced_at: String,
    download_total: usize,
    upload_total: usize,
    downloaded: usize,
    uploaded: usize,
    skipped: usize,
    message: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NoteSyncSettings {
    server_url: String,
    api_key: String,
    last_synced_at: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DebugSettings {
    enabled: bool,
}

#[derive(Clone)]
struct LocalNoteSyncFile {
    id: String,
    sync_id: Option<String>,
    title: String,
    path: PathBuf,
    relative_path: String,
    modified_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize, Clone)]
struct ServerFileInfo {
    id: String,
    original_name: String,
    #[allow(dead_code)]
    stored_name: String,
    relative_path: String,
    #[allow(dead_code)]
    size: u64,
    #[allow(dead_code)]
    mime_type: Option<String>,
    last_updated: String,
    #[allow(dead_code)]
    version: u32,
    #[serde(default)]
    is_deleted: bool,
    #[allow(dead_code)]
    deleted_at: Option<String>,
}

#[derive(Deserialize)]
struct ServerApiResponse<T> {
    success: bool,
    data: Option<T>,
    message: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NoteSettings {
    notes_root_path: String,
    backup_root_path: String,
    mail_root_path: String,
    backup_retention_days: u64,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AIProviderConfig {
    id: String,
    name: String,
    vendor: String,
    api_key: String,
    base_url: String,
    model: String,
    timeout_ms: u64,
    enabled: bool,
    updated_at: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AIProviderInput {
    id: Option<String>,
    name: String,
    vendor: String,
    api_key: String,
    base_url: String,
    model: String,
    timeout_ms: u64,
    enabled: bool,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ImageHostConfig {
    id: String,
    name: String,
    vendor: String,
    bucket: String,
    region: String,
    endpoint: String,
    access_key_id: String,
    access_key_secret: String,
    upload_token: String,
    cdn_url: String,
    path_prefix: String,
    enabled: bool,
    updated_at: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ImageHostInput {
    id: Option<String>,
    name: String,
    vendor: String,
    bucket: String,
    region: String,
    endpoint: String,
    access_key_id: String,
    access_key_secret: String,
    upload_token: String,
    cdn_url: String,
    path_prefix: String,
    enabled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupHistoryItem {
    path: String,
    name: String,
    note_title: String,
    note_path: String,
    created_at: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupDocument {
    path: String,
    name: String,
    note_title: String,
    note_path: String,
    content: String,
    created_at: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct NoteDirectoryIndex {
    notes: Vec<NoteIndexRecord>,
    #[serde(default)]
    deleted: Vec<NoteDeletionRecord>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct NoteIndexRecord {
    id: String,
    #[serde(default)]
    sync_id: Option<String>,
    name: String,
    updated_at: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct NoteDeletionRecord {
    id: String,
    #[serde(default)]
    sync_id: Option<String>,
    relative_path: String,
    deleted_at: String,
}

fn settings_file_path(app: &AppHandle) -> Result<PathBuf, String> {
    let app_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Failed to get app data directory: {error}"))?;

    fs::create_dir_all(&app_dir)
        .map_err(|error| format!("Failed to create app data directory: {error}"))?;
    Ok(app_dir.join("settings.json"))
}

fn read_settings_json(app: &AppHandle) -> Result<Map<String, Value>, String> {
    let settings_path = settings_file_path(app)?;

    if !settings_path.exists() {
        return Ok(Map::new());
    }

    let content =
        fs::read_to_string(&settings_path).map_err(|error| format!("Failed to read settings: {error}"))?;
    let raw: Value =
        serde_json::from_str(&content).map_err(|error| format!("Failed to parse settings: {error}"))?;

    match raw {
        Value::Object(map) => Ok(map),
        _ => Ok(Map::new()),
    }
}

fn write_settings_json(app: &AppHandle, settings: &Map<String, Value>) -> Result<(), String> {
    let settings_path = settings_file_path(app)?;
    let content = serde_json::to_string_pretty(settings)
        .map_err(|error| format!("Failed to serialize settings: {error}"))?;

    fs::write(settings_path, content).map_err(|error| format!("Failed to save settings: {error}"))
}

fn default_notes_root(app: &AppHandle) -> Result<PathBuf, String> {
    let root = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Failed to get app data directory: {error}"))?
        .join("notes");

    fs::create_dir_all(&root)
        .map_err(|error| format!("Failed to create default notes directory: {error}"))?;
    Ok(root)
}

fn default_backup_root(app: &AppHandle) -> Result<PathBuf, String> {
    let root = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Failed to get app data directory: {error}"))?
        .join("backups");

    fs::create_dir_all(&root)
        .map_err(|error| format!("Failed to create default backup directory: {error}"))?;
    Ok(root)
}

fn default_mail_root(app: &AppHandle) -> Result<PathBuf, String> {
    let root = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Failed to get app data directory: {error}"))?
        .join("mail_cache");

    fs::create_dir_all(&root)
        .map_err(|error| format!("Failed to create default mail directory: {error}"))?;
    Ok(root)
}

fn normalize_or_create_dir(path: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(path).map_err(|error| format!("Failed to create directory: {error}"))?;
    fs::canonicalize(path).map_err(|error| format!("Failed to resolve directory path: {error}"))
}

fn display_path(path: &Path) -> String {
    let raw = path.to_string_lossy().to_string();

    #[cfg(windows)]
    {
        raw.strip_prefix(r"\\?\")
            .map(ToString::to_string)
            .unwrap_or(raw)
    }

    #[cfg(not(windows))]
    {
        raw
    }
}

fn file_uri_from_path(path: &Path) -> String {
    let normalized = display_path(path).replace('\\', "/");

    #[cfg(windows)]
    {
        format!("file:///{}", normalized)
    }

    #[cfg(not(windows))]
    {
        format!("file://{}", normalized)
    }
}

fn default_note_settings(app: &AppHandle) -> Result<NoteSettings, String> {
    Ok(NoteSettings {
        notes_root_path: display_path(&default_notes_root(app)?),
        backup_root_path: display_path(&default_backup_root(app)?),
        mail_root_path: display_path(&default_mail_root(app)?),
        backup_retention_days: DEFAULT_BACKUP_RETENTION_DAYS,
    })
}

fn read_note_settings(app: &AppHandle) -> Result<NoteSettings, String> {
    let raw = read_settings_json(app)?;
    let defaults = default_note_settings(app)?;

    let notes_root_path = raw
        .get("notesRootPath")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| defaults.notes_root_path.clone());

    let backup_root_path = raw
        .get("backupRootPath")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| defaults.backup_root_path.clone());

    let mail_root_path = raw
        .get("mailRootPath")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| defaults.mail_root_path.clone());

    let backup_retention_days = raw
        .get("backupRetentionDays")
        .and_then(Value::as_u64)
        .filter(|value| *value > 0)
        .unwrap_or(defaults.backup_retention_days);

    Ok(NoteSettings {
        notes_root_path,
        backup_root_path,
        mail_root_path,
        backup_retention_days,
    })
}

fn write_note_settings(app: &AppHandle, settings: &NoteSettings) -> Result<NoteSettings, String> {
    let notes_root = normalize_or_create_dir(Path::new(&settings.notes_root_path))?;
    let backup_root = normalize_or_create_dir(Path::new(&settings.backup_root_path))?;
    let mail_root = normalize_or_create_dir(Path::new(&settings.mail_root_path))?;
    let normalized = NoteSettings {
        notes_root_path: display_path(&notes_root),
        backup_root_path: display_path(&backup_root),
        mail_root_path: display_path(&mail_root),
        backup_retention_days: settings.backup_retention_days.clamp(1, 30),
    };
    let mut raw = read_settings_json(app)?;
    raw.insert(
        "notesRootPath".to_string(),
        Value::String(normalized.notes_root_path.clone()),
    );
    raw.insert(
        "backupRootPath".to_string(),
        Value::String(normalized.backup_root_path.clone()),
    );
    raw.insert(
        "mailRootPath".to_string(),
        Value::String(normalized.mail_root_path.clone()),
    );
    raw.insert(
        "backupRetentionDays".to_string(),
        Value::Number(normalized.backup_retention_days.into()),
    );

    write_settings_json(app, &raw)?;
    Ok(normalized)
}

fn read_ai_providers(app: &AppHandle) -> Result<Vec<AIProviderConfig>, String> {
    let raw = read_settings_json(app)?;

    match raw.get("aiProviders") {
        Some(value) => serde_json::from_value::<Vec<AIProviderConfig>>(value.clone())
            .map_err(|error| format!("Failed to parse AI providers: {error}")),
        None => Ok(Vec::new()),
    }
}

fn write_ai_providers(app: &AppHandle, providers: &[AIProviderConfig]) -> Result<Vec<AIProviderConfig>, String> {
    let mut raw = read_settings_json(app)?;
    raw.insert(
        "aiProviders".to_string(),
        serde_json::to_value(providers).map_err(|error| format!("Failed to serialize AI providers: {error}"))?,
    );
    write_settings_json(app, &raw)?;
    Ok(providers.to_vec())
}

fn normalize_ai_provider(input: AIProviderInput) -> AIProviderConfig {
    AIProviderConfig {
        id: input.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        name: input.name.trim().to_string(),
        vendor: input.vendor.trim().to_string(),
        api_key: input.api_key.trim().to_string(),
        base_url: input.base_url.trim().trim_end_matches('/').to_string(),
        model: input.model.trim().to_string(),
        timeout_ms: input.timeout_ms.clamp(3000, 120_000),
        enabled: input.enabled,
        updated_at: chrono::Utc::now().to_rfc3339(),
    }
}

fn read_image_hosts(app: &AppHandle) -> Result<Vec<ImageHostConfig>, String> {
    let raw = read_settings_json(app)?;

    match raw.get("imageHosts") {
        Some(value) => serde_json::from_value::<Vec<ImageHostConfig>>(value.clone())
            .map_err(|error| format!("Failed to parse image hosts: {error}")),
        None => Ok(Vec::new()),
    }
}

fn write_image_hosts(app: &AppHandle, hosts: &[ImageHostConfig]) -> Result<Vec<ImageHostConfig>, String> {
    let mut raw = read_settings_json(app)?;
    raw.insert(
        "imageHosts".to_string(),
        serde_json::to_value(hosts).map_err(|error| format!("Failed to serialize image hosts: {error}"))?,
    );
    write_settings_json(app, &raw)?;
    Ok(hosts.to_vec())
}

fn normalize_image_host(input: ImageHostInput) -> ImageHostConfig {
    ImageHostConfig {
        id: input.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        name: input.name.trim().to_string(),
        vendor: input.vendor.trim().to_string(),
        bucket: input.bucket.trim().to_string(),
        region: input.region.trim().to_string(),
        endpoint: input.endpoint.trim().to_string(),
        access_key_id: input.access_key_id.trim().to_string(),
        access_key_secret: input.access_key_secret.trim().to_string(),
        upload_token: input.upload_token.trim().to_string(),
        cdn_url: input.cdn_url.trim().trim_end_matches('/').to_string(),
        path_prefix: input.path_prefix.trim().trim_matches('/').to_string(),
        enabled: input.enabled,
        updated_at: chrono::Utc::now().to_rfc3339(),
    }
}

fn notes_root(app: &AppHandle) -> Result<PathBuf, String> {
    let settings = read_note_settings(app)?;
    normalize_or_create_dir(Path::new(&settings.notes_root_path))
}

fn backups_root(app: &AppHandle) -> Result<PathBuf, String> {
    let settings = read_note_settings(app)?;
    normalize_or_create_dir(Path::new(&settings.backup_root_path))
}

fn canonical_notes_root(app: &AppHandle) -> Result<PathBuf, String> {
    let root = notes_root(app)?;
    fs::canonicalize(&root).map_err(|error| format!("Failed to resolve notes root: {error}"))
}

fn canonical_backups_root(app: &AppHandle) -> Result<PathBuf, String> {
    let root = backups_root(app)?;
    fs::canonicalize(&root).map_err(|error| format!("Failed to resolve backups root: {error}"))
}

fn canonical_path(path: &Path) -> Result<PathBuf, String> {
    fs::canonicalize(path).map_err(|error| format!("Failed to resolve path: {error}"))
}

fn note_index_path(directory: &Path) -> PathBuf {
    directory.join(NOTE_INDEX_FILE)
}

fn note_content_dir(directory: &Path) -> PathBuf {
    directory.join(NOTE_CONTENT_DIR)
}

fn note_content_path(directory: &Path, note_id: &str) -> PathBuf {
    note_content_dir(directory).join(format!("{note_id}.md"))
}

fn note_asset_root(directory: &Path) -> PathBuf {
    directory.join(NOTE_ASSET_DIR)
}

fn note_asset_dir(directory: &Path, note_id: &str) -> PathBuf {
    note_asset_root(directory).join(note_id)
}

fn note_deleted_dir(directory: &Path) -> PathBuf {
    directory.join(NOTE_DELETED_DIR)
}

fn note_deleted_path(directory: &Path, note_id: &str) -> PathBuf {
    note_deleted_dir(directory).join(format!("{note_id}.md"))
}

fn is_reserved_entry(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| {
            name == NOTE_INDEX_FILE || name == NOTE_CONTENT_DIR || name == NOTE_ASSET_DIR || name == NOTE_DELETED_DIR
        })
}

fn ensure_directory_storage(directory: &Path) -> Result<(), String> {
    fs::create_dir_all(directory).map_err(|error| format!("Failed to create note directory: {error}"))?;
    fs::create_dir_all(note_content_dir(directory))
        .map_err(|error| format!("Failed to create note content directory: {error}"))?;
    fs::create_dir_all(note_asset_root(directory))
        .map_err(|error| format!("Failed to create note asset directory: {error}"))?;

    let index_path = note_index_path(directory);
    if !index_path.exists() {
        let content = serde_json::to_string_pretty(&NoteDirectoryIndex::default())
            .map_err(|error| format!("Failed to initialize note index: {error}"))?;
        fs::write(index_path, content).map_err(|error| format!("Failed to write note index: {error}"))?;
    }

    Ok(())
}

fn read_directory_index(directory: &Path) -> Result<NoteDirectoryIndex, String> {
    ensure_directory_storage(directory)?;
    let index_path = note_index_path(directory);
    let content =
        fs::read_to_string(&index_path).map_err(|error| format!("Failed to read note index: {error}"))?;

    serde_json::from_str(&content).map_err(|error| format!("Failed to parse note index: {error}"))
}

fn write_directory_index(directory: &Path, index: &NoteDirectoryIndex) -> Result<(), String> {
    ensure_directory_storage(directory)?;
    let content = serde_json::to_string_pretty(index)
        .map_err(|error| format!("Failed to serialize note index: {error}"))?;

    fs::write(note_index_path(directory), content)
        .map_err(|error| format!("Failed to save note index: {error}"))
}

fn sanitize_note_title(value: &str) -> String {
    let trimmed = value.trim().trim_end_matches(".md").trim();
    if trimmed.is_empty() {
        "Untitled Note".into()
    } else {
        trimmed.into()
    }
}

fn sanitize_backup_name(value: &str) -> String {
    value.chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => ch,
        })
        .collect::<String>()
        .trim()
        .trim_matches('.')
        .to_string()
}

fn sanitize_asset_name(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '-',
            _ => ch,
        })
        .collect::<String>()
        .trim()
        .trim_matches('.')
        .to_string();

    if sanitized.is_empty() {
        "image.png".into()
    } else {
        sanitized
    }
}

fn unique_note_name(index: &NoteDirectoryIndex, desired: &str, current_id: Option<&str>) -> String {
    let base = sanitize_note_title(desired);

    let exists = |candidate: &str| {
        index.notes.iter().any(|note| {
            note.name.eq_ignore_ascii_case(candidate) && current_id.is_none_or(|id| note.id != id)
        })
    };

    if !exists(&base) {
        return base;
    }

    for number in 1..1000 {
        let next = format!("{base}-{number}");
        if !exists(&next) {
            return next;
        }
    }

    format!("{base}-{}", Uuid::new_v4().simple())
}

fn note_directory_from_note_path(note_path: &Path) -> Result<PathBuf, String> {
    let content_dir = note_path
        .parent()
        .ok_or_else(|| "Failed to resolve note content directory".to_string())?;
    let content_dir_name = content_dir
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "Failed to resolve note content directory name".to_string())?;

    if content_dir_name != NOTE_CONTENT_DIR {
        return Err("The note path is not inside the managed note storage".into());
    }

    content_dir
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "Failed to resolve note directory".to_string())
}

fn note_id_from_note_path(note_path: &Path) -> Result<String, String> {
    note_path
        .file_stem()
        .and_then(|value| value.to_str())
        .map(ToString::to_string)
        .ok_or_else(|| "Failed to resolve note id".to_string())
}

fn find_note_record(directory: &Path, note_id: &str) -> Result<NoteIndexRecord, String> {
    let index = read_directory_index(directory)?;
    index
        .notes
        .into_iter()
        .find(|record| record.id == note_id)
        .ok_or_else(|| "Failed to find the note in the index".to_string())
}

fn note_record_for_path(note_path: &Path) -> Result<(PathBuf, String, NoteIndexRecord), String> {
    let directory = note_directory_from_note_path(note_path)?;
    let note_id = note_id_from_note_path(note_path)?;
    let record = find_note_record(&directory, &note_id)?;
    Ok((directory, note_id, record))
}

fn modified_at(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?;
    let local_time = chrono::DateTime::<chrono::Local>::from(modified);
    Some(local_time.format("%Y-%m-%d %H:%M:%S").to_string())
}

fn modified_at_utc(path: &Path) -> Option<chrono::DateTime<chrono::Utc>> {
    let metadata = fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?;
    Some(chrono::DateTime::<chrono::Utc>::from(modified))
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn parse_rfc3339(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&chrono::Utc))
}

fn truncate_utf8(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_string();
    }

    let mut end = 0;
    for (index, ch) in value.char_indices() {
        let next = index + ch.len_utf8();
        if next > max_bytes {
            break;
        }
        end = next;
    }

    value[..end].to_string()
}

fn sanitize_sync_file_name(value: &str) -> String {
    let sanitized = value
        .trim()
        .trim_end_matches(".md")
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => ch,
        })
        .collect::<String>()
        .trim()
        .trim_matches('.')
        .to_string();

    let stem = if sanitized.is_empty() {
        "Untitled Note".to_string()
    } else {
        truncate_utf8(&sanitized, MAX_GENERATED_SYNC_FILE_NAME_BYTES)
    };

    format!("{stem}.md")
}

fn validate_sync_file_name(file_name: &str) -> Result<(), String> {
    let trimmed = file_name.trim();
    if trimmed.is_empty() {
        return Err("同步文件名为空，请检查笔记标题后重试".to_string());
    }

    if trimmed.len() > MAX_SYNC_FILE_NAME_BYTES {
        return Err(format!(
            "同步文件名过长，请缩短笔记标题后重试：{}",
            trimmed
        ));
    }

    if trimmed.chars().any(|ch| matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || ch.is_control()) {
        return Err(format!("同步文件名包含非法字符，请修改后重试：{}", trimmed));
    }

    if trimmed == "." || trimmed == ".." || trimmed.ends_with('.') {
        return Err(format!("同步文件名格式不合法，请修改后重试：{}", trimmed));
    }

    Ok(())
}

fn validate_sync_relative_path(relative_path: &str) -> Result<(), String> {
    let trimmed = relative_path.trim().trim_start_matches("./");
    if trimmed.is_empty() {
        return Err("同步路径为空，请检查远端文件信息".to_string());
    }

    for component in trimmed.split('/') {
        validate_sync_file_name(component)?;
    }

    Ok(())
}

fn sync_upload_file_name(relative_path: &str, title: &str) -> Result<String, String> {
    let file_name = relative_path
        .rsplit(|ch| ch == '/' || ch == '\\')
        .next()
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.to_string())
        .unwrap_or_else(|| sanitize_sync_file_name(title));

    validate_sync_file_name(&file_name)?;
    Ok(file_name)
}

fn relative_sync_path(app: &AppHandle, directory: &Path, title: &str) -> Result<String, String> {
    let root = canonical_notes_root(app)?;
    let normalized_directory = canonical_path(directory)?;
    let relative_directory = normalized_directory
        .strip_prefix(&root)
        .map_err(|error| format!("Failed to build sync relative path: {error}"))?;
    let relative = relative_directory.join(sanitize_sync_file_name(title));
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

fn update_note_sync_id(directory: &Path, note_id: &str, sync_id: &str) -> Result<(), String> {
    let mut index = read_directory_index(directory)?;
    if let Some(record) = index.notes.iter_mut().find(|record| record.id == note_id) {
        record.sync_id = Some(sync_id.to_string());
        write_directory_index(directory, &index)?;
    }
    Ok(())
}

fn collect_local_deletions(directory: &Path, deleted: &mut Vec<NoteDeletionRecord>) -> Result<(), String> {
    ensure_directory_storage(directory)?;

    for entry in fs::read_dir(directory).map_err(|error| format!("Failed to read deleted sync directory: {error}"))? {
        let entry = entry.map_err(|error| format!("Failed to read deleted sync directory entry: {error}"))?;
        let entry_path = entry.path();

        if is_reserved_entry(&entry_path) {
            continue;
        }

        if entry_path.is_dir() {
            collect_local_deletions(&entry_path, deleted)?;
        }
    }

    let index = read_directory_index(directory)?;
    deleted.extend(index.deleted);
    Ok(())
}

fn cleanup_expired_deletions(app: &AppHandle, directory: &Path) -> Result<(), String> {
    ensure_directory_storage(directory)?;

    for entry in fs::read_dir(directory).map_err(|error| format!("Failed to read deletion cleanup directory: {error}"))? {
        let entry = entry.map_err(|error| format!("Failed to read deletion cleanup entry: {error}"))?;
        let entry_path = entry.path();

        if is_reserved_entry(&entry_path) {
            continue;
        }

        if entry_path.is_dir() {
            cleanup_expired_deletions(app, &entry_path)?;
        }
    }

    let settings = read_note_settings(app)?;
    let max_age = chrono::Duration::days(settings.backup_retention_days.clamp(1, 30) as i64);
    let now = chrono::Utc::now();
    let mut index = read_directory_index(directory)?;
    let mut retained = Vec::new();

    for deleted in index.deleted {
        let expired = parse_rfc3339(&deleted.deleted_at)
            .is_some_and(|deleted_at| now.signed_duration_since(deleted_at) > max_age);
        if expired {
            let deleted_path = note_deleted_path(directory, &deleted.id);
            let _ = fs::remove_file(deleted_path);
        } else {
            retained.push(deleted);
        }
    }

    index.deleted = retained;
    write_directory_index(directory, &index)
}

fn mark_local_note_deleted(app: &AppHandle, directory: &Path, note_id: &str) -> Result<(), String> {
    let mut index = read_directory_index(directory)?;
    let Some(position) = index.notes.iter().position(|record| record.id == note_id) else {
        return Ok(());
    };

    let record = index.notes.remove(position);
    let relative_path = relative_sync_path(app, directory, &record.name)?;
    let deleted_at = now_rfc3339();

    let note_path = note_content_path(directory, note_id);
    if note_path.exists() {
        fs::create_dir_all(note_deleted_dir(directory))
            .map_err(|error| format!("Failed to create deleted note directory: {error}"))?;
        let deleted_path = note_deleted_path(directory, note_id);
        fs::rename(&note_path, &deleted_path).or_else(|_| {
            fs::copy(&note_path, &deleted_path)?;
            fs::remove_file(&note_path)
        }).map_err(|error| format!("Failed to move deleted note: {error}"))?;
    }

    index.deleted.push(NoteDeletionRecord {
        id: record.id,
        sync_id: record.sync_id,
        relative_path,
        deleted_at,
    });
    write_directory_index(directory, &index)
}

fn backup_timestamp() -> String {
    chrono::Local::now().format("%Y%m%d_%H%M%S").to_string()
}

fn migrate_legacy_directory(directory: &Path) -> Result<(), String> {
    ensure_directory_storage(directory)?;
    let mut index = read_directory_index(directory)?;
    let mut changed = false;

    for entry in fs::read_dir(directory).map_err(|error| format!("Failed to read directory: {error}"))? {
        let entry = entry.map_err(|error| format!("Failed to read directory entry: {error}"))?;
        let entry_path = entry.path();

        if is_reserved_entry(&entry_path) {
            continue;
        }

        if entry_path.is_dir() {
            migrate_legacy_directory(&entry_path)?;
            continue;
        }

        let is_markdown = entry_path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"));

        if !is_markdown {
            continue;
        }

        let title = entry_path
            .file_stem()
            .and_then(|value| value.to_str())
            .map(sanitize_note_title)
            .unwrap_or_else(|| "Untitled Note".into());

        let unique_title = unique_note_name(&index, &title, None);
        let note_id = Uuid::new_v4().to_string();
        let target = note_content_path(directory, &note_id);

        fs::rename(&entry_path, &target)
            .map_err(|error| format!("Failed to migrate legacy note: {error}"))?;

        index.notes.push(NoteIndexRecord {
            id: note_id,
            sync_id: None,
            name: unique_title,
            updated_at: modified_at(&target),
        });
        changed = true;
    }

    if changed {
        write_directory_index(directory, &index)?;
    }

    Ok(())
}

fn prepare_notes_workspace(app: &AppHandle) -> Result<PathBuf, String> {
    let root = notes_root(app)?;
    migrate_legacy_directory(&root)?;
    Ok(root)
}

fn validate_workspace_path(root: &Path, path: &Path) -> Result<PathBuf, String> {
    let target = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };

    let normalized = if target.exists() {
        canonical_path(&target)?
    } else {
        target
    };

    if !normalized.starts_with(root) {
        return Err("Access outside the workspace is not allowed".into());
    }

    Ok(normalized)
}

fn validate_note_path(app: &AppHandle, path: &Path) -> Result<PathBuf, String> {
    let root = canonical_notes_root(app)?;
    let normalized = validate_workspace_path(&root, path)?;

    if normalized.is_dir() || is_reserved_entry(&normalized) {
        return Err("The requested note path is invalid".into());
    }

    let directory = note_directory_from_note_path(&normalized)?;
    ensure_directory_storage(&directory)?;
    Ok(normalized)
}

fn validate_directory_path(app: &AppHandle, path: &Path) -> Result<PathBuf, String> {
    let root = canonical_notes_root(app)?;
    let normalized = validate_workspace_path(&root, path)?;

    if !normalized.is_dir() {
        return Err("The requested directory path is invalid".into());
    }

    if is_reserved_entry(&normalized) {
        return Err("The requested directory path is reserved".into());
    }

    ensure_directory_storage(&normalized)?;
    Ok(normalized)
}

fn validate_backup_path(app: &AppHandle, path: &Path) -> Result<PathBuf, String> {
    let normalized_root = canonical_backups_root(app)?;
    validate_workspace_path(&normalized_root, path)
}

fn build_tree(path: &Path) -> Result<Vec<NoteTreeNode>, String> {
    ensure_directory_storage(path)?;
    let mut directories = Vec::new();
    let mut files = Vec::new();

    for entry in fs::read_dir(path).map_err(|error| format!("Failed to read directory: {error}"))? {
        let entry = entry.map_err(|error| format!("Failed to read directory entry: {error}"))?;
        let entry_path = entry.path();

        if is_reserved_entry(&entry_path) {
            continue;
        }

        if entry_path.is_dir() {
            directories.push(NoteTreeNode {
                name: entry_path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default()
                    .to_string(),
                path: display_path(&entry_path),
                node_type: "directory".into(),
                children: Some(build_tree(&entry_path)?),
            });
        }
    }

    let index = read_directory_index(path)?;
    for note in index.notes {
        let note_path = note_content_path(path, &note.id);
        if !note_path.exists() {
            continue;
        }

        files.push(NoteTreeNode {
            name: note.name,
            path: display_path(&note_path),
            node_type: "file".into(),
            children: None,
        });
    }

    directories.sort_by(|left, right| left.name.cmp(&right.name));
    files.sort_by(|left, right| left.name.cmp(&right.name));
    directories.extend(files);

    Ok(directories)
}

fn collect_local_sync_files(app: &AppHandle, directory: &Path, files: &mut Vec<LocalNoteSyncFile>) -> Result<(), String> {
    ensure_directory_storage(directory)?;

    for entry in fs::read_dir(directory).map_err(|error| format!("Failed to read sync directory: {error}"))? {
        let entry = entry.map_err(|error| format!("Failed to read sync directory entry: {error}"))?;
        let entry_path = entry.path();

        if is_reserved_entry(&entry_path) {
            continue;
        }

        if entry_path.is_dir() {
            collect_local_sync_files(app, &entry_path, files)?;
        }
    }

    let index = read_directory_index(directory)?;
    for note in index.notes {
        let note_path = note_content_path(directory, &note.id);
        if !note_path.exists() {
            continue;
        }

        let Some(modified_at) = modified_at_utc(&note_path) else {
            continue;
        };

        files.push(LocalNoteSyncFile {
            id: note.id.clone(),
            sync_id: note.sync_id.clone(),
            title: note.name.clone(),
            path: note_path,
            relative_path: relative_sync_path(app, directory, &note.name)?,
            modified_at,
        });
    }

    Ok(())
}

fn sync_settings(app: &AppHandle) -> Result<(String, String, Option<String>), String> {
    let raw = read_settings_json(app)?;
    let server_url = raw
        .get(NOTE_SYNC_SERVER_URL_KEY)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(DEFAULT_NOTE_SYNC_SERVER_URL)
        .trim()
        .trim_end_matches('/')
        .to_string();
    let api_key = raw
        .get(NOTE_SYNC_API_KEY_KEY)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(DEFAULT_NOTE_SYNC_API_KEY)
        .trim()
        .to_string();
    let last_synced_at = raw
        .get(NOTE_SYNC_LAST_SYNCED_AT_KEY)
        .and_then(Value::as_str)
        .map(ToString::to_string);

    Ok((server_url, api_key, last_synced_at))
}

fn read_note_sync_settings(app: &AppHandle) -> Result<NoteSyncSettings, String> {
    let (server_url, api_key, last_synced_at) = sync_settings(app)?;

    Ok(NoteSyncSettings {
        server_url,
        api_key,
        last_synced_at,
    })
}

fn write_note_sync_settings(app: &AppHandle, server_url: String, api_key: String) -> Result<NoteSyncSettings, String> {
    let normalized_server_url = server_url.trim().trim_end_matches('/').to_string();
    let normalized_api_key = api_key.trim().to_string();

    if normalized_server_url.is_empty() {
        return Err("同步 API 地址不能为空".to_string());
    }

    if normalized_api_key.is_empty() {
        return Err("同步校验 Key 不能为空".to_string());
    }

    let mut raw = read_settings_json(app)?;
    raw.insert(
        NOTE_SYNC_SERVER_URL_KEY.to_string(),
        Value::String(normalized_server_url),
    );
    raw.insert(
        NOTE_SYNC_API_KEY_KEY.to_string(),
        Value::String(normalized_api_key),
    );
    write_settings_json(app, &raw)?;

    read_note_sync_settings(app)
}

fn read_debug_settings(app: &AppHandle) -> Result<DebugSettings, String> {
    let raw = read_settings_json(app)?;
    Ok(DebugSettings {
        enabled: raw
            .get(DEBUG_MODE_ENABLED_KEY)
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

fn write_debug_settings(app: &AppHandle, enabled: bool) -> Result<DebugSettings, String> {
    let mut raw = read_settings_json(app)?;
    raw.insert(DEBUG_MODE_ENABLED_KEY.to_string(), Value::Bool(enabled));
    write_settings_json(app, &raw)?;
    Ok(DebugSettings { enabled })
}

fn emit_debug_log(app: &AppHandle, level: &str, action: &str, detail: impl Into<String>) {
    if !read_debug_settings(app).map(|settings| settings.enabled).unwrap_or(false) {
        return;
    }

    let _ = app.emit(
        "app-debug-log",
        serde_json::json!({
            "source": "backend",
            "level": level,
            "action": action,
            "detail": detail.into(),
            "timestamp": now_rfc3339(),
        }),
    );
}

fn write_sync_time(app: &AppHandle, synced_at: &str) -> Result<(), String> {
    let mut raw = read_settings_json(app)?;
    raw.insert(
        NOTE_SYNC_LAST_SYNCED_AT_KEY.to_string(),
        Value::String(synced_at.to_string()),
    );
    write_settings_json(app, &raw)
}

fn emit_sync_progress(window: &WebviewWindow, stage: &str, current: usize, total: usize, message: impl Into<String>) {
    let _ = window.emit(
        "note-sync-progress",
        NoteSyncProgress {
            stage: stage.to_string(),
            current,
            total,
            message: message.into(),
        },
    );
}

async fn list_remote_files(client: &reqwest::Client, server_url: &str, api_key: &str) -> Result<Vec<ServerFileInfo>, String> {
    let response = client
        .get(format!("{server_url}/api/files/list"))
        .header("X-API-Key", api_key)
        .send()
        .await
        .map_err(|error| format!("Failed to request remote file list: {error}"))?
        .json::<ServerApiResponse<Vec<ServerFileInfo>>>()
        .await
        .map_err(|error| format!("Failed to parse remote file list: {error}"))?;

    if !response.success {
        return Err(response.message);
    }

    Ok(response.data.unwrap_or_default())
}

async fn download_remote_file(client: &reqwest::Client, server_url: &str, api_key: &str, file_id: &str) -> Result<Vec<u8>, String> {
    let response = client
        .get(format!("{server_url}/api/files/download/{file_id}"))
        .header("X-API-Key", api_key)
        .send()
        .await
        .map_err(|error| format!("Failed to download remote file: {error}"))?
        .json::<ServerApiResponse<Vec<u8>>>()
        .await
        .map_err(|error| format!("Failed to parse remote file: {error}"))?;

    if !response.success {
        return Err(response.message);
    }

    response.data.ok_or_else(|| "Remote file response is empty".to_string())
}

async fn delete_remote_file(client: &reqwest::Client, server_url: &str, api_key: &str, file_id: &str) -> Result<(), String> {
    let response = client
        .post(format!("{server_url}/api/files/delete/{file_id}"))
        .header("X-API-Key", api_key)
        .send()
        .await
        .map_err(|error| format!("Failed to delete remote file: {error}"))?
        .json::<ServerApiResponse<()>>()
        .await
        .map_err(|error| format!("Failed to parse remote delete response: {error}"))?;

    if !response.success {
        return Err(response.message);
    }

    Ok(())
}

async fn upload_local_file(
    client: &reqwest::Client,
    server_url: &str,
    api_key: &str,
    local: &LocalNoteSyncFile,
    remote_id: Option<&str>,
) -> Result<ServerFileInfo, String> {
    validate_sync_relative_path(&local.relative_path)?;
    let data = fs::read(&local.path).map_err(|error| format!("Failed to read local sync file: {error}"))?;
    let file_name = sync_upload_file_name(&local.relative_path, &local.title)?;
    let file_part = reqwest::multipart::Part::bytes(data)
        .file_name(file_name)
        .mime_str("text/markdown")
        .map_err(|error| format!("Failed to build upload multipart: {error}"))?;
    let form = reqwest::multipart::Form::new()
        .text("relative_path", local.relative_path.clone())
        .text("last_updated", local.modified_at.to_rfc3339())
        .part("file", file_part);

    let request = if let Some(file_id) = remote_id {
        client.post(format!("{server_url}/api/files/update/{file_id}"))
    } else {
        client.post(format!("{server_url}/api/files/upload"))
    };

    let response = request
        .header("X-API-Key", api_key)
        .multipart(form)
        .send()
        .await
        .map_err(|error| format!("Failed to upload local file: {error}"))?
        .json::<ServerApiResponse<ServerFileInfo>>()
        .await
        .map_err(|error| format!("Failed to parse upload response: {error}"))?;

    if !response.success {
        return Err(response.message);
    }

    response.data.ok_or_else(|| "Upload response is empty".to_string())
}

fn upsert_downloaded_note(app: &AppHandle, remote: &ServerFileInfo, content: &[u8]) -> Result<(), String> {
    let root = prepare_notes_workspace(app)?;
    validate_sync_relative_path(&remote.relative_path)?;
    let relative_path = PathBuf::from(remote.relative_path.trim_start_matches("./"));
    let safe_relative = relative_path
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => Some(value.to_os_string()),
            _ => None,
        })
        .collect::<PathBuf>();
    let parent_relative = safe_relative.parent().map(Path::to_path_buf).unwrap_or_default();
    let file_name = safe_relative
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(&remote.original_name);
    let title = sanitize_note_title(file_name);
    let directory = root.join(parent_relative);

    fs::create_dir_all(&directory).map_err(|error| format!("Failed to create synced note directory: {error}"))?;
    ensure_directory_storage(&directory)?;

    let mut index = read_directory_index(&directory)?;
    let existing = index.notes.iter().find_map(|record| {
        if record.sync_id.as_deref() == Some(remote.id.as_str()) {
            return Some(record.id.clone());
        }

        let path = relative_sync_path(app, &directory, &record.name).ok()?;
        if path == remote.relative_path {
            Some(record.id.clone())
        } else {
            None
        }
    });
    let note_id = existing.unwrap_or_else(|| Uuid::new_v4().to_string());
    let note_path = note_content_path(&directory, &note_id);

    fs::write(&note_path, content).map_err(|error| format!("Failed to write synced note: {error}"))?;

    if let Some(record) = index.notes.iter_mut().find(|record| record.id == note_id) {
        record.name = title;
        record.sync_id = Some(remote.id.clone());
        record.updated_at = modified_at(&note_path);
    } else {
        index.notes.push(NoteIndexRecord {
            id: note_id,
            sync_id: Some(remote.id.clone()),
            name: title,
            updated_at: modified_at(&note_path),
        });
    }

    write_directory_index(&directory, &index)
}

fn relative_note_directory_path(app: &AppHandle, directory: &Path) -> Result<PathBuf, String> {
    let normalized_root = canonical_notes_root(app)?;
    let normalized_directory = canonical_path(directory)?;

    normalized_directory
        .strip_prefix(&normalized_root)
        .map(Path::to_path_buf)
        .map_err(|error| format!("Failed to build relative note directory path: {error}"))
}

fn backup_directory_for_note(app: &AppHandle, note_path: &Path) -> Result<PathBuf, String> {
    let note_directory = note_directory_from_note_path(note_path)?;
    let note_id = note_id_from_note_path(note_path)?;
    let relative_directory = relative_note_directory_path(app, &note_directory)?;
    let backup_dir = backups_root(app)?.join(relative_directory).join(note_id);

    fs::create_dir_all(&backup_dir).map_err(|error| format!("Failed to create backup directory: {error}"))?;
    Ok(backup_dir)
}

fn note_title_from_backup_name(file_name: &str) -> String {
    let without_ext = file_name.strip_suffix(".md").unwrap_or(file_name);
    let parts: Vec<&str> = without_ext.split("__").collect();

    if parts.len() >= 2 {
        parts[0].replace('_', " ")
    } else {
        "Untitled Note".into()
    }
}

fn backup_item_from_path(app: &AppHandle, backup_path: &Path) -> Result<BackupHistoryItem, String> {
    let backups_root = canonical_backups_root(app)?;
    let normalized_backup = canonical_path(backup_path)?;
    let relative = normalized_backup
        .strip_prefix(&backups_root)
        .map_err(|error| format!("Failed to build relative backup path: {error}"))?;

    let file_name = normalized_backup
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "Failed to resolve backup file name".to_string())?;
    let note_id = normalized_backup
        .parent()
        .and_then(|value| value.file_name())
        .and_then(|value| value.to_str())
        .ok_or_else(|| "Failed to resolve backup note id".to_string())?;
    let relative_note_directory = relative
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let note_directory = notes_root(app)?.join(relative_note_directory);
    let note_path = note_content_path(&note_directory, note_id);

    let note_title = find_note_record(&note_directory, note_id)
        .map(|record| record.name)
        .unwrap_or_else(|_| note_title_from_backup_name(file_name));

    Ok(BackupHistoryItem {
        path: display_path(&normalized_backup),
        name: file_name.to_string(),
        note_title,
        note_path: display_path(&note_path),
        created_at: modified_at(&normalized_backup),
    })
}

fn create_backup(app: &AppHandle, note_path: &Path, content: &str) -> Result<Option<String>, String> {
    let (_, _, record) = note_record_for_path(note_path)?;
    let backup_dir = backup_directory_for_note(app, note_path)?;
    let backup_name = sanitize_backup_name(&record.name);
    let backup_path = backup_dir.join(format!("{backup_name}__{}.md", backup_timestamp()));

    fs::write(&backup_path, content).map_err(|error| format!("Failed to write backup: {error}"))?;
    cleanup_expired_backups(app)?;

    Ok(modified_at(&backup_path))
}

fn cleanup_expired_backups(app: &AppHandle) -> Result<(), String> {
    let root = backups_root(app)?;
    let settings = read_note_settings(app)?;
    let now = SystemTime::now();
    let max_age = Duration::from_secs(settings.backup_retention_days.clamp(1, 30) * 24 * 60 * 60);

    cleanup_backup_dir(&root, now, max_age)
}

fn cleanup_backup_dir(path: &Path, now: SystemTime, max_age: Duration) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(path).map_err(|error| format!("Failed to read backup directory: {error}"))? {
        let entry = entry.map_err(|error| format!("Failed to read backup entry: {error}"))?;
        let entry_path = entry.path();

        if entry_path.is_dir() {
            cleanup_backup_dir(&entry_path, now, max_age)?;

            let is_empty = fs::read_dir(&entry_path)
                .map_err(|error| format!("Failed to scan backup subdirectory: {error}"))?
                .next()
                .is_none();

            if is_empty {
                let _ = fs::remove_dir(&entry_path);
            }

            continue;
        }

        let metadata =
            fs::metadata(&entry_path).map_err(|error| format!("Failed to read backup metadata: {error}"))?;
        let modified = metadata
            .modified()
            .map_err(|error| format!("Failed to read backup modified time: {error}"))?;

        if now.duration_since(modified).unwrap_or_default() > max_age {
            let _ = fs::remove_file(&entry_path);
        }
    }

    Ok(())
}

fn latest_backup_time_for_note(app: &AppHandle, note_path: &Path) -> Result<Option<String>, String> {
    let backup_dir = backup_directory_for_note(app, note_path)?;
    if !backup_dir.exists() {
        return Ok(None);
    }

    let mut latest: Option<(SystemTime, String)> = None;

    for entry in fs::read_dir(&backup_dir).map_err(|error| format!("Failed to read backup list: {error}"))? {
        let entry = entry.map_err(|error| format!("Failed to read backup item: {error}"))?;
        let entry_path = entry.path();

        let modified = fs::metadata(&entry_path)
            .and_then(|metadata| metadata.modified())
            .map_err(|error| format!("Failed to read backup timestamp: {error}"))?;
        let display = modified_at(&entry_path).unwrap_or_default();

        match &latest {
            Some((current_modified, _)) if modified <= *current_modified => {}
            _ => latest = Some((modified, display)),
        }
    }

    Ok(latest.map(|(_, display)| display))
}

#[tauri::command]
pub fn get_note_workspace_settings(app: AppHandle) -> Result<NoteSettings, String> {
    read_note_settings(&app)
}

#[tauri::command]
pub fn get_note_sync_settings(app: AppHandle) -> Result<NoteSyncSettings, String> {
    read_note_sync_settings(&app)
}

#[tauri::command]
pub fn get_debug_settings(app: AppHandle) -> Result<DebugSettings, String> {
    read_debug_settings(&app)
}

#[tauri::command]
pub fn update_debug_settings(app: AppHandle, enabled: bool) -> Result<DebugSettings, String> {
    let settings = write_debug_settings(&app, enabled)?;
    if settings.enabled {
        emit_debug_log(&app, "info", "debug.mode.updated", "调试模式已开启");
    }
    Ok(settings)
}

#[tauri::command]
pub fn update_note_sync_settings(
    app: AppHandle,
    server_url: String,
    api_key: String,
) -> Result<NoteSyncSettings, String> {
    write_note_sync_settings(&app, server_url, api_key)
}

#[tauri::command]
pub fn update_note_workspace_settings(
    app: AppHandle,
    notes_root_path: String,
    mail_root_path: Option<String>,
) -> Result<NoteSettings, String> {
    let current = read_note_settings(&app)?;
    let settings = NoteSettings {
        notes_root_path,
        backup_root_path: current.backup_root_path,
        mail_root_path: mail_root_path.unwrap_or(current.mail_root_path),
        backup_retention_days: current.backup_retention_days,
    };

    write_note_settings(&app, &settings)
}

#[tauri::command]
pub fn get_backup_settings(app: AppHandle) -> Result<NoteSettings, String> {
    read_note_settings(&app)
}

#[tauri::command]
pub fn get_ai_providers(app: AppHandle) -> Result<Vec<AIProviderConfig>, String> {
    let mut providers = read_ai_providers(&app)?;
    providers.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    Ok(providers)
}

#[tauri::command]
pub fn save_ai_provider(app: AppHandle, provider: AIProviderInput) -> Result<Vec<AIProviderConfig>, String> {
    let mut providers = read_ai_providers(&app)?;
    let normalized = normalize_ai_provider(provider);

    if let Some(existing) = providers.iter_mut().find(|item| item.id == normalized.id) {
        *existing = normalized.clone();
    } else {
        providers.push(normalized);
    }

    providers.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    write_ai_providers(&app, &providers)
}

#[tauri::command]
pub fn delete_ai_provider(app: AppHandle, provider_id: String) -> Result<Vec<AIProviderConfig>, String> {
    let providers: Vec<AIProviderConfig> = read_ai_providers(&app)?
        .into_iter()
        .filter(|item| item.id != provider_id)
        .collect();

    write_ai_providers(&app, &providers)
}

#[tauri::command]
pub fn get_image_hosts(app: AppHandle) -> Result<Vec<ImageHostConfig>, String> {
    let mut hosts = read_image_hosts(&app)?;
    hosts.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    Ok(hosts)
}

#[tauri::command]
pub fn save_image_host(app: AppHandle, host: ImageHostInput) -> Result<Vec<ImageHostConfig>, String> {
    let mut hosts = read_image_hosts(&app)?;
    let normalized = normalize_image_host(host);

    if let Some(existing) = hosts.iter_mut().find(|item| item.id == normalized.id) {
        *existing = normalized.clone();
    } else {
        hosts.push(normalized);
    }

    hosts.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    write_image_hosts(&app, &hosts)
}

#[tauri::command]
pub fn delete_image_host(app: AppHandle, host_id: String) -> Result<Vec<ImageHostConfig>, String> {
    let hosts: Vec<ImageHostConfig> = read_image_hosts(&app)?
        .into_iter()
        .filter(|item| item.id != host_id)
        .collect();

    write_image_hosts(&app, &hosts)
}

#[tauri::command]
pub fn update_backup_settings(
    app: AppHandle,
    backup_root_path: String,
    backup_retention_days: u64,
) -> Result<NoteSettings, String> {
    let current = read_note_settings(&app)?;
    let settings = NoteSettings {
        notes_root_path: current.notes_root_path,
        backup_root_path,
        mail_root_path: current.mail_root_path,
        backup_retention_days,
    };
    let saved = write_note_settings(&app, &settings)?;
    cleanup_expired_backups(&app)?;
    Ok(saved)
}

#[tauri::command]
pub fn list_backups(app: AppHandle, note_path: Option<String>) -> Result<Vec<BackupHistoryItem>, String> {
    prepare_notes_workspace(&app)?;
    let backup_root = backups_root(&app)?;
    let search_root = if let Some(path) = note_path {
        let note = validate_note_path(&app, Path::new(&path))?;
        backup_directory_for_note(&app, &note)?
    } else {
        backup_root
    };

    if !search_root.exists() {
        return Ok(Vec::new());
    }

    let mut items = Vec::new();
    let mut stack = vec![search_root];

    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).map_err(|error| format!("Failed to read backup directory: {error}"))? {
            let entry = entry.map_err(|error| format!("Failed to read backup directory entry: {error}"))?;
            let entry_path = entry.path();

            if entry_path.is_dir() {
                stack.push(entry_path);
                continue;
            }

            if entry_path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
            {
                items.push(backup_item_from_path(&app, &entry_path)?);
            }
        }
    }

    items.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    Ok(items)
}

#[tauri::command]
pub fn read_backup(app: AppHandle, backup_path: String) -> Result<BackupDocument, String> {
    prepare_notes_workspace(&app)?;
    let target = validate_backup_path(&app, Path::new(&backup_path))?;
    let content =
        fs::read_to_string(&target).map_err(|error| format!("Failed to read backup content: {error}"))?;
    let item = backup_item_from_path(&app, &target)?;

    Ok(BackupDocument {
        path: item.path,
        name: item.name,
        note_title: item.note_title,
        note_path: item.note_path,
        content,
        created_at: item.created_at,
    })
}

#[tauri::command]
pub fn restore_backup(app: AppHandle, backup_path: String) -> Result<NoteDocument, String> {
    prepare_notes_workspace(&app)?;
    let target = validate_backup_path(&app, Path::new(&backup_path))?;
    let content =
        fs::read_to_string(&target).map_err(|error| format!("Failed to read backup content: {error}"))?;
    let item = backup_item_from_path(&app, &target)?;
    let note_path = validate_note_path(&app, Path::new(&item.note_path))
        .or_else(|_| Ok::<PathBuf, String>(PathBuf::from(&item.note_path)))?;

    let note_directory = note_directory_from_note_path(&note_path)?;
    let note_id = note_id_from_note_path(&note_path)?;
    ensure_directory_storage(&note_directory)?;

    let mut index = read_directory_index(&note_directory)?;
    if let Some(record) = index.notes.iter_mut().find(|record| record.id == note_id) {
        record.updated_at = Some(
            chrono::Local::now()
                .format("%Y-%m-%d %H:%M:%S")
                .to_string(),
        );
    } else {
        index.notes.push(NoteIndexRecord {
            id: note_id.clone(),
            sync_id: None,
            name: sanitize_note_title(&item.note_title),
            updated_at: Some(
                chrono::Local::now()
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string(),
            ),
        });
    }
    write_directory_index(&note_directory, &index)?;

    fs::write(&note_path, &content).map_err(|error| format!("Failed to restore backup: {error}"))?;

    let (_, _, record) = note_record_for_path(&note_path)?;
    Ok(NoteDocument {
        path: display_path(&note_path),
        title: record.name,
        content,
        updated_at: modified_at(&note_path),
        last_backup_at: latest_backup_time_for_note(&app, &note_path)?,
    })
}

#[tauri::command]
pub async fn sync_notes(app: AppHandle, window: WebviewWindow) -> Result<NoteSyncResult, String> {
    emit_debug_log(&app, "info", "notes.sync.start", "开始同步笔记");
    let root = prepare_notes_workspace(&app)?;
    let (server_url, api_key, last_synced_at_raw) = sync_settings(&app)?;
    let last_synced_at = last_synced_at_raw.as_deref().and_then(parse_rfc3339);
    let client = reqwest::Client::new();
    let checked_at = now_rfc3339();

    emit_sync_progress(&window, "checking", 0, 0, "正在检查本地和远端改动");

    let mut local_files = Vec::new();
    collect_local_sync_files(&app, &root, &mut local_files)?;
    cleanup_expired_deletions(&app, &root)?;
    let mut local_deletions = Vec::new();
    collect_local_deletions(&root, &mut local_deletions)?;

    let remote_files = list_remote_files(&client, &server_url, &api_key).await?;
    emit_debug_log(
        &app,
        "info",
        "notes.sync.checked",
        format!("本地文件 {} 个，远端文件 {} 个", local_files.len(), remote_files.len()),
    );
    let remote_active = remote_files
        .iter()
        .filter(|item| !item.is_deleted)
        .cloned()
        .collect::<Vec<_>>();
    let remote_deleted = remote_files
        .iter()
        .filter(|item| item.is_deleted)
        .cloned()
        .collect::<Vec<_>>();
    let remote_by_id: HashMap<String, ServerFileInfo> = remote_active
        .iter()
        .map(|item| (item.id.clone(), item.clone()))
        .collect();
    let remote_by_path: HashMap<String, ServerFileInfo> = remote_active
        .iter()
        .map(|item| (item.relative_path.clone(), item.clone()))
        .collect();
    let remote_deleted_by_id: HashMap<String, ServerFileInfo> = remote_deleted
        .iter()
        .map(|item| (item.id.clone(), item.clone()))
        .collect();
    let local_by_path: HashMap<String, LocalNoteSyncFile> = local_files
        .iter()
        .map(|item| (item.relative_path.clone(), item.clone()))
        .collect();
    let local_by_sync_id: HashMap<String, LocalNoteSyncFile> = local_files
        .iter()
        .filter_map(|item| item.sync_id.as_ref().map(|sync_id| (sync_id.clone(), item.clone())))
        .collect();

    let download_candidates = remote_active
        .iter()
        .filter(|remote| match local_by_sync_id
            .get(&remote.id)
            .or_else(|| local_by_path.get(&remote.relative_path))
        {
            Some(local) => parse_rfc3339(&remote.last_updated)
                .is_some_and(|remote_at| remote_at > local.modified_at),
            None => {
                let remote_time = parse_rfc3339(&remote.last_updated);
                match (last_synced_at, remote_time) {
                    (Some(last), Some(remote_at)) => remote_at > last,
                    (None, _) => true,
                    _ => false,
                }
            }
        })
        .cloned()
        .collect::<Vec<_>>();

    let upload_candidates = local_files
        .iter()
        .filter(|local| {
            if local
                .sync_id
                .as_ref()
                .is_some_and(|sync_id| remote_deleted_by_id.contains_key(sync_id))
            {
                return false;
            }

            let remote = local
                .sync_id
                .as_ref()
                .and_then(|sync_id| remote_by_id.get(sync_id))
                .or_else(|| remote_by_path.get(&local.relative_path));

            match remote.and_then(|item| parse_rfc3339(&item.last_updated)) {
                Some(remote_at) => local.modified_at > remote_at,
                None => match last_synced_at {
                    Some(last) => local.modified_at > last,
                    None => true,
                },
            }
        })
        .cloned()
        .collect::<Vec<_>>();

    emit_sync_progress(
        &window,
        "checking",
        0,
        download_candidates.len() + upload_candidates.len() + local_deletions.len() + remote_deleted.len(),
        format!(
            "发现远端改动 {} 个，本地改动 {} 个，删除记录 {} 个",
            download_candidates.len(),
            upload_candidates.len(),
            local_deletions.len() + remote_deleted.len()
        ),
    );

    let mut downloaded = 0;
    let mut uploaded = 0;
    let mut skipped = 0;
    let mut deleted = 0;
    let mut downloaded_ids = HashSet::new();

    for deletion in local_deletions.iter() {
        let Some(sync_id) = deletion.sync_id.as_deref() else {
            skipped += 1;
            continue;
        };

        if remote_deleted_by_id.contains_key(sync_id) {
            skipped += 1;
            continue;
        }

        if remote_by_id.contains_key(sync_id) {
            delete_remote_file(&client, &server_url, &api_key, sync_id).await?;
            deleted += 1;
        }
    }

    for remote in remote_deleted.iter() {
        if let Some(local) = local_by_sync_id
            .get(&remote.id)
            .or_else(|| local_by_path.get(&remote.relative_path))
        {
            let directory = note_directory_from_note_path(&local.path)?;
            mark_local_note_deleted(&app, &directory, &local.id)?;
            deleted += 1;
        }
    }

    for (index, remote) in download_candidates.iter().enumerate() {
        let remote_time = parse_rfc3339(&remote.last_updated);
        let local_match = local_by_sync_id
            .get(&remote.id)
            .or_else(|| local_by_path.get(&remote.relative_path));
        if let (Some(local), Some(remote_at)) = (local_match, remote_time) {
            if local.modified_at > remote_at {
                skipped += 1;
                continue;
            }
        }

        emit_sync_progress(
            &window,
            "download",
            index + 1,
            download_candidates.len(),
            format!("正在下载 {}", remote.relative_path),
        );

        let content = download_remote_file(&client, &server_url, &api_key, &remote.id).await?;
        upsert_downloaded_note(&app, remote, &content)?;
        downloaded += 1;
        downloaded_ids.insert(remote.id.clone());
    }

    for (index, local) in upload_candidates.iter().enumerate() {
        if local
            .sync_id
            .as_ref()
            .is_some_and(|sync_id| downloaded_ids.contains(sync_id))
        {
            skipped += 1;
            continue;
        }

        emit_sync_progress(
            &window,
            "upload",
            index + 1,
            upload_candidates.len(),
            format!("正在上传 {}", local.relative_path),
        );

        let remote_id = local
            .sync_id
            .as_ref()
            .and_then(|sync_id| remote_by_id.get(sync_id))
            .or_else(|| remote_by_path.get(&local.relative_path))
            .map(|item| item.id.as_str());
        let uploaded_info = upload_local_file(&client, &server_url, &api_key, local, remote_id).await?;
        let directory = note_directory_from_note_path(&local.path)?;
        update_note_sync_id(&directory, &local.id, &uploaded_info.id)?;
        uploaded += 1;
    }

    let last_synced_at = now_rfc3339();
    write_sync_time(&app, &last_synced_at)?;

    let message = format!("同步完成：下载 {downloaded} 个，上传 {uploaded} 个，删除 {deleted} 个，跳过 {skipped} 个");
    emit_debug_log(&app, "success", "notes.sync.done", message.clone());
    emit_sync_progress(&window, "done", downloaded + uploaded, downloaded + uploaded, &message);

    Ok(NoteSyncResult {
        checked_at,
        last_synced_at,
        download_total: download_candidates.len(),
        upload_total: upload_candidates.len(),
        downloaded,
        uploaded,
        skipped,
        message,
    })
}

#[tauri::command]
pub fn list_notes(app: AppHandle) -> Result<NoteWorkspace, String> {
    let root = prepare_notes_workspace(&app)?;
    let tree = build_tree(&root)?;
    let settings = read_note_settings(&app)?;

    Ok(NoteWorkspace {
        root_path: display_path(&root),
        tree,
        backup_root_path: settings.backup_root_path,
        backup_retention_days: settings.backup_retention_days,
    })
}

#[tauri::command]
pub fn read_note(app: AppHandle, path: String) -> Result<NoteDocument, String> {
    prepare_notes_workspace(&app)?;
    let target = validate_note_path(&app, Path::new(&path))?;
    let content = fs::read_to_string(&target).map_err(|error| format!("Failed to read note: {error}"))?;
    let (_, _, record) = note_record_for_path(&target)?;

    Ok(NoteDocument {
        path: display_path(&target),
        title: record.name,
        content,
        updated_at: modified_at(&target),
        last_backup_at: latest_backup_time_for_note(&app, &target)?,
    })
}

#[tauri::command]
pub fn create_directory(
    app: AppHandle,
    parent_path: Option<String>,
    name: String,
) -> Result<(), String> {
    let root = prepare_notes_workspace(&app)?;
    let base_path = match parent_path {
        Some(path) => validate_directory_path(&app, Path::new(&path))?,
        None => root,
    };

    let target = base_path.join(name.trim());
    fs::create_dir_all(&target).map_err(|error| format!("Failed to create directory: {error}"))?;
    ensure_directory_storage(&target)?;
    Ok(())
}

#[tauri::command]
pub fn create_note(
    app: AppHandle,
    parent_path: Option<String>,
    title: String,
) -> Result<NoteDocument, String> {
    let root = prepare_notes_workspace(&app)?;
    let base_path = match parent_path {
        Some(path) => validate_directory_path(&app, Path::new(&path))?,
        None => root,
    };

    ensure_directory_storage(&base_path)?;
    let mut index = read_directory_index(&base_path)?;
    let note_id = Uuid::new_v4().to_string();
    let note_name = unique_note_name(&index, &title, None);
    let note_path = note_content_path(&base_path, &note_id);
    let initial_content = format!("# {}\n\n", note_name);

    fs::write(&note_path, &initial_content).map_err(|error| format!("Failed to create note: {error}"))?;

    index.notes.push(NoteIndexRecord {
        id: note_id,
        sync_id: None,
        name: note_name.clone(),
        updated_at: modified_at(&note_path),
    });
    write_directory_index(&base_path, &index)?;

    Ok(NoteDocument {
        path: display_path(&note_path),
        title: note_name,
        content: initial_content,
        updated_at: modified_at(&note_path),
        last_backup_at: None,
    })
}

#[tauri::command]
pub fn save_note(app: AppHandle, path: String, content: String) -> Result<NoteSaveResult, String> {
    prepare_notes_workspace(&app)?;
    let target = validate_note_path(&app, Path::new(&path))?;
    fs::write(&target, &content).map_err(|error| format!("Failed to save note: {error}"))?;

    let note_directory = note_directory_from_note_path(&target)?;
    let note_id = note_id_from_note_path(&target)?;
    let mut index = read_directory_index(&note_directory)?;
    if let Some(record) = index.notes.iter_mut().find(|record| record.id == note_id) {
        record.updated_at = modified_at(&target);
    }
    write_directory_index(&note_directory, &index)?;

    let backup_at = create_backup(&app, &target, &content)?;

    Ok(NoteSaveResult {
        updated_at: modified_at(&target),
        last_backup_at: backup_at,
    })
}

#[tauri::command]
pub fn save_note_image(
    app: AppHandle,
    note_path: String,
    file_name: String,
    data: Vec<u8>,
) -> Result<NoteImageAsset, String> {
    prepare_notes_workspace(&app)?;
    let target = validate_note_path(&app, Path::new(&note_path))?;
    let note_directory = note_directory_from_note_path(&target)?;
    let note_id = note_id_from_note_path(&target)?;
    let asset_dir = note_asset_dir(&note_directory, &note_id);

    fs::create_dir_all(&asset_dir).map_err(|error| format!("Failed to create note asset directory: {error}"))?;

    let original_name = sanitize_asset_name(&file_name);
    let stem = Path::new(&original_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("image");
    let extension = Path::new(&original_name)
        .extension()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("png");

    let mut asset_path = asset_dir.join(format!("{stem}.{extension}"));
    if asset_path.exists() {
        asset_path = asset_dir.join(format!(
            "{stem}-{}.{}",
            chrono::Local::now().format("%Y%m%d%H%M%S"),
            extension
        ));
    }

    fs::write(&asset_path, data).map_err(|error| format!("Failed to save note image: {error}"))?;

    Ok(NoteImageAsset {
        file_path: display_path(&asset_path),
        markdown_path: file_uri_from_path(&asset_path),
    })
}

#[tauri::command]
pub fn rename_entry(app: AppHandle, path: String, new_name: String) -> Result<(), String> {
    prepare_notes_workspace(&app)?;
    let target_path = Path::new(&path);

    if let Ok(target_note) = validate_note_path(&app, target_path) {
        let note_directory = note_directory_from_note_path(&target_note)?;
        let note_id = note_id_from_note_path(&target_note)?;
        let mut index = read_directory_index(&note_directory)?;
        let next_name = unique_note_name(&index, &new_name, Some(&note_id));

        if let Some(record) = index.notes.iter_mut().find(|record| record.id == note_id) {
            record.name = next_name;
            record.updated_at = modified_at(&target_note);
        }

        write_directory_index(&note_directory, &index)?;
        return Ok(());
    }

    let target_directory = validate_directory_path(&app, target_path)?;
    let parent = target_directory
        .parent()
        .ok_or_else(|| "Failed to get parent directory".to_string())?
        .to_path_buf();
    let next = parent.join(new_name.trim());
    fs::rename(target_directory, next).map_err(|error| format!("Failed to rename entry: {error}"))?;
    Ok(())
}

#[tauri::command]
pub fn delete_entry(app: AppHandle, path: String) -> Result<(), String> {
    prepare_notes_workspace(&app)?;
    let target_path = Path::new(&path);

    if let Ok(target_note) = validate_note_path(&app, target_path) {
        let note_directory = note_directory_from_note_path(&target_note)?;
        let note_id = note_id_from_note_path(&target_note)?;
        mark_local_note_deleted(&app, &note_directory, &note_id)?;
        return Ok(());
    }

    let target_directory = validate_directory_path(&app, target_path)?;
    fs::remove_dir_all(target_directory).map_err(|error| format!("Failed to delete directory: {error}"))?;
    Ok(())
}
