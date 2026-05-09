use serde::{Deserialize, Serialize};
use std::path::Path;

/// Marcadores que indicam a raiz de um projeto de software.
pub const PROJECT_MARKERS: &[&str] = &[
    ".git",
    "flake.nix",
    "Cargo.toml",
    "pyproject.toml",
    "package.json",
    "go.mod",
    "Makefile",
    "README.md",
    "deno.json",
    "pnpm-lock.yaml",
    "yarn.lock",
];

/// Diretórios internos de projeto que devem ser ignorados.
pub const PROJECT_IGNORED_DIRS: &[&str] = &[
    "build",
    "dist",
    "target",
    "node_modules",
    ".venv",
    "__pycache__",
    "localpycs",
    ".git",
    "vendor",
    "result",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectCandidate {
    pub root_path: String,
    pub name: String,
    pub markers: Vec<String>,
    pub category_id: String,
    pub total_size_bytes: u64,
    pub file_count: usize,
    pub risk: String,
    pub needs_review: bool,
    pub reason: String,
}

/// Detecta se um diretório é a raiz de um projeto.
pub fn detect_project_root(path: &Path) -> Option<Vec<String>> {
    let mut detected_markers = Vec::new();
    for marker in PROJECT_MARKERS {
        if path.join(marker).exists() {
            detected_markers.push(marker.to_string());
        }
    }

    if detected_markers.is_empty() {
        None
    } else {
        Some(detected_markers)
    }
}

/// Classifica um projeto baseado no nome e marcadores.
pub fn classify_project(name: &str, markers: &[String]) -> (String, String) {
    let name_lower = name.to_lowercase();

    // Regras de Kryonix
    if name_lower.contains("kryonix")
        || name_lower.contains("home-brain")
        || markers.contains(&"flake.nix".to_string()) && name_lower.contains("brain")
    {
        return (
            "projetos.kryonix".to_string(),
            "Projeto Kryonix detectado".to_string(),
        );
    }

    // Regras de RAGOS
    if name_lower.contains("ragos") || name_lower.contains("projeto-ragos") {
        return (
            "projetos.ragos".to_string(),
            "Projeto RAGOS detectado".to_string(),
        );
    }

    // Regras de NixOS
    if markers.contains(&"flake.nix".to_string())
        || name_lower.contains("nixos")
        || name_lower.contains("home-manager")
    {
        return (
            "projetos.nixos".to_string(),
            "Projeto NixOS/Flake detectado".to_string(),
        );
    }

    // Regras de IA
    if name_lower.contains("ia")
        || name_lower.contains("ai")
        || name_lower.contains("llm")
        || name_lower.contains("rag")
        || name_lower.contains("agent")
    {
        return (
            "projetos.ia".to_string(),
            "Projeto de IA/LLM detectado".to_string(),
        );
    }

    // Regras de Infra
    if name_lower.contains("proxmox")
        || name_lower.contains("opnsense")
        || name_lower.contains("server")
        || name_lower.contains("network")
    {
        return (
            "projetos.infra".to_string(),
            "Projeto de Infraestrutura detectado".to_string(),
        );
    }

    // Regras de Windows
    if name_lower.contains("windows")
        || name_lower.contains("ativador")
        || name_lower.contains("office")
    {
        return (
            "projetos.windows".to_string(),
            "Projeto relacionado a Windows detectado".to_string(),
        );
    }

    // Fallback
    (
        "projetos.sandbox".to_string(),
        "Projeto genérico detectado".to_string(),
    )
}

/// Calcula o risco de um projeto.
pub fn calculate_project_risk(markers: &[String]) -> (String, bool) {
    if markers.iter().any(|m| m == ".git") {
        ("high".to_string(), true)
    } else {
        ("medium".to_string(), false)
    }
}
