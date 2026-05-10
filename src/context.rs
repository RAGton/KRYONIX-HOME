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

    #[test]
    fn test_context_detects_ancestor_codebase() {
        let dir = tempdir().expect("Failed to create temp dir");
        // Create a subfolder representing nested project directories
        let src_dir = dir.path().join("src").join("nested");
        fs::create_dir_all(&src_dir).expect("Failed to create nested dirs");

        // Create a project marker in the ancestor dir
        let git_marker = dir.path().join(".git");
        fs::create_dir(&git_marker).expect("Failed to create mock .git folder");

        let file_under_test = src_dir.join("main.rs");

        let context = analyze_file_context(&file_under_test);
        assert!(
            context.is_inside_codebase,
            "Should have detected that file is inside a codebase due to ancestor .git marker"
        );
    }
}
