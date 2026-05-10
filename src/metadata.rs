use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Metadados coletados de um arquivo durante o scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: String,
    pub filename: String,
    pub extension: String,
    pub mime: String,
    pub size_bytes: u64,
    pub modified_at: Option<String>,
    pub is_dir: bool,
    pub is_file: bool,
    pub is_symlink: bool,
    pub is_hidden: bool,
    pub is_project_member: bool,
    pub project_root: Option<String>,
    pub source_zone: Option<String>,
    pub readable: bool,
    pub content_sampled: bool,
    pub metadata_only: bool,
    pub protected_reason: Option<String>,
    pub warnings: Vec<String>,
    pub status: FileStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FileStatus {
    Analyzed,
    Ignored,
    Error,
}

/// Detecta se um diretório/arquivo é oculto.
pub fn is_hidden_path(path: &Path) -> bool {
    if let Some(name) = path.file_name() {
        if name.to_string_lossy().starts_with('.') {
            return true;
        }
    }
    if let Some(parent) = path.parent() {
        if parent != path && parent.as_os_str() != "/" && parent.as_os_str() != "" {
            return is_hidden_path(parent);
        }
    }
    false
}

/// Detecta a zona de origem do arquivo/diretório baseado no caminho.
pub fn detect_source_zone(path: &Path, home: &Path) -> String {
    let path_str = path.to_string_lossy().to_lowercase();
    let home_str = home.to_string_lossy().to_lowercase();

    if !path_str.starts_with(&home_str) {
        return "unknown".to_string();
    }

    let relative = match path.strip_prefix(home) {
        Ok(r) => r.to_string_lossy().to_lowercase(),
        Err(_) => return "unknown".to_string(),
    };

    let parts: Vec<&str> = relative.split('/').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return "home_root".to_string();
    }

    match parts[0] {
        "downloads" => "downloads".to_string(),
        "desktop" | "área de trabalho" => "desktop".to_string(),
        "documentos" | "documents" => {
            if parts.contains(&".obsidian") || parts.contains(&"obsidian") {
                "vault".to_string()
            } else if parts.contains(&"notebooks") {
                "notebooks".to_string()
            } else {
                "documents".to_string()
            }
        }
        "imagens" | "pictures" => "pictures".to_string(),
        "vídeos" | "videos" => "videos".to_string(),
        "músicas" | "music" => "music".to_string(),
        "projetos" | "projects" => "projects".to_string(),
        _ => {
            if parts[0].starts_with('.') {
                "hidden_config".to_string()
            } else {
                "unknown".to_string()
            }
        }
    }
}

/// Identifica se o caminho é protegido e por qual motivo.
pub fn is_protected_path(path: &Path) -> Option<String> {
    let path_str = path.to_string_lossy().to_lowercase();
    if path_str.contains("/.ssh/") || path_str.ends_with("/.ssh") {
        return Some("sensitive .ssh directory".to_string());
    }
    if path_str.contains("/.gnupg/") || path_str.ends_with("/.gnupg") {
        return Some("sensitive .gnupg directory".to_string());
    }
    if path_str.contains("/.config/") || path_str.ends_with("/.config") {
        return Some("system/app config directory".to_string());
    }
    if path_str.contains("/.local/") || path_str.ends_with("/.local") {
        return Some("local application state directory".to_string());
    }
    if path_str.contains("/.cache/") || path_str.ends_with("/.cache") {
        return Some("cached files".to_string());
    }
    if path_str.contains("/.mozilla/") || path_str.ends_with("/.mozilla") {
        return Some("browser files".to_string());
    }
    if path_str.contains("/.var/") || path_str.ends_with("/.var") {
        return Some("flatpak and runtime state".to_string());
    }
    if path_str.contains("/.npm/") || path_str.ends_with("/.npm") {
        return Some("node package cache".to_string());
    }
    if path_str.contains("/.cargo/") || path_str.ends_with("/.cargo") {
        return Some("cargo registry/cache".to_string());
    }
    if path_str.contains("/.rustup/") || path_str.ends_with("/.rustup") {
        return Some("rustup toolchains".to_string());
    }
    if path_str.contains("/.env") || path_str.ends_with(".env") {
        return Some("environment variable secrets file".to_string());
    }
    if path_str.contains("/brain.env") || path_str.ends_with("brain.env") {
        return Some("kryonix API credentials".to_string());
    }
    if path_str.contains("/neo4j.env") || path_str.ends_with("neo4j.env") {
        return Some("neo4j database credentials".to_string());
    }
    if path_str.contains("private_key")
        || path_str.contains("id_rsa")
        || path_str.contains("id_ed25519")
        || path_str.ends_with(".pem")
        || path_str.ends_with(".key")
    {
        return Some("cryptographic private key".to_string());
    }
    None
}

/// Coleta metadados de um arquivo de forma segura.
pub fn collect(path: &Path, is_symlink: bool) -> FileMetadata {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mime = mime_guess::from_path(path)
        .first_or_octet_stream()
        .to_string();

    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home/rocha"));
    let source_zone = Some(detect_source_zone(path, &home));
    let is_hidden = is_hidden_path(path);

    let protected_reason = is_protected_path(path);
    let metadata_only = protected_reason.is_some();

    let (size_bytes, modified_at, readable, is_dir, is_file) = match std::fs::symlink_metadata(path)
    {
        Ok(meta) => {
            let mtime_str = meta
                .modified()
                .ok()
                .map(|t| DateTime::<Utc>::from(t).to_rfc3339());

            let is_d = meta.is_dir();
            let is_f = meta.is_file();

            // Check readability
            let readable = if is_f && !metadata_only {
                std::fs::File::open(path).is_ok()
            } else {
                false
            };

            (meta.len(), mtime_str, readable, is_d, is_f)
        }
        Err(_) => (0, None, false, path.is_dir(), path.is_file()),
    };

    let mut warnings = Vec::new();
    if metadata_only {
        warnings.push("Protected path - metadata only scan".to_string());
    }

    // Detect if inside a project codebase
    let mut is_project_member = false;
    let mut project_root = None;
    let mut current = path.parent();
    while let Some(p) = current {
        for marker in crate::project::PROJECT_MARKERS {
            if p.join(marker).exists() {
                is_project_member = true;
                project_root = Some(p.to_string_lossy().to_string());
                break;
            }
        }
        if is_project_member {
            break;
        }
        current = p.parent();
    }

    FileMetadata {
        path: path.to_string_lossy().to_string(),
        filename,
        extension,
        mime,
        size_bytes,
        modified_at,
        is_dir,
        is_file,
        is_symlink,
        is_hidden,
        is_project_member,
        project_root,
        source_zone,
        readable,
        content_sampled: false,
        metadata_only,
        protected_reason,
        warnings,
        status: if is_symlink {
            FileStatus::Ignored
        } else {
            FileStatus::Analyzed
        },
    }
}
