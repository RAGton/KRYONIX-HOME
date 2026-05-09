use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::planner::Plan;
use crate::scanner::ScanResult;

#[derive(Debug, Serialize, Deserialize)]
pub struct ManifestAction {
    pub source_path: String,
    pub target_path: String,
    pub action_type: String, // "move", "rename"
    pub old_hash: Option<String>,
    pub size_bytes: u64,
    pub mime: String,
    pub reason: String,
    pub risk: String,
    pub status: String, // "planned", "executed", "skipped", "failed"
    pub error_msg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules_applied: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub naming_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taxonomy_score: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_keywords: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taxonomy_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taxonomy_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_categories: Option<Vec<String>>,
    pub already_organized: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub run_id: String,
    pub timestamp: DateTime<Utc>,
    pub hostname: String,
    pub user: String,
    pub tool_version: String,
    pub actions: Vec<ManifestAction>,
}

/// Helper for state directories.
pub fn manifests_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Não foi possível determinar o diretório home")?;
    let dir = home.join(".local/state/kryonix/home-brain/manifests");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn audits_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Não foi possível determinar o diretório home")?;
    let dir = home.join(".local/state/kryonix/home-brain/audit");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn create_manifest(plan: &Plan, scan: &ScanResult) -> Result<Manifest> {
    let mut actions = Vec::new();

    // Create an index of file metadata from the scan
    let mut file_map = std::collections::HashMap::new();
    for file in &scan.files {
        file_map.insert(file.path.clone(), file);
    }

    for prop in &plan.proposals {
        if let Some(file_meta) = file_map.get(&prop.old_path) {
            let file_name = Path::new(&prop.old_path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();

            let target_file_name = prop.new_filename.as_deref().unwrap_or(file_name.as_ref());

            let target_path = Path::new(&plan.home_dir)
                .join(&prop.new_dir)
                .join(target_file_name)
                .to_string_lossy()
                .to_string();

            let operation_kind = if prop.action == "rename" {
                Some("move_rename".to_string())
            } else {
                Some(prop.action.clone())
            };

            actions.push(ManifestAction {
                source_path: prop.old_path.clone(),
                target_path,
                action_type: prop.action.clone(),
                old_hash: crate::hashing::sha256_of(Path::new(&prop.old_path)).ok(),
                size_bytes: file_meta.size_bytes,
                mime: file_meta.mime.clone(),
                reason: prop.reason.clone(),
                risk: prop.risk.clone(),
                status: "planned".to_string(),
                error_msg: None,
                old_filename: Some(file_name.into_owned()),
                new_filename: prop.new_filename.clone(),
                rules_applied: prop.rules_applied.clone(),
                naming_profile: prop.naming_profile.clone(),
                operation_kind,
                category_id: prop.category_id.clone(),
                category_label: prop.category_label.clone(),
                category_dir: prop.category_dir.clone(),
                taxonomy_score: prop.taxonomy_score,
                matched_keywords: prop.matched_keywords.clone(),
                taxonomy_reason: prop.taxonomy_reason.clone(),
                taxonomy_profile: prop.taxonomy_profile.clone(),
                candidate_categories: prop.candidate_categories.clone(),
                already_organized: prop.already_organized,
            });
        }
    }

    let manifest = Manifest {
        run_id: plan.run_id.clone(),
        timestamp: Utc::now(),
        hostname: hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string()),
        user: whoami::username().unwrap_or_else(|_| "unknown".to_string()),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        actions,
    };

    let filename = format!(
        "manifest_{}.json",
        manifest.timestamp.format("%Y%m%d-%H%M%S")
    );
    let path = manifests_dir()?.join(&filename);

    let json = serde_json::to_string_pretty(&manifest)?;
    fs::write(&path, json)?;

    eprintln!("Manifesto criado em: {}", path.display());

    Ok(manifest)
}

pub fn get_latest_manifest() -> Result<Manifest> {
    let dir = manifests_dir()?;
    let mut entries = fs::read_dir(&dir)?
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file() && e.path().extension().is_some_and(|ext| ext == "json"))
        .collect::<Vec<_>>();

    entries.sort_by_key(|e| e.path());

    let latest = entries
        .last()
        .context("Nenhum manifesto encontrado. Execute 'kryonix-home manifest create' primeiro.")?;
    let content = fs::read_to_string(latest.path())?;
    let manifest: Manifest = serde_json::from_str(&content)?;

    Ok(manifest)
}

pub fn show_manifest(manifest: &Manifest) {
    let total = manifest.actions.len();
    let moves = manifest
        .actions
        .iter()
        .filter(|a| a.action_type == "move")
        .count();
    let renames = manifest
        .actions
        .iter()
        .filter(|a| a.action_type == "rename")
        .count();
    let high_risk = manifest.actions.iter().filter(|a| a.risk == "high").count();

    println!("=== Resumo do Manifesto ===");
    println!("ID do Run: {}", manifest.run_id);
    println!("Data: {}", manifest.timestamp.format("%Y-%m-%d %H:%M:%S"));
    println!("Host: {} | User: {}", manifest.hostname, manifest.user);
    println!("Total de ações: {}", total);
    println!(" - Moves: {}", moves);
    println!(" - Renames: {}", renames);
    println!(" - Alto risco: {}", high_risk);
    println!("\nAções:");
    for action in &manifest.actions {
        println!(
            "  [{}] {} -> {}",
            action.risk.to_uppercase(),
            action.source_path,
            action.target_path
        );
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_manifest_serialization() {
        let manifest = Manifest {
            run_id: "test-123".to_string(),
            timestamp: Utc::now(),
            hostname: "testhost".to_string(),
            user: "testuser".to_string(),
            tool_version: "1.0".to_string(),
            actions: vec![ManifestAction {
                source_path: "/tmp/a".to_string(),
                target_path: "/tmp/b".to_string(),
                action_type: "move".to_string(),
                old_hash: Some("abc".to_string()),
                size_bytes: 100,
                mime: "text/plain".to_string(),
                reason: "test".to_string(),
                risk: "low".to_string(),
                status: "planned".to_string(),
                error_msg: None,
                old_filename: None,
                new_filename: None,
                rules_applied: None,
                naming_profile: None,
                operation_kind: Some("move".to_string()),
                category_label: None,
                category_dir: None,
                taxonomy_score: None,
                matched_keywords: None,
                taxonomy_reason: None,
                candidate_categories: None,
                already_organized: false,
            }],
        };

        let json = serde_json::to_string(&manifest).unwrap();
        assert!(json.contains("test-123"));
        assert!(json.contains("testhost"));
        assert!(json.contains("move"));
        assert!(json.contains("planned"));
    }
}
