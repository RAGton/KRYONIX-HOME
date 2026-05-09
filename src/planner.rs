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
    pub needs_review: bool,
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
}

/// Plano completo de organização (dry-run).
#[derive(Debug, Serialize, Deserialize)]
pub struct Plan {
    pub run_id: String,
    pub mode: String,
    pub home_dir: String,
    pub files_seen: usize,
    pub proposals: Vec<PlanProposal>,
}

/// Gera um plano de organização determinístico baseado em MIME/extensão e, opcionalmente, sugere renomeações.
/// Este plano é SOMENTE informativo (dry-run). Nenhuma ação é executada.
pub fn generate_plan(
    scan: &ScanResult,
    rename_suggestions: bool,
    taxonomy_suggestions: bool,
    taxonomy_config_path: Option<&str>,
    include_large_files: bool,
    safe_only: bool,
    review_only: bool,
) -> Plan {
    let mut proposals = Vec::new();
    let taxonomy_config = crate::taxonomy::load_taxonomy_config(taxonomy_config_path);

    for file in &scan.files {
        if file.status != FileStatus::Analyzed {
            continue;
        }

        if file.path.contains("/.git/") || file.path.contains("/node_modules/") {
            continue;
        }

        // Limite de 2 GiB para arquivos grandes
        if file.size_bytes > 2_147_483_648 && !include_large_files {
            continue;
        }

        let mut proposal = if taxonomy_suggestions {
            let cat = crate::taxonomy::suggest_category_config(file, &taxonomy_config);
            PlanProposal {
                action: "move".to_string(),
                risk: cat.risk.clone(),
                confidence: cat.confidence as f64,
                old_path: file.path.clone(),
                new_dir: cat.relative_dir.to_string_lossy().to_string(),
                reason: cat.reason.clone(),
                needs_review: cat.needs_review,
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
        if rename_suggestions {
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

        // Aplicar filtro --safe-only se habilitado
        if safe_only {
            let ext_lower = file.extension.to_lowercase();
            let is_media = file.mime.starts_with("image/")
                || file.mime.starts_with("video/")
                || file.mime.starts_with("audio/")
                || matches!(
                    ext_lower.as_str(),
                    "jpg" | "jpeg" | "png" | "gif" | "mp4" | "mkv" | "mp3" | "flac"
                );

            if proposal.risk == "high"
                || proposal.category_id == Some("inbox.conflicts".to_string())
                || ext_lower.is_empty()
                || is_media
                || proposal.taxonomy_score.unwrap_or(1.0) < 0.75
            {
                continue; // Pular item inseguro
            }
        }

        // Aplicar filtro --review-only se habilitado
        if review_only && !proposal.needs_review {
            continue; // Pular item que não precisa de revisão
        }

        proposals.push(proposal);
    }

    Plan {
        run_id: scan.run_id.clone(),
        mode: "dry-run".to_string(),
        home_dir: scan.home_dir.clone(),
        files_seen: scan.files_analyzed,
        proposals,
    }
}

/// Classifica um arquivo por MIME/extensão e sugere destino.
fn classify_file(file: &FileMetadata) -> Option<PlanProposal> {
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
        needs_review: confidence < 0.70,
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
    })
}
