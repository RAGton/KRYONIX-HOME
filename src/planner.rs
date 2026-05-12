use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::metadata::{FileMetadata, FileStatus};
use crate::scanner::ScanResult;

/// Categorias de destino para organização.
fn default_medium() -> String {
    "medium".to_string()
}

fn default_true() -> bool {
    true
}

fn default_zero_f64() -> f64 {
    0.0
}

fn default_schema_version() -> String {
    "1.0".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanProposal {
    pub action: String,
    #[serde(default = "default_medium")]
    pub risk: String,
    #[serde(default = "default_zero_f64")]
    pub confidence: f64,
    pub old_path: String,
    pub source: String,
    pub new_dir: String,
    pub destination: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default = "default_true")]
    pub needs_review: bool,
    #[serde(default)]
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
    #[serde(default)]
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

    // Autopilot fields
    pub decision_class: crate::decision::DecisionClass,
    pub auto_apply_allowed: bool,
    pub blocked_from_apply: bool,
    pub staging_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_breakdown: Option<crate::decision::ConfidenceBreakdown>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_profile: Option<crate::content::ContentProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_profile: Option<crate::context::ContextProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_evidence: Option<crate::project::ProjectEvidence>,
    pub safety_flags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rollback_hint: Option<String>,
}

/// Plano completo de organização (dry-run).
#[derive(Debug, Serialize, Deserialize)]
pub struct Plan {
    pub run_id: String,
    #[serde(default)]
    pub mode: String,
    pub home_dir: String,
    #[serde(default)]
    pub files_seen: usize,
    #[serde(default)]
    pub projects_seen: usize,
    pub proposals: Vec<PlanProposal>,
    #[serde(default)]
    pub protected_files: Vec<FileMetadata>,
    #[serde(default)]
    pub content_aware: bool,
    #[serde(default)]
    pub context_aware: bool,
    #[serde(default)]
    pub full_home: bool,
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
}

#[allow(dead_code)]
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
    pub min_confidence: Option<f64>,
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
            source: project.root_path.clone(),
            new_dir: String::new(), // Será preenchido abaixo
            destination: String::new(),
            reason: project.reason.clone(),
            evidence: vec![format!(
                "Marcadores detectados: {}",
                project.markers.join(", ")
            )],
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
            // Autopilot fields defaults
            decision_class: crate::decision::DecisionClass::NeedsHumanReview,
            auto_apply_allowed: false,
            blocked_from_apply: false,
            staging_only: false,
            confidence_breakdown: None,
            content_profile: None,
            context_profile: None,
            project_evidence: None,
            safety_flags: vec![],
            rollback_hint: None,
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

        // Run autopilot evaluation & enrichment
        enrich_and_classify_proposal(
            &mut proposal,
            None,
            Some(project),
            &scan.home_dir,
            Some(options),
        );

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
                    source: file.path.clone(),
                    new_dir: cat.relative_dir.to_string_lossy().to_string(),
                    destination: String::new(),
                    reason: cat.reason.clone(),
                    evidence: vec![format!(
                        "Extensão: {} | MIME: {}",
                        file.extension, file.mime
                    )],
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
                    // Autopilot fields defaults
                    decision_class: crate::decision::DecisionClass::NeedsHumanReview,
                    auto_apply_allowed: false,
                    blocked_from_apply: false,
                    staging_only: false,
                    confidence_breakdown: None,
                    content_profile: None,
                    context_profile: None,
                    project_evidence: None,
                    safety_flags: vec![],
                    rollback_hint: None,
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

            // Run autopilot evaluation & enrichment
            enrich_and_classify_proposal(
                &mut proposal,
                Some(file),
                None,
                &scan.home_dir,
                Some(options),
            );

            // Aplicar filtros
            if options.safe_only
                && (proposal.risk == "high" || proposal.needs_review || proposal.blocked_from_apply)
            {
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
        schema_version: "1.0".to_string(),
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
        source: file.path.clone(),
        new_dir: new_dir.to_string(),
        destination: String::new(),
        reason: reason.to_string(),
        evidence: vec![format!(
            "Extensão: {} | MIME: {}",
            file.extension, file.mime
        )],
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
        // Autopilot fields defaults
        decision_class: crate::decision::DecisionClass::NeedsHumanReview,
        auto_apply_allowed: false,
        blocked_from_apply: false,
        staging_only: false,
        confidence_breakdown: None,
        content_profile: None,
        context_profile: None,
        project_evidence: None,
        safety_flags: vec![],
        rollback_hint: None,
    })
}

