use std::fs;
use std::path::Path;

use anyhow::Result;
use chrono::Utc;

use crate::hashing;
use crate::manifest::{audits_dir, Manifest};

pub fn run_apply(manifest: &mut Manifest, dry_run: bool) -> Result<()> {
    println!("Iniciando Apply (dry-run: {})", dry_run);

    let mut executed = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for action in &mut manifest.actions {
        if action.status != "planned" {
            continue;
        }

        let source = Path::new(&action.source_path);
        let target = Path::new(&action.target_path);

        // Valida se origem existe
        if !source.exists() {
            action.status = "failed".to_string();
            action.error_msg = Some("Origem não encontrada".to_string());
            failed += 1;
            println!("❌ ERRO: Origem ausente: {}", action.source_path);
            continue;
        }

        // Valida se destino já existe
        if target.exists() {
            let mut is_exact_duplicate = false;
            if let Ok(source_hash) = hashing::sha256_of(source) {
                if let Ok(target_hash) = hashing::sha256_of(target) {
                    if source_hash == target_hash {
                        is_exact_duplicate = true;
                    }
                }
            }

            if is_exact_duplicate {
                action.status = "skipped".to_string();
                action.error_msg =
                    Some("Destino já existe e hash é idêntico (duplicata exata)".to_string());
                skipped += 1;
                println!(
                    "⏭️ PULO: Destino existente (Duplicata exata): {}",
                    action.target_path
                );
            } else {
                action.status = "blocked".to_string();
                action.error_msg = Some("destination_exists".to_string());
                failed += 1;
                println!(
                    "❌ BLOQUEADO: Destino já existe com conteúdo diferente: {}",
                    action.target_path
                );
            }
            continue;
        }

        // Valida hash (opcional mas importante para integridade)
        if let Some(expected_hash) = &action.old_hash {
            if let Ok(current_hash) = hashing::sha256_of(source) {
                if current_hash != *expected_hash {
                    action.status = "skipped".to_string();
                    action.error_msg = Some("Hash alterado desde o plano".to_string());
                    skipped += 1;
                    println!(
                        "⏭️ PULO: Arquivo alterado (hash mismatch): {}",
                        action.source_path
                    );
                    continue;
                }
            } else {
                action.status = "failed".to_string();
                action.error_msg = Some("Falha ao ler hash atual".to_string());
                failed += 1;
                println!(
                    "❌ ERRO: Não foi possível ler arquivo para hash: {}",
                    action.source_path
                );
                continue;
            }
        }

        // Somente permitir move ou rename
        if action.action_type != "move" && action.action_type != "rename" {
            action.status = "skipped".to_string();
            action.error_msg = Some(format!(
                "Ação não suportada ou proibida: {}",
                action.action_type
            ));
            skipped += 1;
            println!(
                "⏭️ PULO: Ação proibida ({}): {}",
                action.action_type, action.source_path
            );
            continue;
        }

        if dry_run {
            if let Some(parent) = target.parent() {
                if !parent.exists() {
                    println!("📁 DRY-RUN: Criaria diretório: {}", parent.display());
                }
            }
            println!(
                "✅ DRY-RUN: Moveria/Renomearia {} -> {}",
                action.source_path, action.target_path
            );
        } else {
            // Executar de fato
            if let Some(parent) = target.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    action.status = "failed".to_string();
                    action.error_msg = Some(format!("Falha ao criar diretório: {}", e));
                    failed += 1;
                    println!("❌ ERRO: {}", action.error_msg.as_ref().unwrap());
                    continue;
                }
            }

            match fs::rename(source, target) {
                Ok(_) => {
                    action.status = "executed".to_string();
                    executed += 1;
                    println!(
                        "✅ SUCESSO: Moveu {} -> {}",
                        action.source_path, action.target_path
                    );
                }
                Err(e) => {
                    action.status = "failed".to_string();
                    action.error_msg = Some(format!("Falha no rename: {}", e));
                    failed += 1;
                    println!("❌ ERRO: {}", action.error_msg.as_ref().unwrap());
                }
            }
        }
    }

    println!("\n=== Resultado do Apply ===");
    println!("Modo: {}", if dry_run { "Dry-Run" } else { "Confirmado" });
    if dry_run {
        println!("Passariam: {}", manifest.actions.len() - skipped - failed);
    } else {
        println!("Executados com sucesso: {}", executed);
    }
    println!("Pulados: {}", skipped);
    println!("Falhas: {}", failed);

    if !dry_run {
        save_audit_log(manifest)?;
    }

    Ok(())
}

fn save_audit_log(manifest: &Manifest) -> Result<()> {
    let filename = format!("audit_{}.json", Utc::now().format("%Y%m%d-%H%M%S"));
    let path = audits_dir()?.join(&filename);

    let json = serde_json::to_string_pretty(manifest)?;
    fs::write(&path, json)?;

    eprintln!("Log de auditoria salvo em: {}", path.display());
    Ok(())
}
