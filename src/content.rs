use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct ContentProfile {
    pub summary: Option<String>,
    pub keywords: Vec<String>,
    pub language: Option<String>,
}

pub fn analyze_file_content(path: &Path) -> Option<ContentProfile> {
    // Proteção rigorosa contra leitura de diretórios ou arquivos confidenciais
    let path_str = path.to_string_lossy().to_lowercase();
    if path_str.contains("/.ssh/")
        || path_str.contains("/.gnupg/")
        || path_str.contains("/.config/")
        || path_str.contains("/brain.env")
        || path_str.contains("/.env")
        || path_str.contains(".key")
        || path_str.contains(".pem")
    {
        return None;
    }

    let ext = path.extension()?.to_str()?.to_lowercase();
    match ext.as_str() {
        "txt" | "md" | "tex" | "latex" => {
            let file = File::open(path).ok()?;
            let mut reader = BufReader::new(file).take(65536); // Max 64 KiB
            let mut content = String::new();
            if reader.read_to_string(&mut content).is_ok() {
                let mut keywords = Vec::new();
                for kw in &[
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
                ] {
                    if content.to_lowercase().contains(kw) {
                        keywords.push(kw.to_string());
                    }
                }
                let clean_content = content.replace('\n', " ").trim().to_string();
                let summary = if clean_content.chars().count() > 120 {
                    Some(format!(
                        "{}...",
                        clean_content.chars().take(120).collect::<String>()
                    ))
                } else {
                    Some(clean_content)
                };
                return Some(ContentProfile {
                    summary,
                    keywords,
                    language: Some("Text".to_string()),
                });
            }
        }
        "ipynb" => {
            let file = File::open(path).ok()?;
            let reader = BufReader::new(file);
            if let Ok(v) = serde_json::from_reader::<_, serde_json::Value>(reader) {
                let mut keywords = Vec::new();
                let mut summary_parts = Vec::new();
                if let Some(cells) = v.get("cells").and_then(|c| c.as_array()) {
                    for cell in cells.iter().take(10) {
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
                            for kw in &[
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
                            ] {
                                if text_lower.contains(kw) {
                                    keywords.push(kw.to_string());
                                }
                            }
                            if text.starts_with('#') {
                                summary_parts.push(text.trim().to_string());
                            }
                        }
                    }
                }
                keywords.dedup();
                let summary = if !summary_parts.is_empty() {
                    Some(summary_parts.join(" | "))
                } else {
                    Some("Jupyter Notebook".to_string())
                };
                return Some(ContentProfile {
                    summary,
                    keywords,
                    language: Some("Python/Jupyter".to_string()),
                });
            }
        }
        "pdf" => {
            if let Ok(output) = Command::new("pdftotext")
                .arg("-l")
                .arg("3")
                .arg(path)
                .arg("-")
                .output()
            {
                if output.status.success() {
                    if let Ok(content) = String::from_utf8(output.stdout) {
                        let mut keywords = Vec::new();
                        for kw in &[
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
                        ] {
                            if content.to_lowercase().contains(kw) {
                                keywords.push(kw.to_string());
                            }
                        }
                        let clean_content = content.replace('\n', " ").trim().to_string();
                        let summary = if clean_content.chars().count() > 120 {
                            Some(format!(
                                "{}...",
                                clean_content.chars().take(120).collect::<String>()
                            ))
                        } else {
                            Some(clean_content)
                        };
                        return Some(ContentProfile {
                            summary,
                            keywords,
                            language: Some("PDF Document".to_string()),
                        });
                    }
                }
            }
        }
        _ => {}
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_confidentiality_protection() {
        // Paths containing confidential patterns must be blocked and return None immediately
        let blocked_paths = [
            Path::new("/home/rocha/.ssh/id_rsa.pub"),
            Path::new("/home/rocha/.gnupg/trustdb.gpg"),
            Path::new("/etc/kryonix/brain.env"),
            Path::new("/home/rocha/Documents/private_key.pem"),
            Path::new("/home/rocha/.config/app/secrets.json"),
        ];

        for path in &blocked_paths {
            assert!(
                analyze_file_content(path).is_none(),
                "Path {:?} should have been blocked for confidentiality",
                path
            );
        }
    }

    #[test]
    fn test_unsupported_extensions_return_none() {
        let path = Path::new("unsupported_file.xyz");
        assert!(analyze_file_content(path).is_none());
    }

    #[test]
    fn test_analyze_text_file_success() {
        let dir = tempdir().expect("Failed to create temp dir");
        let file_path = dir.path().join("comprovante_pix_banco.txt");

        let content = "Aqui está o comprovante de pagamento via pix enviado para o banco de teste.";
        let mut file = File::create(&file_path).expect("Failed to create temp file");
        file.write_all(content.as_bytes())
            .expect("Failed to write to temp file");

        let profile = analyze_file_content(&file_path)
            .expect("Expected a valid ContentProfile for text file");

        assert_eq!(profile.language.as_deref(), Some("Text"));
        assert!(profile.keywords.contains(&"pix".to_string()));
        assert!(profile.keywords.contains(&"comprovante".to_string()));
        assert!(profile.keywords.contains(&"pagamento".to_string()));
        assert!(profile.summary.is_some());
        assert!(profile.summary.unwrap().contains("Aqui está o comprovante"));
    }

    #[test]
    fn test_analyze_jupyter_notebook_success() {
        let dir = tempdir().expect("Failed to create temp dir");
        let file_path = dir.path().join("data_analysis.ipynb");

        let json_content = serde_json::json!({
            "cells": [
                {
                    "cell_type": "code",
                    "execution_count": 1,
                    "metadata": {},
                    "outputs": [],
                    "source": [
                        "import pandas as pd\n",
                        "import numpy as np\n",
                        "import matplotlib.pyplot as plt\n"
                    ]
                }
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 2
        });

        let mut file = File::create(&file_path).expect("Failed to create temp file");
        file.write_all(json_content.to_string().as_bytes())
            .expect("Failed to write to temp file");

        let profile = analyze_file_content(&file_path)
            .expect("Expected a valid ContentProfile for ipynb file");

        assert_eq!(profile.language.as_deref(), Some("Python/Jupyter"));
        assert!(profile.keywords.contains(&"pandas".to_string()));
        assert!(profile.keywords.contains(&"numpy".to_string()));
        assert!(profile.summary.is_some());
    }
}
