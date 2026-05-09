use crate::metadata::FileMetadata;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const TAXONOMY_PROFILE: &str = "kryonix-home-taxonomy-v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxonomyCategory {
    pub id: String,
    pub label: String,
    pub relative_dir: PathBuf,
    pub confidence: f32,
    pub risk: String,
    pub needs_review: bool,
    pub reason: String,
    pub rules_applied: Vec<String>,
    pub matched_keywords: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_categories: Option<Vec<String>>,
    pub already_organized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxonomyConfig {
    pub profile: String,
    pub fallback_dir: String,
    pub categories: Vec<CategoryConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryConfig {
    pub id: String,
    pub label: String,
    pub dir: String,
    pub keywords: Vec<String>,
    pub extensions: Option<Vec<String>>,
    pub risk: Option<String>,
}

const DEFAULT_TAXONOMY_TOML: &str = r#"
[profile]
name = "kryonix-home-taxonomy-v1"
fallback_dir = "Documentos/00_Inbox/Revisar"

# Administrativo
[[category]]
id = "admin.identificacao"
label = "Identificação"
dir = "Documentos/Administrativo/Identificacao"
keywords = ["rg", "cpf", "cnh", "identidade", "titulo eleitor", "certidao", "nascimento", "casamento"]
extensions = ["pdf", "jpg", "png"]
risk = "medium"

[[category]]
id = "admin.contratos"
label = "Contratos"
dir = "Documentos/Administrativo/Contratos"
keywords = ["contrato", "termo", "acordo", "locacao", "aluguel", "assinatura"]
extensions = ["pdf", "docx", "odt"]
risk = "medium"

[[category]]
id = "admin.comprovantes"
label = "Comprovantes"
dir = "Documentos/Administrativo/Comprovantes"
keywords = ["comprovante", "protocolo", "declaracao", "recibo"]
extensions = ["pdf", "jpg", "png"]
risk = "low"

[[category]]
id = "admin.certificados"
label = "Certificados"
dir = "Documentos/Administrativo/Certificados"
keywords = ["certificado", "certificado digital", "curso", "conclusao", "conclusão", "diploma"]
extensions = ["pdf", "jpg", "png"]
risk = "low"

[[category]]
id = "admin.garantias"
label = "Garantias"
dir = "Documentos/Administrativo/Garantias"
keywords = ["garantia", "garantia estendida", "comprovante garantia", "rma", "serial", "nota garantia"]
extensions = ["pdf", "jpg", "png"]
risk = "low"

# Financeiro
[[category]]
id = "financeiro.bancos"
label = "Bancos"
dir = "Documentos/Financeiro/Bancos"
keywords = ["banco", "pix", "extrato", "conta", "agencia", "transferencia"]
extensions = ["pdf", "csv", "xlsx", "txt"]
risk = "medium"

[[category]]
id = "financeiro.boletos"
label = "Boletos"
dir = "Documentos/Financeiro/Boletos"
keywords = ["boleto", "pagamento", "vencimento"]
extensions = ["pdf"]
risk = "medium"

[[category]]
id = "financeiro.faturas"
label = "Faturas"
dir = "Documentos/Financeiro/Faturas"
keywords = ["fatura", "cartao", "cartão", "nubank", "inter", "mercado pago", "energia", "agua", "água", "internet"]
extensions = ["pdf"]
risk = "medium"

[[category]]
id = "financeiro.notas_fiscais"
label = "Notas Fiscais"
dir = "Documentos/Financeiro/Notas_Fiscais"
keywords = ["nota fiscal", "nf", "nfe", "danfe", "cupom fiscal"]
extensions = ["pdf", "xml"]
risk = "low"

[[category]]
id = "financeiro.impostos"
label = "Impostos"
dir = "Documentos/Financeiro/Impostos"
keywords = ["irpf", "imposto", "receita federal", "darf", "inss"]
extensions = ["pdf", "xml", "txt"]
risk = "medium"

# Trabalho
[[category]]
id = "trabalho.supersoft"
label = "Supersoft"
dir = "Documentos/Trabalho/Supersoft"
keywords = ["supersoft"]
extensions = ["pdf", "docx", "xlsx", "txt"]
risk = "medium"

[[category]]
id = "trabalho.clientes"
label = "Clientes"
dir = "Documentos/Trabalho/Clientes"
keywords = ["cliente", "proposta", "orcamento", "orçamento", "atendimento", "ordem de serviço", "ordem servico"]
extensions = ["pdf", "docx"]
risk = "medium"

[[category]]
id = "trabalho.infraestrutura"
label = "Infraestrutura"
dir = "Documentos/Trabalho/Infraestrutura"
keywords = ["servidor", "backup", "mikrotik", "switch", "roteador", "datacenter"]
extensions = ["pdf", "txt", "conf"]
risk = "medium"

# Estudos
[[category]]
id = "estudos.nixos"
label = "NixOS"
dir = "Documentos/Estudos/NixOS"
keywords = ["nixos", "nix", "flake", "home-manager"]
extensions = ["md", "nix", "txt"]
risk = "low"

[[category]]
id = "estudos.linux"
label = "Linux"
dir = "Documentos/Estudos/Linux"
keywords = ["linux", "kernel", "systemd", "shell", "bash"]
extensions = ["md", "sh", "txt"]
risk = "low"

[[category]]
id = "estudos.redes"
label = "Redes"
dir = "Documentos/Estudos/Redes"
keywords = ["rede", "tcp", "ip", "dns", "dhcp", "vlan", "vpn"]
extensions = ["md", "pdf", "txt"]
risk = "low"

[[category]]
id = "estudos.proxmox"
label = "Proxmox"
dir = "Documentos/Estudos/Proxmox"
keywords = ["proxmox", "pve", "vm", "lxc", "cluster"]
extensions = ["md", "txt"]
risk = "low"

[[category]]
id = "estudos.opnsense"
label = "OPNsense"
dir = "Documentos/Estudos/OPNsense"
keywords = ["opnsense", "wireguard", "pfsense"]
extensions = ["md", "txt"]
risk = "low"

[[category]]
id = "estudos.ia_llm_rag"
label = "IA, LLM e RAG"
dir = "Documentos/Estudos/IA_LLM_RAG"
keywords = ["ia", "llm", "rag", "cag", "graphrag", "ollama", "neo4j", "embedding"]
extensions = ["md", "pdf", "py", "ipynb"]
risk = "low"

[[category]]
id = "estudos.rust"
label = "Rust"
dir = "Documentos/Estudos/Rust"
keywords = ["rust", "cargo", "crate"]
extensions = ["md", "rs", "toml"]
risk = "low"

[[category]]
id = "estudos.python"
label = "Python"
dir = "Documentos/Estudos/Python"
keywords = ["python", "pip", "uv", "venv"]
extensions = ["md", "py", "ipynb"]
risk = "low"

# Projetos
[[category]]
id = "projetos.kryonix"
label = "Kryonix"
dir = "Documentos/Projetos/Kryonix"
keywords = ["kryonix", "brain", "lightrag", "graphrag", "vault"]
extensions = ["md", "nix", "json"]
risk = "low"

# Academico
[[category]]
id = "academico.trabalhos"
label = "Trabalhos"
dir = "Documentos/Academico/Trabalhos"
keywords = ["trabalho", "atividade", "avaliacao", "prova", "exercicio"]
extensions = ["pdf", "docx", "md"]
risk = "low"

[[category]]
id = "academico.artigos"
label = "Artigos"
dir = "Documentos/Academico/Artigos"
keywords = ["artigo", "paper", "pesquisa", "tcc"]
extensions = ["pdf", "md"]
risk = "low"

# Mídia
[[category]]
id = "imagens.screenshots"
label = "Screenshots"
dir = "Imagens/Screenshots"
keywords = ["screenshot", "captura", "print", "screen"]
extensions = ["png", "jpg"]
risk = "low"

[[category]]
id = "imagens.wallpapers"
label = "Wallpapers"
dir = "Imagens/Wallpapers"
keywords = ["wallpaper", "background", "fundo"]
extensions = ["jpg", "jpeg", "png"]
risk = "low"

[[category]]
id = "imagens.digitalizados"
label = "Documentos Digitalizados"
dir = "Imagens/Documentos_Digitalizados"
keywords = ["scan", "scanned", "digitalizado", "documento"]
extensions = ["jpg", "jpeg", "png", "pdf"]
risk = "low"

[[category]]
id = "videos.capturas"
label = "Capturas de Tela (Vídeo)"
dir = "Vídeos/Capturas"
keywords = ["screencast", "gravação", "gravacao", "captura"]
extensions = ["mp4", "webm", "mkv"]
risk = "low"

[[category]]
id = "videos.aulas"
label = "Aulas"
dir = "Vídeos/Aulas"
keywords = ["aula", "curso", "treinamento"]
extensions = ["mp4", "mkv", "avi"]
risk = "low"
# Projetos
[[category]]
id = "projetos.kryonix"
label = "Kryonix Ecosystem"
dir = "Documentos/Projetos/Kryonix"
keywords = ["kryonix", "home-brain", "caelestia"]

[[category]]
id = "projetos.ragos"
label = "RAGOS Framework"
dir = "Documentos/Projetos/RAGOS"
keywords = ["ragos", "rag", "rag-os"]

[[category]]
id = "projetos.nixos"
label = "NixOS & Flakes"
dir = "Documentos/Projetos/NixOS"
keywords = ["nixos", "flake", "home-manager"]

[[category]]
id = "projetos.ia"
label = "IA & Machine Learning"
dir = "Documentos/Projetos/IA"
keywords = ["ia", "ai", "llm", "training", "datasets"]

[[category]]
id = "projetos.infra"
label = "Infraestrutura & Lab"
dir = "Documentos/Projetos/Infra"
keywords = ["proxmox", "server", "network", "opnsense"]

[[category]]
id = "projetos.windows"
label = "Windows & Legacy"
dir = "Documentos/Projetos/Windows"
keywords = ["windows", "ativador", "office"]

[[category]]
id = "projetos.sandbox"
label = "Projetos Sandbox"
dir = "Documentos/Projetos/Sandbox"
keywords = []
"#;

pub fn parse_taxonomy_toml(content: &str) -> TaxonomyConfig {
    let mut config = TaxonomyConfig {
        profile: "kryonix-home-taxonomy-v1".to_string(),
        fallback_dir: "Documentos/00_Inbox/Revisar".to_string(),
        categories: Vec::new(),
    };

    let mut current_category: Option<CategoryConfig> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line == "[[category]]" {
            if let Some(cat) = current_category.take() {
                config.categories.push(cat);
            }
            current_category = Some(CategoryConfig {
                id: String::new(),
                label: String::new(),
                dir: String::new(),
                keywords: Vec::new(),
                extensions: None,
                risk: None,
            });
            continue;
        }

        if line.starts_with("[profile]") {
            if let Some(cat) = current_category.take() {
                config.categories.push(cat);
            }
            continue;
        }

        if let Some(idx) = line.find('=') {
            let key = line[..idx].trim();
            let val = line[idx + 1..].trim();
            let unquoted = val.trim_matches('"').trim_matches('\'');

            if let Some(ref mut cat) = current_category {
                match key {
                    "id" => cat.id = unquoted.to_string(),
                    "label" => cat.label = unquoted.to_string(),
                    "dir" => cat.dir = unquoted.to_string(),
                    "risk" => cat.risk = Some(unquoted.to_string()),
                    "keywords" if val.starts_with('[') && val.ends_with(']') => {
                        let list = &val[1..val.len() - 1];
                        cat.keywords = list
                            .split(',')
                            .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    "extensions" if val.starts_with('[') && val.ends_with(']') => {
                        let list = &val[1..val.len() - 1];
                        let exts: Vec<String> = list
                            .split(',')
                            .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                        cat.extensions = Some(exts);
                    }
                    _ => {}
                }
            } else {
                match key {
                    "name" => config.profile = unquoted.to_string(),
                    "fallback_dir" => config.fallback_dir = unquoted.to_string(),
                    _ => {}
                }
            }
        }
    }

    if let Some(cat) = current_category {
        config.categories.push(cat);
    }

    config
}

pub fn load_taxonomy_config(specific_path: Option<&str>) -> TaxonomyConfig {
    let mut paths_to_try = Vec::new();
    if let Some(p) = specific_path {
        paths_to_try.push(PathBuf::from(p));
    }
    paths_to_try.push(PathBuf::from("/etc/kryonix/config/home-taxonomy.toml"));
    if let Some(home) = dirs::home_dir() {
        paths_to_try.push(home.join(".config/kryonix/home-taxonomy.toml"));
    }

    for path in paths_to_try {
        if path.is_file() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                return parse_taxonomy_toml(&content);
            }
        }
    }

    parse_taxonomy_toml(DEFAULT_TAXONOMY_TOML)
}

