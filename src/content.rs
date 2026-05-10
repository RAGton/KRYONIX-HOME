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
