use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::Utc;

use crate::manifest::{audits_dir, Manifest};

pub fn run_rollback() -> Result<()> {
    println!("Iniciando Rollback...");

    let dir = audits_dir()?;
    let mut entries = fs::read_dir(&dir)?
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file() && e.path().extension().is_some_and(|ext| ext == "json"))
        .collect::<Vec<_>>();

    entries.sort_by_key(|e| e.path());

    let latest = entries
        .last()
        .context("Nenhum log de auditoria encontrado. Não há o que reverter.")?;
    let content = fs::read_to_string(latest.path())?;
    let mut manifest: Manifest = serde_json::from_str(&content)?;

    println!("Revertendo auditoria: {}", latest.path().display());

    let mut reverted = 0;
    let mut failed = 0;

    // Iteramos em reverso para desfazer na ordem inversa
    for action in manifest.actions.iter_mut().rev() {
        if action.status != "executed" {
            continue; // Só reverte o que foi de fato executado
        }

        let original_source = Path::new(&action.source_path); // Para onde vamos mover de volta
        let current_target = Path::new(&action.target_path); // Onde o arquivo está agora

        // Valida se o arquivo ainda está no destino
        if !current_target.exists() {
            action.error_msg = Some("Arquivo ausente no destino. Impossível reverter.".to_string());
            failed += 1;
            println!("❌ ERRO: {}", action.error_msg.as_ref().unwrap());
            continue;
        }

        // Valida se o local original não foi ocupado por outro arquivo
        if original_source.exists() {
            action.error_msg =
                Some("Caminho original ocupado. Abortando reversão deste arquivo.".to_string());
            failed += 1;
            println!("❌ ERRO: {}", action.error_msg.as_ref().unwrap());
            continue;
        }

        // Reverte a ação
        if let Some(parent) = original_source.parent() {
            let _ = fs::create_dir_all(parent); // Tenta criar se tiver apagado o diretório (menos provável)
        }

        match fs::rename(current_target, original_source) {
            Ok(_) => {
                action.status = "reverted".to_string();
                reverted += 1;
                println!(
                    "✅ REVERTIDO: {} <- {}",
                    action.source_path, action.target_path
                );
            }
            Err(e) => {
                action.error_msg = Some(format!("Falha no rename de rollback: {}", e));
                failed += 1;
                println!("❌ ERRO: {}", action.error_msg.as_ref().unwrap());
            }
        }
    }

    println!("\n=== Resultado do Rollback ===");
    println!("Revertidos com sucesso: {}", reverted);
    println!("Falhas: {}", failed);

    // Salva estado atualizado de rollback
    let filename = format!("rollback_{}.json", Utc::now().format("%Y%m%d-%H%M%S"));
    let path = dir.join(&filename);
    let json = serde_json::to_string_pretty(&manifest)?;
    fs::write(&path, json)?;

    eprintln!("Log de rollback salvo em: {}", path.display());

    Ok(())
}
