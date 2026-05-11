use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::metadata::{FileMetadata, FileStatus};
use crate::scanner::ScanResult;

/// Categorias de destino para organização.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanProposal {
    pub action: String,
    pub risk: String,
    pub confidence: f64,
    pub old_path: String,
    pub new_dir: String,
    pub reason: String,
    pub evidence: String,
    pub needs_review: bool,
    pub protected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules_applied: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub naming_profile: Option<String>,
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

    // Novos campos para projetos
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_project: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_markers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_file_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_total_size: Option<u64>,
}

/// Plano completo de organização (dry-run).
#[derive(Debug, Serialize, Deserialize)]
pub struct Plan {
    pub run_id: String,
    pub mode: String,
    pub home_dir: String,
    pub files_seen: usize,
    pub projects_seen: usize,
    pub proposals: Vec<PlanProposal>,
    pub protected_files: Vec<FileMetadata>,
    pub content_aware: bool,
    pub context_aware: bool,
    pub full_home: bool,
}

#[derive(Debug, Default, Clone)]
pub struct PlanOptions<'a> {
    pub rename_suggestions: bool,
    pub taxonomy_suggestions: bool,
    pub taxonomy_config_path: Option<&'a str>,
    pub include_large_files: bool,
    pub safe_only: bool,
    pub review_only: bool,
    pub projects_only: bool,
    pub limit: Option<usize>,
    pub ollama: bool,
    pub full_home: bool,
    pub content_aware: bool,
    pub context_aware: bool,
}

