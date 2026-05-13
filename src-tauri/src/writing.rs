use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

const WRITING_DIR: &str = "writing";
const PROJECTS_DIR: &str = "projects";
const PROJECT_INDEX_FILE: &str = "index.json";
const PROJECT_FILE: &str = "project.json";
const MATERIALS_DIR: &str = "materials";
const ASSETS_DIR: &str = "assets";
const MATERIALS_INDEX_FILE: &str = "index.json";
const MATERIAL_FILE_EXTENSION: &str = "json";

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WritingProjectSummary {
    id: String,
    title: String,
    updated_at: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WritingMaterial {
    id: String,
    kind: String,
    title: String,
    content: String,
    source_url: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WritingOutline {
    id: String,
    title: String,
    #[serde(default)]
    content: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WritingSection {
    id: String,
    title: String,
    content: String,
    selected: bool,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WritingProjectDetail {
    id: String,
    title: String,
    publish_directory_path: Option<String>,
    updated_at: String,
    materials: Vec<WritingMaterial>,
    outlines: Vec<WritingOutline>,
    sections: Vec<WritingSection>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WritingProjectCreateInput {
    title: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WritingProjectSaveInput {
    id: String,
    title: String,
    publish_directory_path: Option<String>,
    outlines: Vec<WritingOutlineInput>,
    sections: Vec<WritingSectionInput>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WritingOutlineInput {
    id: Option<String>,
    title: String,
    content: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WritingSectionInput {
    id: Option<String>,
    title: String,
    content: String,
    selected: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WritingMaterialInput {
    kind: String,
    title: String,
    content: String,
    source_url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WritingMaterialUpdateInput {
    id: String,
    kind: String,
    title: String,
    content: String,
    source_url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WritingPublishInput {
    project_id: String,
    directory_path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WritingPublishResult {
    note_path: String,
    note_title: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WritingImageAsset {
    file_path: String,
    markdown_path: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct WritingProjectIndex {
    projects: Vec<WritingProjectSummary>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct WritingMaterialIndex {
    materials: Vec<WritingMaterialIndexRecord>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct WritingMaterialIndexRecord {
    id: String,
    kind: String,
    title: String,
    source_url: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct WritingProjectFile {
    id: String,
    title: String,
    publish_directory_path: Option<String>,
    updated_at: String,
    outlines: Vec<WritingOutline>,
    sections: Vec<WritingSection>,
}

fn now_string() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
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

fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let app_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Failed to get app data directory: {error}"))?;

    fs::create_dir_all(&app_dir).map_err(|error| format!("Failed to create app data directory: {error}"))?;
    Ok(app_dir)
}

fn writing_root(app: &AppHandle) -> Result<PathBuf, String> {
    let root = app_data_dir(app)?.join(WRITING_DIR);
    fs::create_dir_all(&root).map_err(|error| format!("Failed to create writing root: {error}"))?;
    Ok(root)
}

fn projects_root(app: &AppHandle) -> Result<PathBuf, String> {
    let root = writing_root(app)?.join(PROJECTS_DIR);
    fs::create_dir_all(&root).map_err(|error| format!("Failed to create projects root: {error}"))?;
    Ok(root)
}

fn project_index_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(writing_root(app)?.join(PROJECT_INDEX_FILE))
}

fn project_dir(app: &AppHandle, project_id: &str) -> Result<PathBuf, String> {
    let dir = projects_root(app)?.join(project_id);
    fs::create_dir_all(&dir).map_err(|error| format!("Failed to create project directory: {error}"))?;
    Ok(dir)
}

fn project_file_path(app: &AppHandle, project_id: &str) -> Result<PathBuf, String> {
    Ok(project_dir(app, project_id)?.join(PROJECT_FILE))
}

fn materials_dir(app: &AppHandle, project_id: &str) -> Result<PathBuf, String> {
    let dir = project_dir(app, project_id)?.join(MATERIALS_DIR);
    fs::create_dir_all(&dir).map_err(|error| format!("Failed to create materials directory: {error}"))?;
    Ok(dir)
}

fn assets_dir(app: &AppHandle, project_id: &str) -> Result<PathBuf, String> {
    let dir = project_dir(app, project_id)?.join(ASSETS_DIR);
    fs::create_dir_all(&dir).map_err(|error| format!("Failed to create writing asset directory: {error}"))?;
    Ok(dir)
}

fn materials_index_path(app: &AppHandle, project_id: &str) -> Result<PathBuf, String> {
    Ok(materials_dir(app, project_id)?.join(MATERIALS_INDEX_FILE))
}

fn material_file_path(app: &AppHandle, project_id: &str, material_id: &str) -> Result<PathBuf, String> {
    Ok(materials_dir(app, project_id)?.join(format!("{material_id}.{MATERIAL_FILE_EXTENSION}")))
}

fn ensure_materials_index(app: &AppHandle, project_id: &str) -> Result<(), String> {
    let path = materials_index_path(app, project_id)?;
    if !path.exists() {
        let content = serde_json::to_string_pretty(&WritingMaterialIndex::default())
            .map_err(|error| format!("Failed to serialize materials index: {error}"))?;
        fs::write(path, content).map_err(|error| format!("Failed to create materials index: {error}"))?;
    }
    Ok(())
}

fn read_project_index(app: &AppHandle) -> Result<WritingProjectIndex, String> {
    let path = project_index_path(app)?;
    if !path.exists() {
        return Ok(WritingProjectIndex::default());
    }

    let content = fs::read_to_string(path).map_err(|error| format!("Failed to read project index: {error}"))?;
    serde_json::from_str(&content).map_err(|error| format!("Failed to parse project index: {error}"))
}

fn write_project_index(app: &AppHandle, index: &WritingProjectIndex) -> Result<(), String> {
    let path = project_index_path(app)?;
    let content = serde_json::to_string_pretty(index)
        .map_err(|error| format!("Failed to serialize project index: {error}"))?;
    fs::write(path, content).map_err(|error| format!("Failed to save project index: {error}"))
}

fn read_project_file(app: &AppHandle, project_id: &str) -> Result<WritingProjectFile, String> {
    let path = project_file_path(app, project_id)?;
    let content = fs::read_to_string(path).map_err(|error| format!("Failed to read writing project: {error}"))?;
    serde_json::from_str(&content).map_err(|error| format!("Failed to parse writing project: {error}"))
}

fn write_project_file(app: &AppHandle, project: &WritingProjectFile) -> Result<(), String> {
    let path = project_file_path(app, &project.id)?;
    let content = serde_json::to_string_pretty(project)
        .map_err(|error| format!("Failed to serialize writing project: {error}"))?;
    fs::write(path, content).map_err(|error| format!("Failed to save writing project: {error}"))
}

fn read_material_index(app: &AppHandle, project_id: &str) -> Result<WritingMaterialIndex, String> {
    ensure_materials_index(app, project_id)?;
    let path = materials_index_path(app, project_id)?;
    let content = fs::read_to_string(path).map_err(|error| format!("Failed to read materials index: {error}"))?;
    serde_json::from_str(&content).map_err(|error| format!("Failed to parse materials index: {error}"))
}

fn write_material_index(app: &AppHandle, project_id: &str, index: &WritingMaterialIndex) -> Result<(), String> {
    ensure_materials_index(app, project_id)?;
    let path = materials_index_path(app, project_id)?;
    let content = serde_json::to_string_pretty(index)
        .map_err(|error| format!("Failed to serialize materials index: {error}"))?;
    fs::write(path, content).map_err(|error| format!("Failed to save materials index: {error}"))
}

fn read_material_file(app: &AppHandle, project_id: &str, material_id: &str) -> Result<WritingMaterial, String> {
    let path = material_file_path(app, project_id, material_id)?;
    let content = fs::read_to_string(path).map_err(|error| format!("Failed to read material file: {error}"))?;
    serde_json::from_str(&content).map_err(|error| format!("Failed to parse material file: {error}"))
}

fn write_material_file(app: &AppHandle, project_id: &str, material: &WritingMaterial) -> Result<(), String> {
    let path = material_file_path(app, project_id, &material.id)?;
    let content = serde_json::to_string_pretty(material)
        .map_err(|error| format!("Failed to serialize material file: {error}"))?;
    fs::write(path, content).map_err(|error| format!("Failed to save material file: {error}"))
}

fn read_materials(app: &AppHandle, project_id: &str) -> Result<Vec<WritingMaterial>, String> {
    let mut index = read_material_index(app, project_id)?;
    index.materials.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));

    let mut materials = Vec::with_capacity(index.materials.len());
    for item in index.materials {
        if let Ok(material) = read_material_file(app, project_id, &item.id) {
            materials.push(material);
        }
    }

    Ok(materials)
}

fn sanitize_project_title(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        format!("鍐欎綔宸ョ▼ {}", chrono::Local::now().format("%m-%d %H:%M"))
    } else {
        trimmed.to_string()
    }
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

fn normalize_outline(input: WritingOutlineInput) -> WritingOutline {
    let content = input.content.trim().to_string();
    let title = if input.title.trim().is_empty() {
        let sample = content.chars().take(36).collect::<String>();
        if sample.is_empty() {
            String::new()
        } else {
            sample
        }
    } else {
        input.title.trim().to_string()
    };

    WritingOutline {
        id: input.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        title,
        content,
    }
}

fn normalize_section(input: WritingSectionInput) -> WritingSection {
    WritingSection {
        id: input.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        title: input.title.trim().to_string(),
        content: input.content,
        selected: input.selected,
    }
}

fn load_project_detail(app: &AppHandle, project_id: &str) -> Result<WritingProjectDetail, String> {
    let project = read_project_file(app, project_id)?;
    let materials = read_materials(app, project_id)?;

    Ok(WritingProjectDetail {
        id: project.id,
        title: project.title,
        publish_directory_path: project.publish_directory_path,
        updated_at: project.updated_at,
        materials,
        outlines: project.outlines,
        sections: project.sections,
    })
}

fn update_project_summary(index: &mut WritingProjectIndex, project: &WritingProjectFile) {
    let summary = WritingProjectSummary {
        id: project.id.clone(),
        title: project.title.clone(),
        updated_at: project.updated_at.clone(),
    };

    if let Some(existing) = index.projects.iter_mut().find(|item| item.id == summary.id) {
        *existing = summary;
    } else {
        index.projects.push(summary);
    }

    index.projects.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
}

fn selected_sections_to_markdown(project: &WritingProjectDetail) -> String {
    let mut lines = vec![format!("# {}", project.title.trim())];
    let sections = project
        .sections
        .iter()
        .filter(|section| section.selected)
        .collect::<Vec<_>>();

    for section in sections {
        if !section.title.trim().is_empty() {
            lines.push(String::new());
            lines.push(format!("## {}", section.title.trim()));
        }

        if !section.content.trim().is_empty() {
            lines.push(String::new());
            lines.push(section.content.trim().to_string());
        }
    }

    lines.join("\n")
}

#[tauri::command]
pub fn list_writing_projects(app: AppHandle) -> Result<Vec<WritingProjectSummary>, String> {
    let mut index = read_project_index(&app)?;
    index.projects.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    Ok(index.projects)
}

#[tauri::command]
pub fn create_writing_project(
    app: AppHandle,
    input: Option<WritingProjectCreateInput>,
) -> Result<WritingProjectDetail, String> {
    let title = sanitize_project_title(
        input
            .as_ref()
            .and_then(|item| item.title.as_deref())
            .unwrap_or(""),
    );
    let now = now_string();
    let project = WritingProjectFile {
        id: Uuid::new_v4().to_string(),
        title,
        publish_directory_path: None,
        updated_at: now.clone(),
        outlines: vec![WritingOutline {
            id: Uuid::new_v4().to_string(),
            title: String::new(),
            content: String::new(),
        }],
        sections: vec![WritingSection {
            id: Uuid::new_v4().to_string(),
            title: "姝ｆ枃".to_string(),
            content: String::new(),
            selected: true,
        }],
    };

    write_project_file(&app, &project)?;
    write_material_index(&app, &project.id, &WritingMaterialIndex::default())?;

    let mut index = read_project_index(&app)?;
    update_project_summary(&mut index, &project);
    write_project_index(&app, &index)?;

    load_project_detail(&app, &project.id)
}

#[tauri::command]
pub fn get_writing_project(app: AppHandle, project_id: String) -> Result<WritingProjectDetail, String> {
    load_project_detail(&app, &project_id)
}

#[tauri::command]
pub fn save_writing_project(app: AppHandle, project: WritingProjectSaveInput) -> Result<WritingProjectDetail, String> {
    let current = read_project_file(&app, &project.id)?;
    let next = WritingProjectFile {
        id: current.id,
        title: sanitize_project_title(&project.title),
        publish_directory_path: project
            .publish_directory_path
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        updated_at: now_string(),
        outlines: project
            .outlines
            .into_iter()
            .map(normalize_outline)
            .filter(|item| !item.content.is_empty())
            .collect(),
        sections: project.sections.into_iter().map(normalize_section).collect(),
    };

    write_project_file(&app, &next)?;

    let mut index = read_project_index(&app)?;
    update_project_summary(&mut index, &next);
    write_project_index(&app, &index)?;

    load_project_detail(&app, &next.id)
}

#[tauri::command]
pub fn save_writing_image(
    app: AppHandle,
    project_id: String,
    file_name: String,
    data: Vec<u8>,
) -> Result<WritingImageAsset, String> {
    let _project = read_project_file(&app, &project_id)?;
    let asset_dir = assets_dir(&app, &project_id)?;
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

    let asset_path = asset_dir.join(format!(
        "{}-{}.{}",
        stem,
        Uuid::new_v4().simple(),
        extension
    ));

    fs::write(&asset_path, data).map_err(|error| format!("Failed to save writing image: {error}"))?;

    Ok(WritingImageAsset {
        file_path: display_path(&asset_path),
        markdown_path: file_uri_from_path(&asset_path),
    })
}

#[tauri::command]
pub fn delete_writing_project(app: AppHandle, project_id: String) -> Result<Vec<WritingProjectSummary>, String> {
    let project_path = project_dir(&app, &project_id)?;
    if project_path.exists() {
        fs::remove_dir_all(&project_path).map_err(|error| format!("Failed to remove writing project: {error}"))?;
    }

    let mut index = read_project_index(&app)?;
    index.projects.retain(|item| item.id != project_id);
    write_project_index(&app, &index)?;
    Ok(index.projects)
}

#[tauri::command]
pub fn add_writing_material(
    app: AppHandle,
    project_id: String,
    material: WritingMaterialInput,
) -> Result<WritingProjectDetail, String> {
    let now = now_string();
    let title = if material.title.trim().is_empty() {
        let sample = material
            .content
            .trim()
            .chars()
            .take(18)
            .collect::<String>();
        if sample.is_empty() {
            "未命名素材".to_string()
        } else {
            sample
        }
    } else {
        material.title.trim().to_string()
    };

    let entry = WritingMaterial {
        id: Uuid::new_v4().to_string(),
        kind: material.kind.trim().to_string(),
        title: title.clone(),
        content: material.content,
        source_url: material
            .source_url
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        created_at: now.clone(),
        updated_at: now.clone(),
    };

    write_material_file(&app, &project_id, &entry)?;

    let mut index = read_material_index(&app, &project_id)?;
    index.materials.push(WritingMaterialIndexRecord {
        id: entry.id.clone(),
        kind: entry.kind.clone(),
        title,
        source_url: entry.source_url.clone(),
        created_at: entry.created_at.clone(),
        updated_at: entry.updated_at.clone(),
    });
    index.materials.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    write_material_index(&app, &project_id, &index)?;

    let mut project = read_project_file(&app, &project_id)?;
    project.updated_at = now_string();
    write_project_file(&app, &project)?;

    let mut project_index = read_project_index(&app)?;
    update_project_summary(&mut project_index, &project);
    write_project_index(&app, &project_index)?;

    load_project_detail(&app, &project_id)
}

#[tauri::command]
pub fn update_writing_material(
    app: AppHandle,
    project_id: String,
    material: WritingMaterialUpdateInput,
) -> Result<WritingProjectDetail, String> {
    let current = read_material_file(&app, &project_id, &material.id)?;
    let now = now_string();
    let title = if material.title.trim().is_empty() {
        let sample = material
            .content
            .trim()
            .chars()
            .take(18)
            .collect::<String>();
        if sample.is_empty() {
            current.title.clone()
        } else {
            sample
        }
    } else {
        material.title.trim().to_string()
    };

    let next = WritingMaterial {
        id: current.id.clone(),
        kind: material.kind.trim().to_string(),
        title: title.clone(),
        content: material.content,
        source_url: material
            .source_url
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        created_at: current.created_at,
        updated_at: now,
    };

    write_material_file(&app, &project_id, &next)?;

    let mut index = read_material_index(&app, &project_id)?;
    if let Some(record) = index.materials.iter_mut().find(|item| item.id == next.id) {
        record.kind = next.kind.clone();
        record.title = next.title.clone();
        record.source_url = next.source_url.clone();
        record.updated_at = next.updated_at.clone();
    }
    index.materials.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    write_material_index(&app, &project_id, &index)?;

    let mut project = read_project_file(&app, &project_id)?;
    project.updated_at = now_string();
    write_project_file(&app, &project)?;

    let mut project_index = read_project_index(&app)?;
    update_project_summary(&mut project_index, &project);
    write_project_index(&app, &project_index)?;

    load_project_detail(&app, &project_id)
}

#[tauri::command]
pub fn delete_writing_material(
    app: AppHandle,
    project_id: String,
    material_id: String,
) -> Result<WritingProjectDetail, String> {
    let file_path = material_file_path(&app, &project_id, &material_id)?;
    if file_path.exists() {
        fs::remove_file(&file_path).map_err(|error| format!("Failed to delete material file: {error}"))?;
    }

    let mut index = read_material_index(&app, &project_id)?;
    index.materials.retain(|item| item.id != material_id);
    write_material_index(&app, &project_id, &index)?;

    let mut project = read_project_file(&app, &project_id)?;
    project.updated_at = now_string();
    write_project_file(&app, &project)?;

    let mut project_index = read_project_index(&app)?;
    update_project_summary(&mut project_index, &project);
    write_project_index(&app, &project_index)?;

    load_project_detail(&app, &project_id)
}

#[tauri::command]
pub fn publish_writing_project(app: AppHandle, input: WritingPublishInput) -> Result<WritingPublishResult, String> {
    let project = load_project_detail(&app, &input.project_id)?;
    if !project.sections.iter().any(|section| section.selected) {
        return Err("Please select at least one section before publishing.".to_string());
    }

    let content = selected_sections_to_markdown(&project);
    let note = crate::notes::create_note(app.clone(), Some(input.directory_path.clone()), project.title.clone())?;
    crate::notes::save_note(app.clone(), note.path.clone(), content)?;

    let next = WritingProjectFile {
        id: project.id.clone(),
        title: project.title.clone(),
        publish_directory_path: Some(display_path(Path::new(&input.directory_path))),
        updated_at: now_string(),
        outlines: project.outlines,
        sections: project.sections,
    };
    write_project_file(&app, &next)?;

    let mut index = read_project_index(&app)?;
    update_project_summary(&mut index, &next);
    write_project_index(&app, &index)?;

    Ok(WritingPublishResult {
        note_path: note.path,
        note_title: note.title,
    })
}
