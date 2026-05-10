use std::path::Path;

#[derive(Debug, Clone)]
pub struct ContextProfile {
    pub is_inside_codebase: bool,
    pub sibling_categories: Vec<String>,
}

pub fn analyze_file_context(path: &Path) -> ContextProfile {
    let mut is_inside_codebase = false;
    let mut sibling_categories = Vec::new();

    // Verifica se algum diretório pai contém marcadores de projeto
    let mut current = path.parent();
    while let Some(p) = current {
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
        if let Ok(entries) = std::fs::read_dir(parent) {
            for entry in entries.flatten() {
                let filename = entry.file_name().to_string_lossy().to_lowercase();
                if filename.contains("nix") || filename.contains("flake") {
                    sibling_categories.push("projetos.nixos".to_string());
                }
                if filename.contains("cargo") || filename.contains("rust") {
                    sibling_categories.push("estudos.rust".to_string());
                }
                if filename.contains("kryonix") {
                    sibling_categories.push("projetos.kryonix".to_string());
                }
            }
        }
    }

    sibling_categories.dedup();

    ContextProfile {
        is_inside_codebase,
        sibling_categories,
    }
}
