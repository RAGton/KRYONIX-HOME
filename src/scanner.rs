use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::ignore;
use crate::metadata::{self, FileMetadata, FileStatus};

/// Diretórios permitidos para scan na Home do usuário.
#[allow(dead_code)]
const SCAN_DIRS: &[&str] = &[
    "Downloads",
    "Documentos",
    "Imagens",
    "Vídeos",
    "Músicas",
    "Área de Trabalho",
    "Desktop",
    "Pictures",
    "Videos",
    "Music",
    "Documents",
];

fn default_schema_version() -> String {
    "1.0".to_string()
}

/// Resultado completo de um scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub run_id: String,
    pub timestamp: DateTime<Utc>,
    pub home_dir: String,
    pub dirs_scanned: Vec<String>,
    pub files: Vec<FileMetadata>,
    pub projects: Vec<crate::project::ProjectCandidate>,
    pub files_analyzed: usize,
    pub files_ignored: usize,
    pub files_error: usize,
    pub total_size_bytes: u64,
    pub warnings: Vec<String>,
    #[serde(default)]
    pub full_home: bool,
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    #[serde(default)]
    pub denied_count: usize,
    #[serde(default)]
    pub skipped_count: usize,
    #[serde(default)]
    pub protected_count: usize,
    #[serde(default)]
    pub inbox_count: usize,
    #[serde(default)]
    pub project_count: usize,
}

/// Retorna o diretório de estado do Kryonix Home Brain.
fn state_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Não foi possível determinar o diretório home")?;
    let dir = home.join(".local/state/kryonix/home-brain");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Retorna o diretório de runs.
