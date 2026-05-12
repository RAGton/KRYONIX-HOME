use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderContext {
    pub folder_path: String,
    pub folder_name: String,
    pub folder_kind: String,
    pub dominant_categories: Vec<String>,
    pub project_markers: Vec<String>,
    pub neighbor_extensions: Vec<String>,
    pub neighbor_keywords: Vec<String>,
    pub warnings: Vec<String>,
    // Novas propriedades solicitadas:
    pub folder_signals: Vec<String>,
    pub dominant_category: Option<String>,
    pub is_project: bool,
    pub is_vault: bool,
    pub is_inbox: bool,
    pub is_review_bucket: bool,
    pub is_staging: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextProfile {
    pub is_inside_codebase: bool,
    pub sibling_categories: Vec<String>,
    pub folder_context: Option<FolderContext>,
}

pub fn analyze_folder_context(folder_path: &Path) -> FolderContext {
    let folder_name = folder_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let folder_path_str = folder_path.to_string_lossy().to_string();
    let folder_path_lower = folder_path_str.to_lowercase();

    let mut project_markers = Vec::new();
    for marker in crate::project::PROJECT_MARKERS {
        if folder_path.join(marker).exists() {
            project_markers.push(marker.to_string());
        }
    }

    let mut folder_kind = "unknown".to_string();
    let mut warnings = Vec::new();

    if !project_markers.is_empty() {
        folder_kind = "project".to_string();
        if folder_path_lower.contains("/downloads") {
            warnings.push("Projeto localizado na pasta Downloads (não ideal)".to_string());
        } else if folder_path_lower.contains("/music") || folder_path_lower.contains("/músicas") {
            warnings.push("Projeto localizado na pasta Músicas (não ideal)".to_string());
        } else if folder_path_lower.contains("/pictures") || folder_path_lower.contains("/imagens")
        {
            warnings.push("Projeto localizado na pasta Imagens (não ideal)".to_string());
        } else if folder_path_lower.contains("/videos") || folder_path_lower.contains("/vídeos") {
            warnings.push("Projeto localizado na pasta Vídeos (não ideal)".to_string());
        }
    } else if folder_path_lower.contains("/downloads") {
        folder_kind = "downloads".to_string();
    } else if folder_path_lower.contains("/obsidian") || folder_path_lower.contains("/vault") {
        folder_kind = "vault".to_string();
    } else if folder_path_lower.contains("/documentos") || folder_path_lower.contains("/documents")
    {
        folder_kind = "documents".to_string();
    }

    let mut neighbor_extensions = Vec::new();
    let mut neighbor_keywords = Vec::new();
    let mut dominant_categories = Vec::new();

    if let Ok(entries) = std::fs::read_dir(folder_path) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    let path = entry.path();
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        neighbor_extensions.push(ext.to_lowercase());
                    }
                    let filename = entry.file_name().to_string_lossy().to_lowercase();
                    if filename.contains("nix") || filename.contains("flake") {
                        dominant_categories.push("projetos.nixos".to_string());
                    }
                    if filename.contains("cargo") || filename.contains("rust") {
                        dominant_categories.push("estudos.rust".to_string());
                    }
                    if filename.contains("kryonix") {
                        dominant_categories.push("projetos.kryonix".to_string());
                    }

                    // Simple sibling keywords collection
                    for kw in &[
                        "comprovante",
                        "boleto",
                        "contrato",
                        "nota",
                        "fatura",
                        "pix",
                        "aula",
                        "curso",
                    ] {
                        if filename.contains(kw) {
                            neighbor_keywords.push(kw.to_string());
                        }
                    }
                }
            }
        }
    }

    neighbor_extensions.sort();
    neighbor_extensions.dedup();
    neighbor_keywords.sort();
    neighbor_keywords.dedup();
    dominant_categories.sort();
    dominant_categories.dedup();

    let mut folder_signals = Vec::new();
    for ext in &neighbor_extensions {
        folder_signals.push(format!("ext:{}", ext));
    }
    for kw in &neighbor_keywords {
        folder_signals.push(format!("kw:{}", kw));
    }
    for marker in &project_markers {
        folder_signals.push(format!("marker:{}", marker));
    }

    let dominant_category = dominant_categories.first().cloned();
    let is_project = folder_kind == "project";
    let is_vault = folder_kind == "vault"
        || folder_name.to_lowercase().contains(".obsidian")
        || folder_path_str.to_lowercase().contains("obsidian vault");
    let is_inbox = folder_kind == "downloads"
        || folder_name.to_lowercase().contains("inbox")
        || folder_name.to_lowercase().contains("downloads");
    let is_review_bucket = folder_name.to_lowercase().contains("revisar")
        || folder_name.to_lowercase().contains("baixa_confianca")
        || folder_name.to_lowercase().contains("conflitos");
    let is_staging = folder_name.to_lowercase().contains("staging")
        || folder_name.to_lowercase().contains("00_inbox");

    FolderContext {
        folder_path: folder_path_str,
        folder_name,
        folder_kind,
        dominant_categories,
        project_markers,
        neighbor_extensions,
        neighbor_keywords,
        warnings,
        folder_signals,
        dominant_category,
        is_project,
        is_vault,
        is_inbox,
        is_review_bucket,
        is_staging,
    }
}

pub fn analyze_file_context(path: &Path) -> ContextProfile {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home/rocha"));
    let mut is_inside_codebase = false;
    let mut sibling_categories = Vec::new();
    let mut folder_context = None;

    // Verifica se algum diretório pai contém marcadores de projeto (dentro de home)
    let mut current = path.parent();
    while let Some(p) = current {
        if !p.starts_with(&home) {
            break;
        }
        for marker in crate::project::PROJECT_MARKERS {
            if p.join(marker).exists() {
                is_inside_codebase = true;
                break;
            }
        }
        if is_inside_codebase {
            break;
        }
        current = p.parent();
    }

    // Varre arquivos irmãos imediatos para herdar contexto de categoria
    if let Some(parent) = path.parent() {
        let f_context = analyze_folder_context(parent);
        sibling_categories = f_context.dominant_categories.clone();
        folder_context = Some(f_context);
    }

    ContextProfile {
        is_inside_codebase,
        sibling_categories,
        folder_context,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::tempdir;

    #[test]
    fn test_context_detects_sibling_nixos_category() {
        let dir = tempdir().expect("Failed to create temp dir");
        let flake_path = dir.path().join("flake.nix");
        File::create(&flake_path).expect("Failed to create mock flake.nix");

        let file_under_test = dir.path().join("study_notes.txt");

        let context = analyze_file_context(&file_under_test);
        assert!(
            context
                .sibling_categories
                .contains(&"projetos.nixos".to_string()),
            "Should have matched projetos.nixos due to sibling flake.nix"
        );
    }

    #[test]
    fn test_context_detects_sibling_rust_category() {
        let dir = tempdir().expect("Failed to create temp dir");
        let cargo_path = dir.path().join("Cargo.toml");
        File::create(&cargo_path).expect("Failed to create mock Cargo.toml");

        let file_under_test = dir.path().join("test_notes.md");

        let context = analyze_file_context(&file_under_test);
        assert!(
            context
                .sibling_categories
                .contains(&"estudos.rust".to_string()),
            "Should have matched estudos.rust due to sibling Cargo.toml"
        );
    }
}