/// Gera um plano de organização determinístico baseado em MIME/extensão e, opcionalmente, sugere renomeações.
/// Este plano é SOMENTE informativo (dry-run). Nenhuma ação é executada.
pub fn generate_plan(scan: &ScanResult, options: &PlanOptions) -> Plan {
    let mut proposals = Vec::new();
    let mut protected_files = Vec::new();
    let taxonomy_config = crate::taxonomy::load_taxonomy_config(options.taxonomy_config_path);

    // 1. Processar Projetos primeiro
    for project in &scan.projects {
        // SEGURANÇA: Projetos em paths protegidos NUNCA devem gerar propostas de ação
        if let Some(reason) = crate::metadata::is_protected_path(Path::new(&project.root_path)) {
            // Converter ProjectCandidate para FileMetadata fake para entrar na lista de protegidos
            let fake_meta = FileMetadata {
                path: project.root_path.clone(),
                filename: project.name.clone(),
                extension: String::new(),
                mime: "inode/directory".to_string(),
                size_bytes: project.total_size_bytes,
                modified_at: None,
                is_dir: true,
                is_file: false,
                is_symlink: false,
                is_hidden: project.root_path.contains("/."),
                is_project_member: true,
                project_root: Some(project.root_path.clone()),
                source_zone: None,
                readable: true,
                content_sampled: false,
                metadata_only: true,
                protected_reason: Some(reason),
                warnings: vec![],
                content: None,
                context: None,
                status: FileStatus::Analyzed,
            };
            protected_files.push(fake_meta);
            continue;
        }

        let taxonomy_config_ref = &taxonomy_config;
        let mut proposal = PlanProposal {
            action: "move_project".to_string(),
            risk: project.risk.clone(),
            confidence: 0.95,
            old_path: project.root_path.clone(),
            new_dir: String::new(), // Será preenchido abaixo
            reason: project.reason.clone(),
            evidence: format!("Marcadores detectados: {}", project.markers.join(", ")),
            needs_review: project.needs_review || project.root_path.contains("Obsidian Vault"),
            protected: false,
            new_filename: None,
            rules_applied: Some(project.markers.clone()),
            naming_profile: None,
            category_id: Some(project.category_id.clone()),
            category_label: None,
            category_dir: None,
            taxonomy_score: Some(0.95),
            matched_keywords: None,
            taxonomy_reason: None,
            taxonomy_profile: Some(taxonomy_config.profile.clone()),
            candidate_categories: None,
            already_organized: false,
            is_project: Some(true),
            project_markers: Some(project.markers.clone()),
            project_file_count: Some(project.file_count),
            project_total_size: Some(project.total_size_bytes),
        };

        // Classificar projeto pela taxonomia baseada no ID
        if let Some(cat_config) = taxonomy_config_ref
            .categories
            .iter()
            .find(|c| c.id == project.category_id)
        {
            proposal.new_dir = cat_config.dir.clone();
            proposal.category_label = Some(cat_config.label.clone());
            proposal.category_dir = Some(proposal.new_dir.clone());
        } else {
            // Fallback se não estiver no TOML (ex: sandbox)
            if project.root_path.to_lowercase().contains("/downloads/") {
                proposal.new_dir = "Documentos/00_Inbox/Downloads/Revisar".to_string();
                proposal.category_label = Some("Downloads / Transient Review".to_string());
            } else {
                proposal.new_dir = "Projetos/Sandbox".to_string();
                proposal.category_label = Some("Sandbox".to_string());
            }
        }

        // Verifica se já está organizado
        let expected_dir_path = Path::new(&scan.home_dir).join(&proposal.new_dir);
        let current_path = Path::new(&project.root_path);
        let in_correct_dir = current_path
            .parent()
            .map(|p| p == expected_dir_path)
            .unwrap_or(false);
        proposal.already_organized = in_correct_dir;

        if !in_correct_dir {
            proposals.push(proposal);
        }
    }

    // Se pedirmos apenas projetos, paramos aqui
    if !options.projects_only {
        // 2. Processar Arquivos Soltos
        for file in &scan.files {
            if file.status != FileStatus::Analyzed {
                continue;
            }

            // REGRA 7 & 8: Arquivos protegidos ou dentro de projetos Git NUNCA devem gerar propostas de ação
            if file.metadata_only || file.protected_reason.is_some() || file.is_project_member {
                // Mascarar path para o relatório se for protegido
                let mut masked_file = file.clone();
                if let Some(ref reason) = file.protected_reason {
                    masked_file.path = mask_protected_path(&file.path, reason);
                }
                protected_files.push(masked_file);
                continue;
            }

            // Limite de 2 GiB para arquivos grandes
            if file.size_bytes > 2_147_483_648 && !options.include_large_files {
                continue;
            }

            let mut proposal = if options.taxonomy_suggestions {
                let cat = if options.ollama {
                    let sug = crate::ollama::get_advisor_suggestion(file);
                    let matched_cat = taxonomy_config
                        .categories
                        .iter()
                        .find(|c| c.id == sug.category_id);
                    if let Some(c) = matched_cat {
                        crate::taxonomy::TaxonomyCategory {
                            id: c.id.clone(),
                            label: c.label.clone(),
                            relative_dir: std::path::PathBuf::from(&c.dir),
                            confidence: sug.confidence,
                            risk: if sug.confidence < 0.70 {
                                "medium".to_string()
                            } else {
                                "low".to_string()
                            },
                            needs_review: sug.confidence < 0.70,
                            reason: format!("{} | Sugerido por Ollama Advisor", sug.reason),
                            rules_applied: vec!["ollama_advisor".to_string()],
                            matched_keywords: vec![],
                            candidate_categories: None,
                            already_organized: false,
                        }
                    } else {
                        crate::taxonomy::suggest_category_config(file, &taxonomy_config)
                    }
                } else {
                    crate::taxonomy::suggest_category_config(file, &taxonomy_config)
                };
                PlanProposal {
                    action: "move".to_string(),
                    risk: cat.risk.clone(),
                    confidence: cat.confidence as f64,
                    old_path: file.path.clone(),
                    new_dir: cat.relative_dir.to_string_lossy().to_string(),
                    reason: cat.reason.clone(),
                    evidence: format!("Extensão: {} | MIME: {}", file.extension, file.mime),
                    needs_review: cat.needs_review || file.path.contains("Obsidian Vault"),
                    protected: false,
                    new_filename: None,
                    rules_applied: Some(cat.rules_applied.clone()),
                    naming_profile: None,
                    category_id: Some(cat.id.clone()),
                    category_label: Some(cat.label.clone()),
                    category_dir: Some(cat.relative_dir.to_string_lossy().to_string()),
                    taxonomy_score: Some(cat.confidence),
                    matched_keywords: Some(cat.matched_keywords.clone()),
                    taxonomy_reason: Some(cat.reason.clone()),
                    taxonomy_profile: Some(taxonomy_config.profile.clone()),
                    candidate_categories: cat.candidate_categories.clone(),
                    already_organized: false,
                    is_project: Some(false),
                    project_markers: None,
                    project_file_count: None,
                    project_total_size: None,
                }
            } else {
                match classify_file(file) {
                    Some(p) => p,
                    None => continue,
                }
            };

            // Verifica se já está no diretório correto
            let expected_dir_path = Path::new(&scan.home_dir).join(&proposal.new_dir);
            let current_dir_path = Path::new(&file.path).parent().unwrap();
            let in_correct_dir = current_dir_path == expected_dir_path;
            proposal.already_organized = in_correct_dir;

            let mut has_rename = false;
            if options.rename_suggestions {
                if let Some(suggestion) = crate::naming::suggest_rename(file) {
                    if suggestion.suggested_filename != file.filename {
                        proposal.action = "rename".to_string();
                        proposal.new_filename = Some(suggestion.suggested_filename);

                        let mut rules = proposal.rules_applied.unwrap_or_default();
                        rules.extend(suggestion.rules_applied);
                        proposal.rules_applied = Some(rules);

                        proposal.naming_profile = Some(suggestion.naming_profile);
                        proposal.needs_review = proposal.needs_review || suggestion.needs_review;

                        if suggestion.risk == "high" {
                            proposal.risk = "high".to_string();
                        } else if suggestion.risk == "medium" && proposal.risk == "low" {
                            proposal.risk = "medium".to_string();
                        }
                        proposal.confidence =
                            (proposal.confidence + suggestion.confidence as f64) / 2.0;
                        proposal.reason =
                            format!("Move: {} | Rename: {}", proposal.reason, suggestion.reason);
                        has_rename = true;
                    }
                }
            }

            // Se o arquivo já estiver na pasta final correta e não houver renomeação proposta, ignorar
            if in_correct_dir && !has_rename {
                continue;
            }

            // Aplicar filtros
            if options.safe_only && (proposal.risk == "high" || proposal.needs_review) {
                continue;
            }
            if options.review_only && !proposal.needs_review {
                continue;
            }

            proposals.push(proposal);
        }
    }

    // Aplicar limite se solicitado
    if let Some(l) = options.limit {
        if proposals.len() > l {
            proposals.truncate(l);
        }
    }

    Plan {
        run_id: scan.run_id.clone(),
        mode: "dry-run".to_string(),
        home_dir: scan.home_dir.clone(),
        files_seen: scan.files_analyzed,
        projects_seen: scan.projects.len(),
        proposals,
        protected_files,
        content_aware: options.content_aware,
        context_aware: options.context_aware,
        full_home: scan.full_home,
    }
}

