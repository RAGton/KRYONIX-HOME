use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentProfile {
    pub path: String,
    pub content_read: bool,
    pub extractor: String,
    pub content_kind: String,
    pub sample_text: Option<String>,
    pub keywords: Vec<String>,
    pub title_candidates: Vec<String>,
    pub warnings: Vec<String>,
    pub truncated: bool,
    pub bytes_read: u64,
}

pub fn analyze_content_safe(path: &Path, limit_bytes: u64) -> Result<ContentProfile> {
    let path_str = path.to_string_lossy().to_string();

    // Proteção rigorosa contra leitura de diretórios ou arquivos confidenciais
    if let Some(reason) = crate::metadata::is_protected_path(path) {
        anyhow::bail!("Access blocked for confidentiality: {}", reason);
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mut profile = ContentProfile {
        path: path_str,
        content_read: false,
        extractor: "none".to_string(),
        content_kind: "unknown".to_string(),
        sample_text: None,
        keywords: Vec::new(),
        title_candidates: Vec::new(),
        warnings: Vec::new(),
        truncated: false,
        bytes_read: 0,
    };

    let keywords_to_match = &[
        "pix",
        "comprovante",
        "pagamento",
        "boleto",
        "contrato",
        "rg",
        "cpf",
        "extrato",
        "declaracao",
        "recibo",
        "certificado",
        "diploma",
        "kryonix",
        "ragos",
        "nixos",
        "nix",
        "import",
        "plt",
        "pandas",
        "numpy",
        "torch",
        "sklearn",
        "tensorflow",
        "model",
        "train",
        "dataset",
    ];

    match ext.as_str() {
        "txt" | "md" | "tex" | "csv" | "json" | "yaml" | "yml" | "toml" | "nix" | "py" | "rs"
        | "sh" | "go" | "js" | "ts" | "html" | "css" => {
            let file = File::open(path)?;
            let size = file.metadata()?.len();
            let to_read = std::cmp::min(size, limit_bytes);

            let mut reader = BufReader::new(file).take(to_read);
            let mut content = String::new();

            match reader.read_to_string(&mut content) {
                Ok(bytes) => {
                    profile.content_read = true;
                    profile.extractor = "text_reader".to_string();
                    profile.content_kind = "text/plain".to_string();
                    profile.bytes_read = bytes as u64;
                    profile.truncated = size > to_read;

                    let mut matched_keywords = Vec::new();
                    let content_lower = content.to_lowercase();
                    for kw in keywords_to_match {
                        if content_lower.contains(kw) {
                            matched_keywords.push(kw.to_string());
                        }
                    }
                    profile.keywords = matched_keywords;

                    // Title candidates
                    let mut titles = Vec::new();
                    for line in content.lines().take(50) {
                        let trimmed = line.trim();
                        if trimmed.starts_with('#') {
                            titles.push(trimmed.trim_start_matches('#').trim().to_string());
                        } else if trimmed.starts_with("//") || trimmed.starts_with("/*") {
                            // code comment title candidate
                            let clean = trimmed
                                .trim_start_matches('/')
                                .trim_start_matches('*')
                                .trim();
                            if !clean.is_empty() && clean.len() < 100 {
                                titles.push(clean.to_string());
                            }
                        }
                    }
                    if titles.is_empty() {
                        if let Some(first_line) = content.lines().find(|l| !l.trim().is_empty()) {
                            if first_line.len() < 80 {
                                titles.push(first_line.trim().to_string());
                            }
                        }
                    }
                    profile.title_candidates = titles;

                    let clean_content = content.replace('\n', " ").trim().to_string();
                    profile.sample_text = if clean_content.chars().count() > 120 {
                        Some(format!(
                            "{}...",
                            clean_content.chars().take(120).collect::<String>()
                        ))
                    } else {
                        Some(clean_content)
                    };
                }
                Err(e) => {
                    profile
                        .warnings
                        .push(format!("Failed to read text file content: {}", e));
                }
            }
        }
        "ipynb" => {
            let file = File::open(path)?;
            let size = file.metadata()?.len();
            let reader = BufReader::new(file);

            if let Ok(v) = serde_json::from_reader::<_, serde_json::Value>(reader) {
                profile.content_read = true;
                profile.extractor = "ipynb_json_parser".to_string();
                profile.content_kind = "notebook".to_string();
                profile.bytes_read = std::cmp::min(size, limit_bytes);

                let mut matched_keywords = Vec::new();
                let mut titles = Vec::new();
                let mut cell_texts = Vec::new();

                if let Some(cells) = v.get("cells").and_then(|c| c.as_array()) {
                    for cell in cells.iter().take(20) {
                        if let Some(source) = cell.get("source") {
                            let text = if let Some(arr) = source.as_array() {
                                arr.iter()
                                    .filter_map(|s| s.as_str())
                                    .collect::<Vec<_>>()
                                    .join("")
                            } else if let Some(s) = source.as_str() {
                                s.to_string()
                            } else {
                                String::new()
                            };

                            let text_lower = text.to_lowercase();
                            for kw in keywords_to_match {
                                if text_lower.contains(kw) {
                                    matched_keywords.push(kw.to_string());
                                }
                            }

                            for line in text.lines() {
                                let trimmed = line.trim();
                                if trimmed.starts_with('#') {
                                    titles.push(trimmed.trim_start_matches('#').trim().to_string());
                                }
                            }

                            let clean_cell = text.replace('\n', " ");
                            if !clean_cell.trim().is_empty() && cell_texts.len() < 5 {
                                cell_texts.push(clean_cell.trim().to_string());
                            }
                        }
                    }
                }

                matched_keywords.dedup();
                profile.keywords = matched_keywords;
                profile.title_candidates = titles;

                if !cell_texts.is_empty() {
                    let joined = cell_texts.join(" | ");
                    profile.sample_text = if joined.chars().count() > 120 {
                        Some(format!(
                            "{}...",
                            joined.chars().take(120).collect::<String>()
                        ))
                    } else {
                        Some(joined)
                    };
                }
            } else {
                profile
                    .warnings
                    .push("Failed to parse Jupyter Notebook JSON structure".to_string());
            }
        }
        "pdf" => {
            // Se pdftotext existir: extrair até 3 páginas ou 32 KiB
            let size_limit = std::cmp::min(32768, limit_bytes);
            match Command::new("pdftotext")
                .arg("-l")
                .arg("3")
                .arg(path)
                .arg("-")
                .output()
            {
                Ok(output) if output.status.success() => {
                    if let Ok(content) = String::from_utf8(output.stdout) {
                        profile.content_read = true;
                        profile.extractor = "pdftotext_cli".to_string();
                        profile.content_kind = "pdf".to_string();
                        profile.bytes_read = std::cmp::min(content.len() as u64, size_limit);
                        profile.truncated = content.len() as u64 > size_limit;

                        let content_to_analyze = if content.len() as u64 > size_limit {
                            content
                                .chars()
                                .take(size_limit as usize)
                                .collect::<String>()
                        } else {
                            content.clone()
                        };

                        let content_lower = content_to_analyze.to_lowercase();
                        let mut matched_keywords = Vec::new();
                        for kw in keywords_to_match {
                            if content_lower.contains(kw) {
                                matched_keywords.push(kw.to_string());
                            }
                        }
                        profile.keywords = matched_keywords;

                        // Title candidates from first lines
                        let mut titles = Vec::new();
                        for line in content_to_analyze.lines().take(10) {
                            let trimmed = line.trim();
                            if !trimmed.is_empty() && trimmed.len() > 5 && trimmed.len() < 80 {
                                titles.push(trimmed.to_string());
                            }
                        }
                        profile.title_candidates = titles;

                        let clean_content =
                            content_to_analyze.replace('\n', " ").trim().to_string();
                        profile.sample_text = if clean_content.chars().count() > 120 {
                            Some(format!(
                                "{}...",
                                clean_content.chars().take(120).collect::<String>()
                            ))
                        } else {
                            Some(clean_content)
                        };
                    } else {
                        profile
                            .warnings
                            .push("pdftotext stdout was not valid UTF-8".to_string());
                    }
                }
                _ => {
                    profile
                        .warnings
                        .push("pdftotext utility not available or failed".to_string());
                }
            }
        }
        _ => {}
    }

    Ok(profile)
}

pub fn analyze_file_content(path: &Path) -> Option<ContentProfile> {
    analyze_content_safe(path, 65536).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_confidentiality_protection() {
        let blocked_paths = [
            Path::new("/home/rocha/.ssh/id_rsa.pub"),
            Path::new("/home/rocha/.gnupg/trustdb.gpg"),
            Path::new("/etc/kryonix/brain.env"),
            Path::new("/home/rocha/Documents/private_key.pem"),
            Path::new("/home/rocha/.config/app/secrets.json"),
        ];

        for path in &blocked_paths {
            assert!(
                analyze_content_safe(path, 1024).is_err(),
                "Path {:?} should have been blocked for confidentiality",
                path
            );
        }
    }

    #[test]
    fn test_unsupported_extensions_return_empty_profile() {
        let path = Path::new("unsupported_file.xyz");
        let profile = analyze_content_safe(path, 1024).unwrap();
        assert!(!profile.content_read);
    }

    #[test]
    fn test_analyze_text_file_success() {
        let dir = tempdir().expect("Failed to create temp dir");
        let file_path = dir.path().join("comprovante_pix_banco.txt");

        let content = "Aqui está o comprovante de pagamento via pix enviado para o banco de teste.";
        let mut file = File::create(&file_path).expect("Failed to create temp file");
        file.write_all(content.as_bytes())
            .expect("Failed to write to temp file");

        let profile = analyze_content_safe(&file_path, 1024)
            .expect("Expected a valid ContentProfile for text file");

        assert_eq!(profile.content_kind.as_str(), "text/plain");
        assert!(profile.keywords.contains(&"pix".to_string()));
        assert!(profile.keywords.contains(&"comprovante".to_string()));
        assert!(profile.keywords.contains(&"pagamento".to_string()));
        assert!(profile.sample_text.is_some());
    }
}
