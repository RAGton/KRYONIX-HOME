use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use crate::manifest::Manifest;
use crate::planner;

/// Evento de arquivo unificado e achatado para exportação auditável.
#[derive(Debug, Serialize, Deserialize)]
pub struct FileEvent {
    pub event_id: String,
    pub timestamp: String,
    pub hostname: String,
    pub user: String,
    pub source_type: String, // "scan", "plan", "manifest", "audit"
    pub file_path: String,
    pub file_hash: Option<String>,
    pub mime: String,
    pub size: u64,
    pub action: String,
    pub category_id: Option<String>,
    pub category_label: Option<String>,
    pub category_dir: Option<String>,
    pub taxonomy_score: Option<f32>,
    pub matched_keywords: Option<Vec<String>>,
    pub suggested_dir: Option<String>,
    pub suggested_filename: Option<String>,
    pub naming_profile: Option<String>,
    pub taxonomy_profile: Option<String>,
    pub manifest_id: Option<String>,
    pub audit_id: Option<String>,
    pub action_status: String,
    pub reason: String,
    pub source_path: String,
    pub target_path: String,
}

/// Helper para obter o diretório de memória.
pub fn memory_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Não foi possível determinar o diretório home")?;
    let dir = home.join(".local/state/kryonix/home-brain/memory");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Retorna a string SHA256 de forma determinística para ID de evento.
fn sha256_str(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

/// Carrega o relatório de auditoria mais recente.
pub fn get_latest_audit() -> Result<Manifest> {
    let dir = crate::manifest::audits_dir()?;
    let mut entries = fs::read_dir(&dir)?
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file() && e.path().extension().is_some_and(|ext| ext == "json"))
        .collect::<Vec<_>>();

    entries.sort_by_key(|e| e.path());

    let latest = entries
        .last()
        .context("Nenhum log de auditoria encontrado. Não há o que exportar.")?;

    let content = fs::read_to_string(latest.path())?;
    let audit: Manifest = serde_json::from_str(&content)?;
    Ok(audit)
}