/// Classifica um arquivo por MIME/extensão e sugere destino.
fn classify_file(file: &FileMetadata) -> Option<PlanProposal> {
    // SEGURANÇA MÁXIMA: Se for caminho protegido, NUNCA sugerir ação de movimentação.
    if file.metadata_only || file.protected_reason.is_some() {
        return None;
    }

    // Redundância defensiva: checar o path novamente com a lógica canônica
    if let Some(_reason) = crate::metadata::is_protected_path(std::path::Path::new(&file.path)) {
        return None;
    }

    // Bloqueio de arquivos ocultos por padrão na Home (pontos)
    // Exceto se for explicitamente parte de um vault Obsidian ou outro local permitido
    if file.is_hidden && !file.path.to_lowercase().contains("/.obsidian/") {
        return None;
    }

    let mime = file.mime.as_str();
    let ext = file.extension.as_str();

    let (new_dir, reason, confidence) = match mime {
        // Documentos
        "application/pdf" => ("Documentos/Revisar", "PDF detectado por MIME", 0.85),
        m if m.starts_with("text/") && matches!(ext, "md" | "txt" | "rst" | "org") => {
            ("Documentos/Revisar", "Documento de texto detectado", 0.80)
        }
        m if m == "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            || m == "application/msword" =>
        {
            ("Documentos/Revisar", "Documento Word detectado", 0.85)
        }
        m if m == "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            || m == "application/vnd.ms-excel" =>
        {
            ("Documentos/Revisar", "Planilha detectada", 0.85)
        }

        // Imagens
        m if m.starts_with("image/") => ("Midia/Imagens", "Imagem detectada por MIME", 0.90),

        // Vídeos
        m if m.starts_with("video/") => ("Midia/Videos", "Vídeo detectado por MIME", 0.90),

        // Áudio
        m if m.starts_with("audio/") => ("Midia/Audio", "Áudio detectado por MIME", 0.90),

        // Compactados
        "application/zip"
        | "application/x-tar"
        | "application/gzip"
        | "application/x-7z-compressed"
        | "application/x-rar-compressed"
        | "application/x-bzip2"
        | "application/x-xz"
        | "application/zstd" => ("Arquivos/Compactados", "Arquivo compactado detectado", 0.88),

        // ISOs
        m if m == "application/x-iso9660-image" || matches!(ext, "iso" | "img") => {
            ("Arquivos/ISOs", "Imagem de disco detectada", 0.92)
        }

        // Executáveis
        m if m == "application/x-executable"
            || m == "application/x-sharedlib"
            || matches!(ext, "appimage" | "run" | "bin") =>
        {
            ("Arquivos/Executaveis", "Executável detectado", 0.80)
        }

        // Fallback por extensão
        _ => match ext {
            "pdf" => ("Documentos/Revisar", "PDF por extensão", 0.75),
            "doc" | "docx" | "odt" => ("Documentos/Revisar", "Documento por extensão", 0.75),
            "xls" | "xlsx" | "ods" => ("Documentos/Revisar", "Planilha por extensão", 0.75),
            "ppt" | "pptx" | "odp" => ("Documentos/Revisar", "Apresentação por extensão", 0.75),
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "bmp" | "tiff" | "ico" | "heic" => {
                ("Midia/Imagens", "Imagem por extensão", 0.80)
            }
            "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" => {
                ("Midia/Videos", "Vídeo por extensão", 0.80)
            }
            "mp3" | "flac" | "ogg" | "wav" | "m4a" | "aac" | "opus" | "wma" => {
                ("Midia/Audio", "Áudio por extensão", 0.80)
            }
            "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "zst" => {
                ("Arquivos/Compactados", "Compactado por extensão", 0.80)
            }
            "iso" | "img" => ("Arquivos/ISOs", "ISO por extensão", 0.85),
            "appimage" | "run" | "bin" => ("Arquivos/Executaveis", "Executável por extensão", 0.70),
            _ => (
                "Arquivos/Revisar",
                "Tipo desconhecido; requer revisão",
                0.40,
            ),
        },
    };

    Some(PlanProposal {
        action: "move".to_string(),
        risk: if confidence >= 0.85 {
            "low".to_string()
        } else if confidence >= 0.65 {
            "medium".to_string()
        } else {
            "high".to_string()
        },
        confidence,
        old_path: file.path.clone(),
        new_dir: new_dir.to_string(),
        reason: reason.to_string(),
        evidence: format!("Extensão: {} | MIME: {}", file.extension, file.mime),
        needs_review: confidence < 0.70 || file.path.contains("Obsidian Vault"),
        protected: false,
        new_filename: None,
        rules_applied: None,
        naming_profile: None,
        category_id: None,
        category_label: None,
        category_dir: None,
        taxonomy_score: None,
        matched_keywords: None,
        taxonomy_reason: None,
        taxonomy_profile: None,
        candidate_categories: None,
        already_organized: false,
        is_project: None,
        project_markers: None,
        project_file_count: None,
        project_total_size: None,
    })
}