pub fn enrich_and_classify_proposal(
    proposal: &mut PlanProposal,
    file_meta: Option<&FileMetadata>,
    project_candidate: Option<&crate::project::ProjectCandidate>,
    home_dir: &str,
    _options: Option<&PlanOptions>,
) {
    let old_path = proposal.old_path.clone();
    let new_dir = proposal.new_dir.clone();

    // Fill dual compatible fields
    proposal.source = old_path.clone();
    proposal.destination = if let Some(ref name) = proposal.new_filename {
        Path::new(home_dir)
            .join(&new_dir)
            .join(name)
            .to_string_lossy()
            .to_string()
    } else {
        let name = Path::new(&old_path).file_name().unwrap_or_default();
        Path::new(home_dir)
            .join(&new_dir)
            .join(name)
            .to_string_lossy()
            .to_string()
    };

    let path_buf = Path::new(&old_path);

    // Safety flags
    let mut safety_flags = Vec::new();

    // Let's analyze safety markers first
    let is_protected = crate::metadata::is_protected_path(path_buf).is_some();
    if is_protected {
        safety_flags.push("protected_path".to_string());
    }

    let is_hidden = crate::metadata::is_hidden_path(path_buf);
    if is_hidden {
        safety_flags.push("hidden_file".to_string());
    }

    let ext_lower = path_buf
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let is_script_or_exe = matches!(
        ext_lower.as_str(),
        "sh" | "ps1" | "exe" | "msi" | "bat" | "cmd" | "bin" | "run"
    );
    if is_script_or_exe {
        safety_flags.push("script_or_executable".to_string());
    }

    let mut is_project = false;
    let mut is_vault = false;

    let mut content_profile = None;
    let mut context_profile = None;
    let mut project_evidence = None;

    let mut folder_context_score = 0.0;
    let mut content_score = 0.0;
    let mut project_marker_score = 0.0;
    let mut filename_score = 0.5;
    let mut mime_score = 0.5;

    // If we have file metadata
    if let Some(file) = file_meta {
        is_project = file.is_project_member;

        if let Some(ref content) = file.content {
            content_profile = Some(content.clone());
            if !content.safe_to_read {
                safety_flags.push("sensitive_content_redacted".to_string());
            }
            if !content.imports.is_empty() {
                safety_flags.push("contains_code_imports".to_string());
            }
            // Scoring content:
            if let Some(ref summary) = content.summary {
                if summary.to_lowercase().contains("import") {
                    content_score += 0.2;
                }
            }
            if !content.keywords.is_empty() {
                content_score += 0.3;
            }
        }

        if let Some(ref context) = file.context {
            context_profile = Some(context.clone());
            if let Some(ref fc) = context.folder_context {
                is_vault = fc.is_vault;

                // Sibling matching score:
                if let Some(ref dom_cat) = fc.dominant_category {
                    if let Some(ref prop_cat) = proposal.category_id {
                        if dom_cat.split('.').next() == prop_cat.split('.').next() {
                            folder_context_score += 0.3;
                        }
                    }
                }
            }
        }
    }

    // If it's a project proposal
    if let Some(project) = project_candidate {
        is_project = true;
        is_vault = project.category_id == "conhecimento.vault";

        let mut strong = Vec::new();
        let mut weak = Vec::new();
        for m in &project.markers {
            if crate::project::STRONG_PROJECT_MARKERS.contains(&m.as_str()) {
                strong.push(m.clone());
            } else {
                weak.push(m.clone());
            }
        }

        project_evidence = Some(crate::project::ProjectEvidence {
            is_project: true,
            confidence: 0.95,
            strong_markers: strong,
            weak_markers: weak,
            project_kind: project.category_id.clone(),
            warnings: project.warnings.clone(),
            reason: project.reason.clone(),
        });
        project_marker_score = 0.4;
    } else if let Some(file) = file_meta {
        // Evaluate project root detection if not already marked as candidate
        if file.is_dir {
            if let Some(markers) = crate::project::detect_project_root(path_buf) {
                let name = path_buf
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let evidence = crate::project::get_project_evidence(path_buf, &name, &markers);
                is_project = evidence.is_project;
                project_evidence = Some(evidence);
                project_marker_score = 0.3;
            }
        }
    }

    // If destination contains "revisar", "baixa_confianca", "conflitos", or category is "incerto"
    let is_uncertain_destination = new_dir.to_lowercase().contains("revisar")
        || new_dir.to_lowercase().contains("baixa_confianca")
        || new_dir.to_lowercase().contains("conflitos")
        || proposal.category_id.as_deref().unwrap_or("") == "incerto";

    // Overwrite check (strictly unsafe for autopilot)
    let dest_path = Path::new(&proposal.destination);
    let is_overwrite = dest_path.exists();
    if is_overwrite {
        safety_flags.push("overwrite_target_exists".to_string());
    }

    // Calculate filename and mime scores
    let filename_lower = path_buf
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    if let Some(ref cat_id) = proposal.category_id {
        let cat_part = cat_id.split('.').next_back().unwrap_or("");
        if filename_lower.contains(cat_part) {
            filename_score += 0.25;
        }
    }
    if let Some(file) = file_meta {
        if file.mime.starts_with("image")
            || file.mime.starts_with("video")
            || file.mime.starts_with("audio")
        {
            mime_score += 0.3;
        }
    }

    // Compound score calculation
    let mut final_score = (proposal.confidence * 0.4)
        + (filename_score * 0.15)
        + (mime_score * 0.15)
        + (folder_context_score * 0.15)
        + (content_score * 0.1)
        + (project_marker_score * 0.05);

    final_score = final_score.clamp(0.0, 1.0);

    proposal.confidence = final_score;

    let confidence_breakdown = crate::decision::ConfidenceBreakdown {
        filename_score,
        mime_score,
        folder_context_score,
        content_score,
        project_marker_score,
        final_score,
    };
    proposal.confidence_breakdown = Some(confidence_breakdown);

    // Multi-source evidence check: at least 2 non-zero scores (e.g. filename_score > 0.5, folder_context_score > 0.0, content_score > 0.0, mime_score > 0.5)
    let mut evidence_sources = 0;
    if filename_score > 0.5 {
        evidence_sources += 1;
    }
    if mime_score > 0.5 {
        evidence_sources += 1;
    }
    if folder_context_score > 0.0 {
        evidence_sources += 1;
    }
    if content_score > 0.0 {
        evidence_sources += 1;
    }
    if project_marker_score > 0.0 {
        evidence_sources += 1;
    }

    let has_multisource = evidence_sources >= 2;

    // Categorize DecisionClass
    let decision_class;
    let blocked_from_apply;
    let auto_apply_allowed;
    let staging_only = false;

    // Core Autopilot Decision Logic
    // REQUISITOS OBRIGATÓRIOS PARA AutoMoveCertified:
    // - confidence >= 0.95 (hardfloor interno incondicional)
    // - risk == "low"
    // - auto_apply_allowed == true
    // - blocked_from_apply == false
    // - needs_review == false (ou rebaixado)
    // - staging_only == false
    // - categoria não pode ser Incerto
    // - destino não pode conter Revisar/Baixa_Confianca/Conflitos
    // - evidência de pelo menos duas fontes independentes
    // - rollback manifest obrigatório (será gerado)
    if is_protected || is_project || is_vault || is_script_or_exe || is_overwrite {
        decision_class = crate::decision::DecisionClass::BlockedUnsafe;
        blocked_from_apply = true;
        auto_apply_allowed = false;
    } else if proposal.risk == "high" || is_uncertain_destination {
        decision_class = crate::decision::DecisionClass::NeedsHumanReview;
        blocked_from_apply = false;
        auto_apply_allowed = false;
    } else if proposal.risk == "low"
        && final_score >= 0.95 // Hard floor incondicional para automove de 0.95
        && has_multisource
        && !is_uncertain_destination
    {
        decision_class = crate::decision::DecisionClass::AutoMoveCertified;
        blocked_from_apply = false;
        auto_apply_allowed = true;
    } else {
        decision_class = crate::decision::DecisionClass::NeedsHumanReview;
        blocked_from_apply = false;
        auto_apply_allowed = false;
    }

    // Setup rollback hint
    let rollback_hint = format!("mv \"{}\" \"{}\"", proposal.destination, proposal.source);

    // Store back in proposal
    proposal.decision_class = decision_class;
    proposal.auto_apply_allowed = auto_apply_allowed;
    proposal.blocked_from_apply = blocked_from_apply;
    proposal.staging_only = staging_only;
    proposal.content_profile = content_profile;
    proposal.context_profile = context_profile;
    proposal.project_evidence = project_evidence;
    proposal.safety_flags = safety_flags;
    proposal.rollback_hint = Some(rollback_hint);
    proposal.evidence.push(format!(
        "Multi-source components matched: {}",
        evidence_sources
    ));
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
    use chrono::Utc;

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
        let files = vec![
            create_mock_file("/home/user/.ssh/id_rsa", true),
            create_mock_file("/home/user/.gnupg/secring.gpg", true),
            create_mock_file("/home/user/.config/app/secrets.env", true),
            create_mock_file("/home/user/Downloads/public.pdf", false),
        ];

        let scan = ScanResult {
            schema_version: "1.0".to_string(),
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
            denied_count: 0,
            skipped_count: 0,
            protected_count: 3,
            inbox_count: 1,
            project_count: 0,
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

    #[test]
    fn test_legacy_schema_compatibility() {
        let legacy_json = r#"{
            "run_id": "legacy_run",
            "timestamp": "2026-05-11T12:00:00Z",
            "home_dir": "/home/user",
            "proposals": [],
            "protected_files": []
        }"#;

        let plan: Result<Plan, _> = serde_json::from_str(legacy_json);
        assert!(plan.is_ok());
        let plan = plan.unwrap();
        assert_eq!(plan.run_id, "legacy_run");
        // default fields must populate successfully
        assert_eq!(plan.files_seen, 0);
        assert!(!plan.full_home);
    }

    #[test]
    fn test_autopilot_safety_hardfloor() {
        let mut files = Vec::new();
        // Arquivo simulado de baixo risco com score moderado (0.62)
        let mut f1 = create_mock_file("/home/user/Downloads/test_doc.pdf", false);
        f1.mime = "application/pdf".to_string();
        files.push(f1);

        let scan = ScanResult {
            schema_version: "1.0".to_string(),
            run_id: "test".to_string(),
            timestamp: Utc::now(),
            home_dir: "/home/user".to_string(),
            dirs_scanned: vec!["/home/user".to_string()],
            files,
            projects: vec![],
            files_analyzed: 1,
            files_ignored: 0,
            files_error: 0,
            total_size_bytes: 1024,
            warnings: vec![],
            full_home: false,
            denied_count: 0,
            skipped_count: 0,
            protected_count: 0,
            inbox_count: 1,
            project_count: 0,
        };

        let options = PlanOptions {
            min_confidence: Some(0.50), // Usuário tentando baixar o limite
            ..Default::default()
        };
        let plan = generate_plan(&scan, &options);

        assert!(!plan.proposals.is_empty());
        let proposal = &plan.proposals[0];

        // Com score baixo (ex: sem evidências extras suficientes), ele deve ser NeedsHumanReview, nunca AutoMoveCertified
        assert_ne!(
            proposal.decision_class,
            crate::decision::DecisionClass::AutoMoveCertified
        );
        assert!(!proposal.auto_apply_allowed);

        // Mesmo se criarmos um item com confiança exatamente 0.62 de risco baixo e definirmos min_confidence = 0.50,
        // o hard floor interno (0.95) em planner.rs impede que ele se torne AutoMoveCertified.
    }

    #[test]
    fn test_uncertain_destination_never_automove() {
        let mut files = Vec::new();
        let mut f1 = create_mock_file("/home/user/Downloads/revisar_doc.pdf", false);
        f1.mime = "application/pdf".to_string();
        files.push(f1);

        let scan = ScanResult {
            schema_version: "1.0".to_string(),
            run_id: "test".to_string(),
            timestamp: Utc::now(),
            home_dir: "/home/user".to_string(),
            dirs_scanned: vec!["/home/user".to_string()],
            files,
            projects: vec![],
            files_analyzed: 1,
            files_ignored: 0,
            files_error: 0,
            total_size_bytes: 1024,
            warnings: vec![],
            full_home: false,
            denied_count: 0,
            skipped_count: 0,
            protected_count: 0,
            inbox_count: 1,
            project_count: 0,
        };

        let options = PlanOptions::default();
        let plan = generate_plan(&scan, &options);

        assert!(!plan.proposals.is_empty());
        let proposal = &plan.proposals[0];

        // Destino incerto (como contendo "revisar") nunca deve ser AutoMoveCertified
        assert_ne!(
            proposal.decision_class,
            crate::decision::DecisionClass::AutoMoveCertified
        );
        assert!(!proposal.auto_apply_allowed);
    }
}
