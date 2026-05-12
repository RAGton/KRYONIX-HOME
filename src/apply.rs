use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::Utc;

use crate::hashing;
use crate::manifest::{audits_dir, Manifest};

/// Limpa recursivamente pastas vazias a partir do caminho de origem de um arquivo movido,
/// parando ao encontrar o diretório home ou uma pasta não vazia.
fn cleanup_empty_parents(source_path: &str) {
    let mut current = Path::new(source_path).parent();
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home/rocha"));

    while let Some(path) = current {
        // Impede a deleção da raiz da home ou caminhos fora dela
        if path == home || !path.starts_with(&home) {
            break;
        }

        // Verifica se a pasta está vazia
        if let Ok(mut entries) = fs::read_dir(path) {
            if entries.next().is_none() {
                println!("🧹 Limpando pasta que ficou vazia: {}", path.display());
                if let Err(e) = fs::remove_dir(path) {
                    eprintln!(
                        "Aviso: Falha ao remover pasta vazia {}: {}",
                        path.display(),
                        e
                    );
                    break;
                }
            } else {
                // Pasta não está vazia, podemos parar o fluxo de subida
                break;
            }
        } else {
            break;
        }

        current = path.parent();
    }
}

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

        // DEFESA EM PROFUNDIDADE: Bloquear qualquer ação em paths protegidos
        if let Some(reason) = crate::metadata::is_protected_path(source) {
            action.status = "blocked".to_string();
            action.error_msg = Some(format!(
                "Caminho protegido ({}): não pode ser movido",
                reason
            ));
            failed += 1;
            println!(
                "❌ BLOQUEADO (Proteção): {} -> {}",
                action.source_path, reason
            );
            continue;
        }

        // DEFESA EM PROFUNDIDADE: Bloquear qualquer ação não permitida para aplicação automática
        // Requisitos incondicionais:
        // - decision_class deve ser "AutoMoveCertified"
        // - auto_apply_allowed deve ser true
        // - blocked_from_apply deve ser false
        // - risk deve ser "low"
        if action.blocked_from_apply
            || !action.auto_apply_allowed
            || action.decision_class != "AutoMoveCertified"
            || action.risk != "low"
        {
            action.status = "blocked_autopilot".to_string();
            action.error_msg =
                Some("Bloqueado pelas políticas estritas de segurança do autopilot (confidence < 0.95 ou risco != low)".to_string());
            failed += 1;
            println!(
                "❌ BLOQUEADO (Autopilot Policy): {} (Classe: {}, Risco: {})",
                action.source_path, action.decision_class, action.risk
            );
            continue;
        }

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
        if action.action_type != "move_project" {
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
        }

        // Somente permitir move, rename ou move_project
        if action.action_type != "move"
            && action.action_type != "rename"
            && action.action_type != "move_project"
        {
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
            let label = if action.action_type == "move_project" {
                "Projeto"
            } else {
                "Arquivo"
            };
            println!(
                "✅ DRY-RUN: Moveria {} {} -> {}",
                label, action.source_path, action.target_path
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
                    let label = if action.action_type == "move_project" {
                        "Projeto"
                    } else {
                        "Arquivo"
                    };
                    println!(
                        "✅ SUCESSO: Moveu {} {} -> {}",
                        label, action.source_path, action.target_path
                    );

                    // Limpar pastas vazias recursivamente
                    cleanup_empty_parents(&action.source_path);
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
