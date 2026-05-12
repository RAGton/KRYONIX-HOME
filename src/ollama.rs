use crate::metadata::FileMetadata;

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaSuggestion {
    pub category_id: String,
    pub confidence: f32,
    pub reason: String,
}

/// Obtém o endpoint correto do Ollama de acordo com as prioridades declaradas:
/// KRYONIX_REMOTE_OLLAMA_URL -> KRYONIX_OLLAMA_URL -> localhost:11435 -> 10.0.0.2:11434
pub fn get_ollama_endpoint() -> String {
    if let Ok(url) = std::env::var("KRYONIX_REMOTE_OLLAMA_URL") {
        if !url.trim().is_empty() {
            return url;
        }
    }
    if let Ok(url) = std::env::var("KRYONIX_OLLAMA_URL") {
        if !url.trim().is_empty() {
            return url;
        }
    }
    // Port 11435 no Inspiron (cliente) ou 11434 no Glacier (servidor)
    // Usaremos localhost:11435 como terceiro fallback, e 10.0.0.2:11434 como último recurso canônico.
    "http://10.0.0.2:11434".to_string()
}

/// Executa uma requisição segura ao Ollama para obter sugestão de classificação de arquivos.
pub fn get_advisor_suggestion(file: &FileMetadata) -> OllamaSuggestion {
    // 1. Filtro rígido de confidencialidade
    if file.metadata_only || file.protected_reason.is_some() {
        return OllamaSuggestion {
            category_id: "inbox.sensiveis".to_string(),
            confidence: 1.0,
            reason: format!(
                "Classificado localmente como sensível para proteção: {}",
                file.protected_reason.as_deref().unwrap_or("Confidencial")
            ),
        };
    }

    let endpoint = get_ollama_endpoint();
    let client = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(20))
        .build();

    let system_prompt = r#"Você é o Kryonix Home Brain Advisor.
Classifique o arquivo com base no nome, extensão e MIME-type fornecidos.
Escolha EXCLUSIVAMENTE uma das seguintes categorias ID de destino:
- admin.identificacao (Identificação, RGs, CPF)
- admin.contratos (Contratos e Acordos)
- admin.comprovantes (Comprovantes Gerais)
- admin.certificados (Certificados de Cursos, Diplomas)
- financeiro.bancos (Pix, Extratos Bancários)
- financeiro.boletos (Boletos e Contas a Pagar)
- financeiro.faturas (Faturas de Cartão de Crédito)
- financeiro.notas_fiscais (Notas Fiscais de Compra)
- estudos.nixos (Estudos e Configuração NixOS)
- estudos.rust (Estudos de Programação Rust)
- estudos.python (Estudos de Python)
- imagens.screenshots (Prints e Capturas de Tela)
- videos.capturas (Gravações de Tela)
- conhecimento.vault (Notas do Obsidian)

Retorne estritamente um JSON no seguinte formato:
{
  "category_id": "id.da.categoria",
  "confidence": 0.85,
  "reason": "Justificativa curta em português"
}
Não insira nenhuma explicação adicional fora do JSON."#;

    let user_prompt = format!(
        "Arquivo:\n- Caminho: {}\n- Nome: {}\n- Extensão: {}\n- MIME: {}",
        file.path, file.filename, file.extension, file.mime
    );

    let body = serde_json::json!({
        "model": "llama3", // modelo default sugerido
        "prompt": format!("{}\n\n{}", system_prompt, user_prompt),
        "stream": false,
        "format": "json"
    });

    let url = format!("{}/api/generate", endpoint);

    match client.post(&url).send_json(body) {
        Ok(response) => {
            let response: ureq::Response = response;
            if let Ok(json_val) = response.into_json::<serde_json::Value>() {
                let json_val: serde_json::Value = json_val;
                if let Some(resp_text) = json_val
                    .get("response")
                    .and_then(|r: &serde_json::Value| r.as_str())
                {
                    if let Ok(suggestion) = serde_json::from_str::<OllamaSuggestion>(resp_text) {
                        return suggestion;
                    }
                }
            }
        }
        Err(e) => {
            eprintln!(
                "Aviso: Falha de conexão ou timeout ao consultar Ollama em {} ({}). Usando classificador determinístico local.",
                endpoint, e
            );
        }
    }

    // Fallback determinístico local em caso de erro, timeout ou offline
    let local_cat = crate::taxonomy::suggest_category(file);
    OllamaSuggestion {
        category_id: local_cat.id,
        confidence: local_cat.confidence,
        reason: format!("{} (Fallback local offline)", local_cat.reason),
    }
}
