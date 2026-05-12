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
    pub content: Option<crate::content::ContentProfile>,
    pub context: Option<crate::context::ContextProfile>,
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

    // 1. Checagem por componentes de diretório sensíveis
    for component in path.components() {
        if let std::path::Component::Normal(name) = component {
            let name_str = name.to_string_lossy().to_lowercase();
            match name_str.as_str() {
                ".ssh" => return Some("sensitive .ssh directory".to_string()),
                ".gnupg" => return Some("sensitive .gnupg directory".to_string()),
                ".config" => return Some("system/app config directory".to_string()),
                ".local" => return Some("local application state directory".to_string()),
                ".cache" => return Some("cached files".to_string()),
                ".mozilla" | ".thunderbird" => return Some("browser profile files".to_string()),
                ".var" => return Some("flatpak and runtime state".to_string()),
                ".npm" | ".cargo" | ".rustup" | ".node-gyp" | ".electron" => {
                    return Some("toolchain/package manager files".to_string())
                }
                ".gnome" | ".kde" | ".dbus" | ".pki" | ".password-store" => {
                    return Some("desktop/security secrets".to_string())
                }
                _ => {}
            }
        }
    }

    // 2. Checagem por nomes de arquivos e padrões
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();

    if filename == ".env"
        || filename.ends_with(".env")
        || filename == "brain.env"
        || filename == "neo4j.env"
    {
        return Some("environment variable secrets file".to_string());
    }

    if filename.contains("private_key")
        || filename.contains("id_rsa")
        || filename.contains("id_ed25519")
        || filename.contains("id_ecdsa")
        || filename.ends_with(".pem")
        || filename.ends_with(".key")
        || filename.ends_with(".secret")
        || filename.ends_with(".token")
    {
        return Some("cryptographic private key or secret token".to_string());
    }

    // 3. Bloqueio extra por substring no path completo (backup safety)
    if path_str.contains("/.ssh")
        || path_str.contains("/.gnupg")
        || path_str.contains("/.config")
        || path_str.contains("/.local/share/kryonix")
        || path_str.contains("/.local/state/kryonix")
    {
        return Some("protected system or hidden path".to_string());
    }

    None
}

/// Coleta metadados de um arquivo de forma segura.
pub fn collect(path: &Path, content_aware: bool) -> FileMetadata {
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

    // Se o usuário pediu content_aware mas o path é protegido, mantemos metadata_only por segurança
    let (size_bytes, modified_at, readable, is_dir, is_file, is_symlink) =
        match std::fs::symlink_metadata(path) {
            Ok(meta) => {
                let mtime_str = meta
                    .modified()
                    .ok()
                    .map(|t| DateTime::<Utc>::from(t).to_rfc3339());

                let is_d = meta.is_dir();
                let is_f = meta.is_file();
                let is_s = meta.file_type().is_symlink();

                // Check readability
                let readable = if is_f && !metadata_only {
                    std::fs::File::open(path).is_ok()
                } else {
                    false
                };

                (meta.len(), mtime_str, readable, is_d, is_f, is_s)
            }
            Err(_) => (0, None, false, path.is_dir(), path.is_file(), false),
        };

    let mut warnings = Vec::new();
    if metadata_only {
        warnings.push("Protected path - metadata only scan".to_string());
    }

    // Detect if inside a project codebase (restricted to remain inside user home)
    let mut is_project_member = false;
    let mut project_root = None;
    let mut current = path.parent();
    while let Some(p) = current {
        if !p.starts_with(&home) {
            break;
        }
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

    // Amostragem de conteúdo se solicitado e seguro
    let content_sampled = content_aware && !metadata_only && readable && is_file && size_bytes > 0;

    let content_profile = if content_sampled {
        crate::content::analyze_file_content(path)
    } else {
        None
    };

    let context_profile = if content_aware && !is_dir && !metadata_only {
        Some(crate::context::analyze_file_context(path))
    } else {
        None
    };

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
        content_sampled,
        metadata_only,
        protected_reason,
        warnings,
        content: content_profile,
        context: context_profile,
        status: if is_symlink {
            FileStatus::Ignored
        } else {
            FileStatus::Analyzed
        },
    }
}
