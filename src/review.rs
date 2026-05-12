use crate::manifest::Manifest;
use anyhow::Result;
use std::fs;
use std::io::{self, Write};

pub struct ReviewOptions {
    pub category: Option<String>,
    pub max_risk: Option<String>,
    pub min_confidence: u8,
}

pub fn run_review(manifest: &mut Manifest, options: ReviewOptions) -> Result<()> {
    println!("\x1b[1m🤝 Kryonix Home Review — Fila de Aprovação Assistida\x1b[0m");
    println!("────────────────────────────────────────────────────────────");

    let mut safe = 0;
    let mut review_req = 0;
    let mut high_risk = 0;
    for action in &manifest.actions {
        if action.status == "planned" && !action.already_organized {
            if action.risk == "low" {
                safe += 1;
            } else if action.risk == "medium" {
                review_req += 1;
            } else {
                high_risk += 1;
            }
        }
    }
    println!("Fila Atual de Decisões:");
    println!("  🟢 Ações Seguras (Risco Baixo):     {}", safe);
    println!("  🟡 Precisam de Revisão (Risco Méd):  {}", review_req);
    println!("  🔴 Alto Risco (Risco Alto):         {}", high_risk);
    println!("────────────────────────────────────────────────────────────");

    let mut approved_count = 0;
    let mut skipped_count = 0;

    let actions_len = manifest.actions.len();

    for i in 0..actions_len {
        let action = &manifest.actions[i];

        // Filtros
        if let Some(ref cat) = options.category {
            let cat_label = action
                .category_label
                .as_deref()
                .unwrap_or("")
                .to_lowercase();
            let cat_id = action.category_id.as_deref().unwrap_or("").to_lowercase();
            if !cat_label.contains(&cat.to_lowercase()) && !cat_id.contains(&cat.to_lowercase()) {
                continue;
            }
        }

        if let Some(ref risk) = options.max_risk {
            let risk_level = match action.risk.as_str() {
                "low" => 0,
                "medium" => 1,
                "high" => 2,
                _ => 3,
            };
            let target_level = match risk.to_lowercase().as_str() {
                "low" => 0,
                "medium" => 1,
                "high" => 2,
                _ => 3,
            };
            if risk_level > target_level {
                continue;
            }
        }

        let confidence = (action.taxonomy_score.unwrap_or(0.0) * 100.0) as u8;
        if confidence < options.min_confidence {
            continue;
        }

        // Se já foi processado ou está organizado, pulamos
        if action.status != "planned" || action.already_organized {
            continue;
        }

        println!("\n[{}/{}] Proposta:", i + 1, actions_len);
        println!("  DE:      \x1b[34m{}\x1b[0m", action.source_path);
        println!("  PARA:    \x1b[32m{}\x1b[0m", action.target_path);
        println!("  MOTIVO:  {}", action.reason);
        println!(
            "  DETALHE: Categoria: {} | Confiança: {}% | Risco: {}",
            action.category_label.as_deref().unwrap_or("Incerto"),
            confidence,
            action.risk.to_uppercase()
        );

        loop {
            print!("\nAção [a]provar, [s]pular, [e]xplicar, [q]uair: ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let cmd = input.trim().to_lowercase();

            match cmd.as_str() {
                "a" | "approve" => {
                    manifest.actions[i].status = "planned".to_string(); // Já está planejado, mas aqui confirmamos a intenção
                    println!("✅ Aprovado.");
                    approved_count += 1;
                    break;
                }
                "s" | "skip" => {
                    manifest.actions[i].status = "skipped".to_string();
                    println!("⏭️ Pulado.");
                    skipped_count += 1;
                    break;
                }
                "e" | "explain" => {
                    println!("\n--- Explicação Detalhada ---");
                    println!("MIME: {}", action.mime);
                    println!("Tamanho: {} bytes", action.size_bytes);
                    if let Some(ref keywords) = action.matched_keywords {
                        println!("Keywords: {}", keywords.join(", "));
                    }
                    if let Some(ref reason) = action.taxonomy_reason {
                        println!("Lógica: {}", reason);
                    }
                    println!("----------------------------");
                }
                "q" | "quit" => {
                    println!("Saindo da revisão.");
                    save_manifest_state(manifest)?;
                    return Ok(());
                }
                _ => println!("Comando inválido."),
            }
        }
    }

    println!("\n────────────────────────────────────────────────────────────");
    println!(
        "Revisão concluída: {} aprovados, {} pulados.",
        approved_count, skipped_count
    );

    save_manifest_state(manifest)?;

    if approved_count > 0 {
        println!("\nPróximo passo: \x1b[1mkryonix home apply --confirm\x1b[0m");
    }

    Ok(())
}

fn save_manifest_state(manifest: &Manifest) -> Result<()> {
    let json = serde_json::to_string_pretty(manifest)?;
    // Encontrar o arquivo original do manifesto para sobrescrever
    let dir = crate::manifest::manifests_dir()?;
    let mut entries = fs::read_dir(&dir)?
        .filter_map(Result::ok)
        .filter(|e: &fs::DirEntry| {
            e.path().is_file() && e.path().extension().is_some_and(|ext| ext == "json")
        })
        .collect::<Vec<_>>();
    entries.sort_by_key(|e: &fs::DirEntry| e.path());

    if let Some(latest) = entries.last() {
        std::fs::write(latest.path(), json)?;
        // Também atualizar o Markdown
        crate::manifest::save_markdown_report(manifest)?;
    }

    Ok(())
}
