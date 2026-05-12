use serde::{Deserialize, Serialize};
use std::path::Path;

/// Marcadores que indicam a raiz de um projeto de software ou vault de conhecimento.
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
    ".obsidian",
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
    ".obsidian",
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
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectEvidence {
    pub is_project: bool,
    pub confidence: f64,
    pub strong_markers: Vec<String>,
    pub weak_markers: Vec<String>,
    pub project_kind: String,
    pub warnings: Vec<String>,
    pub reason: String,
}

pub const STRONG_PROJECT_MARKERS: &[&str] = &[
    ".git",
    "flake.nix",
    "Cargo.toml",
    "pyproject.toml",
    "package.json",
    "go.mod",
    ".obsidian",
];

pub const WEAK_PROJECT_MARKERS: &[&str] = &[
    "Makefile",
    "README.md",
    "deno.json",
    "pnpm-lock.yaml",
    "yarn.lock",
];

/// Detecta se um diretório é a raiz de um projeto.
pub fn detect_project_root(path: &Path) -> Option<Vec<String>> {
    let mut detected_markers = Vec::new();
    for marker in PROJECT_MARKERS {
        if path.join(marker).exists() {
            detected_markers.push(marker.to_string());
        }
    }

    let folder_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let folder_name_upper = folder_name.to_uppercase();

    // Regra defensiva contra falsos positivos para pastas como "AULAS" ou "DOCUMENTOS"
    if folder_name_upper == "AULAS"
        || folder_name_upper == "DOCUMENTOS"
        || folder_name_upper == "ESTUDOS"
    {
        // Exige pelo menos um marcador forte
        let has_strong = detected_markers
            .iter()
            .any(|m| STRONG_PROJECT_MARKERS.contains(&m.as_str()));
        if !has_strong {
            return None;
        }
    }

    if detected_markers.is_empty() {
        None
    } else {
        Some(detected_markers)
    }
}

pub fn get_project_evidence(_path: &Path, name: &str, markers: &[String]) -> ProjectEvidence {
    let mut strong_markers = Vec::new();
    let mut weak_markers = Vec::new();

    for marker in markers {
        if STRONG_PROJECT_MARKERS.contains(&marker.as_str()) {
            strong_markers.push(marker.clone());
        } else {
            weak_markers.push(marker.clone());
        }
    }

    let (category_id, reason) = classify_project(name, markers);
    let mut warnings = Vec::new();
    let mut confidence = 0.5;

    if !strong_markers.is_empty() {
        confidence += 0.4;
        if strong_markers.contains(&".git".to_string()) {
            confidence += 0.05;
        }
    }
    if !weak_markers.is_empty() {
        confidence += 0.05;
    }

    let name_lower = name.to_lowercase();
    if name_lower.contains("aulas") || name_lower.contains("estudos") {
        if strong_markers.is_empty() {
            confidence = 0.2;
            warnings.push("Pasta de estudos sem marcadores fortes de código".to_string());
        }
    }

    let is_project = confidence >= 0.7 && !strong_markers.is_empty();
    let final_confidence = if confidence > 1.0 {
        1.0
    } else if confidence < 0.0 {
        0.0
    } else {
        confidence
    };

    ProjectEvidence {
        is_project,
        confidence: final_confidence,
        strong_markers,
        weak_markers,
        project_kind: category_id,
        warnings,
        reason,
    }
}

/// Classifica um projeto baseado no nome e marcadores.
pub fn classify_project(name: &str, markers: &[String]) -> (String, String) {
    let name_lower = name.to_lowercase();

    // Regras de Obsidian Knowledge Vault (Prioridade absoluta)
    if markers.contains(&".obsidian".to_string())
        || name_lower.contains("obsidian")
        || name_lower.contains("vault")
    {
        return (
            "conhecimento.vault".to_string(),
            "Obsidian Knowledge Vault detectado".to_string(),
        );
    }

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
    if markers.iter().any(|m| m == ".obsidian" || m == ".git") {
        ("high".to_string(), true)
    } else {
        ("medium".to_string(), false)
    }
}
