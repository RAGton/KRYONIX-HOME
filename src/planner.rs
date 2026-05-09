use serde::{Deserialize, Serialize};

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
pub fn generate_plan(scan: &ScanResult, rename_suggestions: bool) -> Plan {
    let mut proposals = Vec::new();

    for file in &scan.files {
        if file.status != FileStatus::Analyzed {
            continue;
        }

        if let Some(mut proposal) = classify_file(file) {
            if rename_suggestions {
                if let Some(suggestion) = crate::naming::suggest_rename(file) {
                    // Combinar move e rename em uma unica proposta
                    proposal.action = "rename".to_string();
                    proposal.new_filename = Some(suggestion.suggested_filename);
                    proposal.rules_applied = Some(suggestion.rules_applied);
                    proposal.naming_profile = Some(suggestion.naming_profile);
                    proposal.needs_review = proposal.needs_review || suggestion.needs_review;

                    // Ajustar risk e confidence
                    if suggestion.risk == "high" {
                        proposal.risk = "high".to_string();
                    } else if suggestion.risk == "medium" && proposal.risk == "low" {
                        proposal.risk = "medium".to_string();
                    }
                    proposal.confidence =
                        (proposal.confidence + suggestion.confidence as f64) / 2.0;
                    proposal.reason = format!("{} | {}", proposal.reason, suggestion.reason);
                }
            }
            proposals.push(proposal);
        } else if rename_suggestions {
            if let Some(suggestion) = crate::naming::suggest_rename(file) {
                // Arquivo não seria movido, mas vai ser renomeado no lugar
                let new_dir = std::path::Path::new(&file.path)
                    .parent()
                    .and_then(|p| p.strip_prefix(&scan.home_dir).ok())
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();

                proposals.push(PlanProposal {
                    action: "rename".to_string(),
                    risk: suggestion.risk,
                    confidence: suggestion.confidence as f64,
                    old_path: file.path.clone(),
                    new_dir,
                    reason: suggestion.reason,
                    needs_review: suggestion.needs_review,
                    new_filename: Some(suggestion.suggested_filename),
                    rules_applied: Some(suggestion.rules_applied),
                    naming_profile: Some(suggestion.naming_profile),
                });
            }
        }
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
    })
}