/// Mascara paths protegidos para relatórios
fn mask_protected_path(path: &str, reason: &str) -> String {
    let p = Path::new(path);
    // Tenta achar o diretório raiz sensível (.ssh, .gnupg, etc)
    for component in p.components() {
        if let std::path::Component::Normal(name) = component {
            let n = name.to_string_lossy().to_lowercase();
            if n == ".ssh" || n == ".gnupg" || n == ".config" || n == ".local" || n == ".cache" {
                return format!("~/{}/[PROTECTED: {}]", n, reason);
            }
        }
    }
    format!("[PROTECTED: {}]", reason)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::FileStatus;

    fn create_mock_file(path: &str, protected: bool) -> FileMetadata {
        FileMetadata {
            path: path.to_string(),
            filename: Path::new(path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            extension: Path::new(path)
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            mime: if protected {
                "application/octet-stream"
            } else {
                "application/pdf"
            }
            .to_string(),
            size_bytes: 1024,
            modified_at: None,
            is_dir: false,
            is_file: true,
            is_symlink: false,
            is_hidden: path.contains("/."),
            is_project_member: false,
            project_root: None,
            source_zone: None,
            readable: !protected,
            content_sampled: false,
            metadata_only: protected,
            protected_reason: if protected {
                Some("protected".to_string())
            } else {
                None
            },
            warnings: vec![],
            content: None,
            context: None,
            status: FileStatus::Analyzed,
        }
    }

    #[test]
    fn test_planner_absolute_protection_enforcement() {
        let mut files = Vec::new();
        files.push(create_mock_file("/home/user/.ssh/id_rsa", true));
        files.push(create_mock_file("/home/user/.gnupg/secring.gpg", true));
        files.push(create_mock_file("/home/user/.config/app/secrets.env", true));
        files.push(create_mock_file("/home/user/Downloads/public.pdf", false));

        let scan = ScanResult {
            run_id: "test".to_string(),
            timestamp: Utc::now(),
            home_dir: "/home/user".to_string(),
            dirs_scanned: vec!["/home/user".to_string()],
            files,
            projects: vec![],
            files_analyzed: 4,
            files_ignored: 0,
            files_error: 0,
            total_size_bytes: 4096,
            warnings: vec![],
            full_home: true,
        };

        let options = PlanOptions {
            full_home: true,
            ..Default::default()
        };
        let plan = generate_plan(&scan, &options);

        // Somente o PDF deve gerar proposta
        assert_eq!(plan.proposals.len(), 1);
        assert_eq!(
            plan.proposals[0].old_path,
            "/home/user/Downloads/public.pdf"
        );

        // 3 arquivos devem estar na lista de protegidos
        assert_eq!(plan.protected_files.len(), 3);

        // Verificar se todos os protegidos foram capturados e mascarados
        let protected_paths: Vec<_> = plan.protected_files.iter().map(|f| &f.path).collect();
        assert!(protected_paths.contains(&&"~/.ssh/[PROTECTED: protected]".to_string()));
        assert!(protected_paths.contains(&&"~/.gnupg/[PROTECTED: protected]".to_string()));
        assert!(protected_paths.contains(&&"~/.config/[PROTECTED: protected]".to_string()));
    }
}