fn runs_dir(run_id: &str) -> Result<PathBuf> {
    let dir = state_dir()?.join("runs").join(run_id);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Gera um run_id baseado no timestamp e hostname.
fn generate_run_id() -> String {
    let ts = Utc::now().format("%Y%m%d-%H%M%S");
    let host = hostname::get()
        .map(|h: std::ffi::OsString| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    format!("{ts}-{host}")
}

/// Executa o scan da Home do usuário.
#[allow(dead_code)]
pub fn run_scan() -> Result<ScanResult> {
    run_scan_options(false, false, false, false)
}

pub fn run_scan_options(
    full_home: bool,
    metadata_only_override: bool,
    safe_content: bool,
    inbox_only: bool,
) -> Result<ScanResult> {
    let home = dirs::home_dir().context("Não foi possível determinar o diretório home")?;
    let run_id = generate_run_id();
    let timestamp = Utc::now();

    let mut files: Vec<FileMetadata> = Vec::new();
    let mut projects: Vec<crate::project::ProjectCandidate> = Vec::new();
    let mut dirs_scanned: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    if full_home {
        dirs_scanned.push("~".to_string());
        walk_directory(
            &home,
            &mut files,
            &mut projects,
            &mut warnings,
            true,
            metadata_only_override,
            safe_content,
        );
    } else {
        let scan_targets = if inbox_only {
            vec!["Downloads", "Desktop", "Área de Trabalho"]
        } else {
            vec![
                "Downloads",
                "Documentos",
                "Imagens",
                "Vídeos",
                "Músicas",
                "Área de Trabalho",
                "Desktop",
                "Pictures",
                "Videos",
                "Music",
                "Documents",
                "Projects",
                "Projetos",
            ]
        };

        for dir_name in scan_targets {
            let scan_path = home.join(dir_name);
            if !scan_path.exists() || !scan_path.is_dir() {
                continue;
            }
            dirs_scanned.push(dir_name.to_string());

            walk_directory(
                &scan_path,
                &mut files,
                &mut projects,
                &mut warnings,
                false,
                metadata_only_override,
                safe_content,
            );
        }
    }

    let files_analyzed = files
        .iter()
        .filter(|f| f.status == FileStatus::Analyzed)
        .count();
    let files_ignored = files
        .iter()
        .filter(|f| f.status == FileStatus::Ignored)
        .count();
    let files_error = files
        .iter()
        .filter(|f| f.status == FileStatus::Error)
        .count();

    let mut total_size_bytes: u64 = files
        .iter()
        .filter(|f| f.status == FileStatus::Analyzed)
        .map(|f| f.size_bytes)
        .sum();

    // Adicionar tamanho dos projetos ao total
    total_size_bytes += projects.iter().map(|p| p.total_size_bytes).sum::<u64>();

    let project_count = projects.len();
    let inbox_count = files
        .iter()
        .filter(|f| {
            let p = Path::new(&f.path);
            p.components().any(|c| {
                let s = c.as_os_str();
                s == "Downloads" || s == "Desktop" || s == "Área de Trabalho"
            })
        })
        .count();
    let protected_count = files
        .iter()
        .filter(|f| {
            let p = Path::new(&f.path);
            ignore::is_secret_file(p)
        })
        .count();
    let denied_count = files_error;
    let skipped_count = files_ignored;

    Ok(ScanResult {
        run_id,
        timestamp,
        home_dir: home.to_string_lossy().to_string(),
        dirs_scanned,
        files,
        projects,
        files_analyzed,
        files_ignored,
        files_error,
        total_size_bytes,
        warnings,
        full_home,
        schema_version: "1.0".to_string(),
        denied_count,
        skipped_count,
        protected_count,
        inbox_count,
        project_count,
    })
}

/// Percorre um diretório recursivamente.
fn walk_directory(
    root: &Path,
    files: &mut Vec<FileMetadata>,
    projects: &mut Vec<crate::project::ProjectCandidate>,
    warnings: &mut Vec<String>,
    full_home: bool,
    metadata_only_override: bool,
    safe_content: bool,
) {
    let walker = WalkDir::new(root)
        .follow_links(false)
        .same_file_system(true)
        .into_iter();

    let mut it = walker.filter_entry(|e: &walkdir::DirEntry| {
        let path = e.path();
        if e.file_type().is_dir() && ignore::should_ignore_dir_options(path, full_home) {
            return false;
        }
        true
    });

    while let Some(entry) = it.next() {
        let entry: walkdir::DirEntry = match entry {
            Ok(e) => e,
            Err(err) => {
                let err_str = err.to_string();
                if err_str.to_lowercase().contains("permission denied") {
                    warnings.push(format!(
                        "Permissão negada ao acessar path: {:?}",
                        err.path()
                    ));
                } else {
                    warnings.push(format!("Erro de leitura no path: {}", err_str));
                }
                continue;
            }
        };

        let path = entry.path();

        // Se for um diretório, checar se é um projeto
        if entry.file_type().is_dir() {
            if let Some(markers) = crate::project::detect_project_root(path) {
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                let (category_id, reason) = crate::project::classify_project(&name, &markers);
                let (mut risk, mut needs_review) = crate::project::calculate_project_risk(&markers);

                let mut project_warnings = Vec::new();

                let path_lower = path.to_string_lossy().to_lowercase();
                let is_non_ideal = path_lower.contains("/downloads/")
                    || path_lower.contains("/music/")
                    || path_lower.contains("/músicas/")
                    || path_lower.contains("/pictures/")
                    || path_lower.contains("/imagens/")
                    || path_lower.contains("/videos/")
                    || path_lower.contains("/vídeos/")
                    || path_lower.contains("/área de trabalho/")
                    || path_lower.contains("/desktop/");

                if is_non_ideal {
                    project_warnings.push("Projeto detectado em diretório não ideal".to_string());
                    needs_review = true;
                    if risk == "low" {
                        risk = "medium".to_string();
                    }
                    if markers.iter().any(|m| m == ".git") {
                        risk = "high".to_string();
                    }
                }

                // Calcular estatísticas do projeto de forma recursiva (incluindo ignorados como target)
                let (size, count) = calculate_dir_stats(path);

                projects.push(crate::project::ProjectCandidate {
                    root_path: path.to_string_lossy().to_string(),
                    name,
                    markers,
                    category_id,
                    total_size_bytes: size,
                    file_count: count,
                    risk,
                    needs_review,
                    reason,
                    warnings: project_warnings,
                });

                // Pular subdiretório do projeto no scanner normal para evitar duplicidade de arquivos
                it.skip_current_dir();
                continue;
            }
        }

        // Processar arquivos regulares (que não estão dentro de projetos detectados acima)
        if !entry.file_type().is_file() && !entry.file_type().is_symlink() {
            continue;
        }

        let is_symlink = entry.file_type().is_symlink();

        if ignore::should_ignore_file_options(path, full_home)
            || (!full_home && ignore::is_secret_file(path))
        {
            files.push(FileMetadata {
                path: path.to_string_lossy().to_string(),
                filename: path
                    .file_name()
                    .and_then(|n: &std::ffi::OsStr| n.to_str())
                    .unwrap_or("")
                    .to_string(),
                extension: String::new(),
                mime: String::new(),
                size_bytes: 0,
                modified_at: None,
                is_dir: false,
                is_file: true,
                is_symlink,
                is_hidden: crate::metadata::is_hidden_path(path),
                is_project_member: false,
                project_root: None,
                source_zone: Some("unknown".to_string()),
                readable: false,
                content_sampled: false,
                metadata_only: true,
                protected_reason: Some("Ignored or secret file".to_string()),
                warnings: vec!["Ignored or secret file".to_string()],
                content: None,
                context: None,
                status: FileStatus::Ignored,
            });
            continue;
        }

        let mut meta = metadata::collect(path, safe_content);

        if metadata_only_override {
            meta.metadata_only = true;
            meta.content_sampled = false;
            meta.protected_reason = Some("Forced metadata-only scan".to_string());
        }

        files.push(meta);
    }
}

/// Helper para calcular estatísticas de um diretório recursivamente.
fn calculate_dir_stats(path: &Path) -> (u64, usize) {
    let mut total_size = 0;
    let mut count = 0;

    for entry in WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e: Result<walkdir::DirEntry, walkdir::Error>| e.ok())
    {
        let entry: walkdir::DirEntry = entry;
        if entry.file_type().is_file() {
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            let path_str = entry.path().to_string_lossy();
            let should_skip = crate::project::PROJECT_IGNORED_DIRS.iter().any(|d| {
                let pattern = format!("/{}/", d);
                path_str.contains(&pattern) || path_str.ends_with(format!("/{}", d).as_str())
            });

            if !should_skip {
                total_size += meta.len();
                count += 1;
            }
        }
    }

    (total_size, count)
}

/// Salva o resultado do scan em disco.
pub fn save_scan(scan: &ScanResult) -> Result<()> {
    let state = state_dir()?;
    let run_dir = runs_dir(&scan.run_id)?;

    // Salvar no diretório do run
    let run_path = run_dir.join("scan.json");
    let json = serde_json::to_string_pretty(scan)?;
    fs::write(&run_path, &json)?;

    // Salvar como latest
    let latest_path = state.join("latest-scan.json");
    fs::write(&latest_path, &json)?;

    eprintln!("Scan salvo em: {}", run_path.display());
    eprintln!("Latest:        {}", latest_path.display());

    Ok(())
}

/// Carrega o último scan saved.
pub fn load_latest_scan() -> Result<ScanResult> {
    let state = state_dir()?;
    let path = state.join("latest-scan.json");

    if !path.exists() {
        anyhow::bail!(
            "Nenhum scan encontrado. Execute 'kryonix home scan' primeiro.\n\
             Arquivo esperado: {}",
            path.display()
        );
    }

    let json = fs::read_to_string(&path)?;
    let scan: ScanResult = serde_json::from_str(&json)?;
    Ok(scan)
}