/// Exporta os eventos da fonte especificada em formato JSONL.
pub fn export_memory(from_source: &str, jsonl_stdout: bool, dry_run: bool) -> Result<()> {
    let mut events = Vec::new();

    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let user = whoami::username().unwrap_or_else(|_| "unknown".to_string());

    match from_source {
        "latest-scan" => {
            let scan = crate::scanner::load_latest_scan()?;
            for file in &scan.files {
                let file_hash = crate::hashing::sha256_of(Path::new(&file.path)).ok();
                let event_id = format!("evt_{}", sha256_str(&format!("scan:{}:{}", scan.run_id, file.path))[..32].to_string());
                events.push(FileEvent {
                    event_id,
                    timestamp: scan.timestamp.to_rfc3339(),
                    hostname: hostname.clone(),
                    user: user.clone(),
                    source_type: "scan".to_string(),
                    file_path: file.path.clone(),
                    file_hash,
                    mime: file.mime.clone(),
                    size: file.size_bytes,
                    action: "scan".to_string(),
                    category_id: None,
                    category_label: None,
                    category_dir: None,
                    taxonomy_score: None,
                    matched_keywords: None,
                    suggested_dir: None,
                    suggested_filename: None,
                    naming_profile: None,
                    taxonomy_profile: None,
                    manifest_id: None,
                    audit_id: None,
                    action_status: "none".to_string(),
                    reason: format!("Arquivo escaneado na run {}", scan.run_id),
                    source_path: file.path.clone(),
                    target_path: file.path.clone(),
                });
            }
        }
        "latest-plan" => {
            let scan = crate::scanner::load_latest_scan()?;
            let plan = planner::generate_plan(
                &scan,
                true,  // rename_suggestions
                true,  // taxonomy_suggestions
                None,  // taxonomy_config_path
                true,  // include_large_files
                false, // safe_only
                false, // review_only
            );

            // Mapear arquivos do scan para fácil acesso de mime/size
            let mut file_map = std::collections::HashMap::new();
            for file in &scan.files {
                file_map.insert(file.path.clone(), file);
            }

            for prop in &plan.proposals {
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

                let mime = file_map.get(&prop.old_path)
                    .map(|f| f.mime.clone())
                    .unwrap_or_else(|| mime_guess::from_path(&prop.old_path).first_or_octet_stream().to_string());

                let size = file_map.get(&prop.old_path)
                    .map(|f| f.size_bytes)
                    .unwrap_or(0);

                let file_hash = crate::hashing::sha256_of(Path::new(&prop.old_path)).ok();
                let event_id = format!("evt_{}", sha256_str(&format!("plan:{}:{}", plan.run_id, prop.old_path))[..32].to_string());

                events.push(FileEvent {
                    event_id,
                    timestamp: scan.timestamp.to_rfc3339(),
                    hostname: hostname.clone(),
                    user: user.clone(),
                    source_type: "plan".to_string(),
                    file_path: prop.old_path.clone(),
                    file_hash,
                    mime,
                    size,
                    action: prop.action.clone(),
                    category_id: prop.category_id.clone(),
                    category_label: prop.category_label.clone(),
                    category_dir: prop.category_dir.clone(),
                    taxonomy_score: prop.taxonomy_score,
                    matched_keywords: prop.matched_keywords.clone(),
                    suggested_dir: Some(prop.new_dir.clone()),
                    suggested_filename: prop.new_filename.clone(),
                    naming_profile: prop.naming_profile.clone(),
                    taxonomy_profile: prop.taxonomy_profile.clone(),
                    manifest_id: None,
                    audit_id: None,
                    action_status: "planned".to_string(),
                    reason: prop.reason.clone(),
                    source_path: prop.old_path.clone(),
                    target_path,
                });
            }
        }
        "latest-manifest" => {
            let manifest = crate::manifest::get_latest_manifest()?;
            for action in &manifest.actions {
                let file_hash = action.old_hash.clone().or_else(|| crate::hashing::sha256_of(Path::new(&action.source_path)).ok());
                let event_id = format!("evt_{}", sha256_str(&format!("manifest:{}:{}", manifest.run_id, action.source_path))[..32].to_string());

                events.push(FileEvent {
                    event_id,
                    timestamp: manifest.timestamp.to_rfc3339(),
                    hostname: manifest.hostname.clone(),
                    user: manifest.user.clone(),
                    source_type: "manifest".to_string(),
                    file_path: action.source_path.clone(),
                    file_hash,
                    mime: action.mime.clone(),
                    size: action.size_bytes,
                    action: action.action_type.clone(),
                    category_id: action.category_id.clone(),
                    category_label: action.category_label.clone(),
                    category_dir: action.category_dir.clone(),
                    taxonomy_score: action.taxonomy_score,
                    matched_keywords: action.matched_keywords.clone(),
                    suggested_dir: action.category_dir.clone(),
                    suggested_filename: action.new_filename.clone(),
                    naming_profile: action.naming_profile.clone(),
                    taxonomy_profile: action.taxonomy_profile.clone(),
                    manifest_id: Some(manifest.run_id.clone()),
                    audit_id: None,
                    action_status: action.status.clone(),
                    reason: action.reason.clone(),
                    source_path: action.source_path.clone(),
                    target_path: action.target_path.clone(),
                });
            }
        }
        "latest-audit" => {
            let audit = get_latest_audit()?;
            for action in &audit.actions {
                let file_hash = action.old_hash.clone().or_else(|| crate::hashing::sha256_of(Path::new(&action.source_path)).ok());
                let event_id = format!("evt_{}", sha256_str(&format!("audit:{}:{}", audit.run_id, action.source_path))[..32].to_string());

                events.push(FileEvent {
                    event_id,
                    timestamp: audit.timestamp.to_rfc3339(),
                    hostname: audit.hostname.clone(),
                    user: audit.user.clone(),
                    source_type: "audit".to_string(),
                    file_path: action.source_path.clone(),
                    file_hash,
                    mime: action.mime.clone(),
                    size: action.size_bytes,
                    action: action.action_type.clone(),
                    category_id: action.category_id.clone(),
                    category_label: action.category_label.clone(),
                    category_dir: action.category_dir.clone(),
                    taxonomy_score: action.taxonomy_score,
                    matched_keywords: action.matched_keywords.clone(),
                    suggested_dir: action.category_dir.clone(),
                    suggested_filename: action.new_filename.clone(),
                    naming_profile: action.naming_profile.clone(),
                    taxonomy_profile: action.taxonomy_profile.clone(),
                    manifest_id: Some(audit.run_id.clone()),
                    audit_id: Some(audit.run_id.clone()),
                    action_status: action.status.clone(),
                    reason: action.reason.clone(),
                    source_path: action.source_path.clone(),
                    target_path: action.target_path.clone(),
                });
            }
        }
        other => {
            anyhow::bail!("Fonte de eventos desconhecida: {}", other);
        }
    }

    // Se jsonl_stdout for verdadeiro, imprimir cada evento compactado
    if jsonl_stdout {
        for ev in &events {
            println!("{}", serde_json::to_string(ev)?);
        }
    }

    // Se dry_run for falso, gravar os eventos no arquivo file-events.jsonl
    if !dry_run {
        let path = memory_dir()?.join("file-events.jsonl");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;

        for ev in &events {
            let json_line = format!("{}\n", serde_json::to_string(ev)?);
            file.write_all(json_line.as_bytes())?;
        }

        if !jsonl_stdout {
            eprintln!("Exportados {} eventos da fonte '{}' com sucesso para: {}", events.len(), from_source, path.display());
        }
    } else if !jsonl_stdout {
        eprintln!("Dry-run: exportaria {} eventos da fonte '{}' (simulação).", events.len(), from_source);
    }

    Ok(())
}