pub fn suggest_category(file: &FileMetadata) -> TaxonomyCategory {
    let config = load_taxonomy_config(None);
    suggest_category_config(file, &config)
}

pub fn suggest_category_config(file: &FileMetadata, config: &TaxonomyConfig) -> TaxonomyCategory {
    let basename_lower = file.filename.to_lowercase();
    let ext_lower = file.extension.to_lowercase();

    let is_image = file.mime.starts_with("image/")
        || matches!(ext_lower.as_str(), "jpg" | "jpeg" | "png" | "gif" | "webp");
    let is_video = file.mime.starts_with("video/")
        || matches!(ext_lower.as_str(), "mp4" | "mkv" | "avi" | "mov" | "webm");
    let is_audio = file.mime.starts_with("audio/")
        || matches!(ext_lower.as_str(), "mp3" | "flac" | "ogg" | "wav");

    let mut best_score = 0.0_f32;
    let mut best_candidates: Vec<&CategoryConfig> = Vec::new();
    let mut best_matches = Vec::new();

    for def in &config.categories {
        // Ignorar categorias de mídia se não corresponderem ao tipo
        if def.dir.starts_with("Imagens/") && !is_image {
            continue;
        }
        if def.dir.starts_with("Vídeos/") && !is_video {
            continue;
        }
        if def.dir.starts_with("Músicas/") && !is_audio {
            continue;
        }

        // Filtrar por extensão se estiver especificado na categoria
        if let Some(ref exts) = def.extensions {
            if !ext_lower.is_empty() && !exts.contains(&ext_lower) {
                continue; // Extensão não permitida para esta categoria
            }
        }

        let mut score = 0.0_f32;
        let mut matched = Vec::new();

        for kw in &def.keywords {
            if basename_lower.contains(&kw.to_lowercase()) {
                score += 0.50;
                matched.push(kw.clone());
            }
        }

        // Penalidade por media vs doc
        if def.dir.starts_with("Documentos/") && (is_image || is_video || is_audio) {
            score -= 0.15;
        }

        // Boost para mídia em pastas de mídia sem keywords exatas se o nome remeter a isso (ex: "Screenshot de...")
        if def.id == "imagens.screenshots" && is_image && basename_lower.starts_with("screenshot") {
            score += 0.60;
        }

        if score > 0.0 {
            if (score - best_score).abs() < f32::EPSILON {
                best_candidates.push(def);
            } else if score > best_score {
                best_score = score;
                best_candidates.clear();
                best_candidates.push(def);
                best_matches = matched;
            }
        }
    }

    // Tratamento de empates (Tie-Breaking)
    if best_candidates.len() > 1 {
        let candidate_ids: Vec<String> = best_candidates.iter().map(|c| c.id.clone()).collect();
        return TaxonomyCategory {
            id: "inbox.conflicts".to_string(),
            label: "Conflito de Categorias".to_string(),
            relative_dir: PathBuf::from("Documentos/00_Inbox/Conflitos"),
            confidence: best_score.clamp(0.0, 1.0),
            risk: "medium".to_string(),
            needs_review: true,
            reason: format!(
                "Empate de pontuação ({:.2}) entre múltiplas categorias: {}",
                best_score,
                candidate_ids.join(", ")
            ),
            rules_applied: vec!["taxonomy_tie_breaking".to_string()],
            matched_keywords: best_matches,
            candidate_categories: Some(candidate_ids),
            already_organized: false,
        };
    }

    if let Some(def) = best_candidates.first() {
        if best_score >= 0.45 {
            // Regras de confiança baseadas nas heurísticas sugeridas:
            // PDFs, DOCX, Imagem, Vídeo e Áudio sempre precisam de revisão na 3B
            let is_restricted_format =
                matches!(ext_lower.as_str(), "pdf" | "docx" | "doc" | "xlsx" | "xls")
                    || is_image
                    || is_video
                    || is_audio
                    || ext_lower.is_empty();

            let needs_review = if is_restricted_format {
                true
            } else {
                best_score < 0.75
            };

            let risk = if ext_lower.is_empty() {
                "high".to_string()
            } else if needs_review {
                "medium".to_string()
            } else {
                "low".to_string()
            };

            return TaxonomyCategory {
                id: def.id.clone(),
                label: def.label.clone(),
                relative_dir: PathBuf::from(&def.dir),
                confidence: best_score.clamp(0.0, 1.0),
                risk,
                needs_review,
                reason: format!(
                    "Classificado por palavras-chave: {}",
                    best_matches.join(", ")
                ),
                rules_applied: vec!["taxonomy_keyword_match".to_string()],
                matched_keywords: best_matches,
                candidate_categories: None,
                already_organized: false,
            };
        } else if best_score > 0.0 {
            // Baixa confiança (score > 0 mas < 0.45) -> Baixa_Confianca
            return TaxonomyCategory {
                id: "inbox.low_confidence".to_string(),
                label: "Inbox / Baixa Confiança".to_string(),
                relative_dir: PathBuf::from("Documentos/00_Inbox/Baixa_Confianca"),
                confidence: best_score.clamp(0.0, 1.0),
                risk: "medium".to_string(),
                needs_review: true,
                reason: format!(
                    "Match com score muito baixo ({:.2}) para categoria '{}'",
                    best_score, def.label
                ),
                rules_applied: vec!["taxonomy_low_confidence".to_string()],
                matched_keywords: best_matches,
                candidate_categories: None,
                already_organized: false,
            };
        }
    }

    // Sem categoria correspondente -> Fallback total baseado no tipo de arquivo
    let (fallback_dir, fallback_label) = if is_image {
        ("Imagens/Revisar", "Revisão de Imagens")
    } else if is_video {
        ("Vídeos/Revisar", "Revisão de Vídeos")
    } else if is_audio {
        ("Músicas/Revisar", "Revisão de Músicas")
    } else {
        ("Documentos/00_Inbox/Revisar", "Inbox / Revisão Geral")
    };

    let risk = if ext_lower.is_empty() {
        "high"
    } else {
        "medium"
    };

    TaxonomyCategory {
        id: "fallback.revisar".to_string(),
        label: fallback_label.to_string(),
        relative_dir: PathBuf::from(fallback_dir),
        confidence: 0.20,
        risk: risk.to_string(),
        needs_review: true,
        reason: "Nenhuma palavra-chave correspondente encontrada".to_string(),
        rules_applied: vec!["taxonomy_fallback".to_string()],
        matched_keywords: vec![],
        candidate_categories: None,
        already_organized: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::FileStatus;

    fn dummy_file(name: &str, ext: &str, mime: &str) -> FileMetadata {
        FileMetadata {
            path: format!("/tmp/home/Downloads/{}", name),
            filename: name.to_string(),
            extension: ext.to_string(),
            size_bytes: 1000,
            modified_at: None,
            mime: mime.to_string(),
            is_symlink: false,
            status: FileStatus::Analyzed,
        }
    }

    #[test]
    fn test_classifica_comprovante_pix() {
        let file = dummy_file("comprovante pix banco.pdf", "pdf", "application/pdf");
        let cat = suggest_category(&file);
        assert_eq!(
            cat.relative_dir.to_str().unwrap(),
            "Documentos/Financeiro/Bancos"
        );
        assert!(cat.matched_keywords.contains(&"pix".to_string()));
        assert!(cat.matched_keywords.contains(&"banco".to_string()));
        assert!(cat.confidence >= 0.75);
    }

    #[test]
    fn test_classifica_boleto() {
        let file = dummy_file("boleto vencimento maio.pdf", "pdf", "application/pdf");
        let cat = suggest_category(&file);
        assert_eq!(
            cat.relative_dir.to_str().unwrap(),
            "Documentos/Financeiro/Boletos"
        );
    }

    #[test]
    fn test_classifica_nota_fiscal() {
        let file = dummy_file("nota fiscal compra pc.pdf", "pdf", "application/pdf");
        let cat = suggest_category(&file);
        assert_eq!(
            cat.relative_dir.to_str().unwrap(),
            "Documentos/Financeiro/Notas_Fiscais"
        );
    }

    #[test]
    fn test_classifica_nixos() {
        let file = dummy_file("nixos flake estudo.txt", "txt", "text/plain");
        let cat = suggest_category(&file);
        assert_eq!(
            cat.relative_dir.to_str().unwrap(),
            "Documentos/Estudos/NixOS"
        );
    }

    #[test]
    fn test_classifica_kryonix_brain() {
        let file = dummy_file("kryonix brain planejamento.md", "md", "text/markdown");
        let cat = suggest_category(&file);
        assert_eq!(
            cat.relative_dir.to_str().unwrap(),
            "Documentos/Projetos/Kryonix"
        );
    }

    #[test]
    fn test_fallback_revisar() {
        let file = dummy_file("coisa aleatoria.txt", "txt", "text/plain");
        let cat = suggest_category(&file);
        assert_eq!(
            cat.relative_dir.to_str().unwrap(),
            "Documentos/00_Inbox/Revisar"
        );
    }

    #[test]
    fn test_media_screenshot() {
        let file = dummy_file("screenshot erro.png", "png", "image/png");
        let cat = suggest_category(&file);
        assert_eq!(cat.relative_dir.to_str().unwrap(), "Imagens/Screenshots");
    }

    #[test]
    fn test_empate_conflitos() {
        // "comprovante" (Comprovantes) e "banco" (Bancos). Ambas são extensões permitidas.
        // Se ambos forem carregados do TOML, comprovante tem id admin.comprovantes, banco tem id financeiro.bancos.
        // Ambas dão score 0.40. Empate -> Inbox/Conflitos
        let file = dummy_file("comprovante banco.pdf", "pdf", "application/pdf");
        let cat = suggest_category(&file);
        assert_eq!(
            cat.relative_dir.to_str().unwrap(),
            "Documentos/00_Inbox/Conflitos"
        );
        assert_eq!(cat.id, "inbox.conflicts");
        let candidates = cat.candidate_categories.unwrap();
        assert!(candidates.contains(&"admin.comprovantes".to_string()));
        assert!(candidates.contains(&"financeiro.bancos".to_string()));
    }

    #[test]
    fn test_no_extension_high_risk() {
        let file = dummy_file("documento_importante", "", "application/octet-stream");
        let cat = suggest_category(&file);
        assert_eq!(cat.risk, "high");
        assert!(cat.needs_review);
    }

    #[test]
    fn test_low_confidence_inbox() {
        // Match com 1 keyword (0.50) mas penalidade de imagem em Documentos (-0.15) = 0.35 score (< 0.45)
        let file = dummy_file("comprovante_avulso.png", "png", "image/png");
        let cat = suggest_category(&file);
        assert_eq!(
            cat.relative_dir.to_str().unwrap(),
            "Documentos/00_Inbox/Baixa_Confianca"
        );
        assert_eq!(cat.id, "inbox.low_confidence");
    }

    #[test]
    fn test_load_taxonomy_toml() {
        let config = load_taxonomy_config(None);
        assert_eq!(config.profile, "kryonix-home-taxonomy-v1");
        assert!(config.categories.len() > 10);
    }
}
