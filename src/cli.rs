use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::Path;

use crate::{apply, hashing, manifest, planner, report, review, rollback, scanner};

/// Kryonix Home Brain — scanner determinístico e organizador seguro da Home
#[derive(Parser)]
#[command(name = "kryonix-home", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum ManifestCommands {
    /// Cria um manifesto a partir do último scan
    Create {
        /// Incluir sugestões de renomeação ABNT-like
        #[arg(long, default_value_t = false)]
        rename_suggestions: bool,

        /// Incluir sugestões de taxonomia (classificação de pastas)
        #[arg(long, default_value_t = false)]
        taxonomy_suggestions: bool,

        /// Caminho opcional para arquivo de taxonomia TOML
        #[arg(long)]
        taxonomy_config: Option<String>,

        /// Permitir análise e hash de arquivos maiores de 2 GiB
        #[arg(long, default_value_t = false)]
        include_large_files: bool,

        /// Filtrar apenas por propostas 100% seguras (sem mídia, risco baixo, sem conflitos)
        #[arg(long, default_value_t = false)]
        safe_only: bool,

        /// Filtrar apenas por propostas que precisam de revisão humana
        #[arg(long, default_value_t = false)]
        review_only: bool,

        /// Utiliza o Ollama Advisor para classificação de taxonomia
        #[arg(long, default_value_t = false)]
        ollama: bool,
    },
    /// Mostra o resumo do manifesto mais recente
    Show,
}

#[derive(Subcommand)]
enum Commands {
    /// Escaneia a Home e salva resultado em JSON
    Scan {
        /// Realiza um scan completo de toda a Home, com proteção contra vazamento
        #[arg(long, default_value_t = false)]
        full_home: bool,

        /// Coleta apenas metadados básicos, sem tentar ler conteúdo para hash ou análise
        #[arg(long, default_value_t = false)]
        metadata_only: bool,

        /// Lê apenas conteúdo de arquivos considerados seguros e pequenos
        #[arg(long, default_value_t = false)]
        safe_content: bool,
    },
    /// Mostra relatório do último scan
    Report,
    /// Lista duplicatas exatas (SHA256 idêntico)
    Duplicates,
    /// Lista raízes de projetos detectadas (Git, Rust, Nix, etc.)
    Projects {
        /// Emitir saída em formato JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Lista todas as categorias de taxonomia atualmente configuradas
    Categories {
        /// Emitir saída em formato JSON
        #[arg(long, default_value_t = false)]
        json: bool,

        /// Caminho opcional para arquivo de taxonomia TOML
        #[arg(long)]
        taxonomy_config: Option<String>,
    },
    /// Explica por que um arquivo específico se encaixa em uma categoria e regras aplicadas
    Explain {
        /// Caminho para o arquivo que será analisado
        path: String,

        /// Caminho opcional para arquivo de taxonomia TOML
        #[arg(long)]
        taxonomy_config: Option<String>,

        /// Habilita inspeção de conteúdo para classificação mais precisa
        #[arg(long)]
        content_aware: bool,

        /// Saída em formato JSON
        #[arg(long)]
        json: bool,
    },
    /// Diagnostica um arquivo de forma detalhada e estilizada, ou em formato JSON
    Diagnose {
        /// Caminho do arquivo a ser diagnosticado
        path: String,

        /// Força a saída no formato puramente JSON
        #[arg(long, default_value_t = false)]
        json: bool,

        /// Realiza análise sensível ao conteúdo durante o diagnóstico
        #[arg(long, default_value_t = false)]
        content_aware: bool,

        /// Utiliza o Ollama Advisor para o diagnóstico
        #[arg(long, default_value_t = false)]
        ollama: bool,
    },
    /// Gera plano de organização (dry-run por padrão)
    Plan {
        /// Emitir saída em JSON ao invés de texto
        #[arg(long)]
        json: bool,

        /// Modo dry-run (padrão; existe para documentação)
        #[arg(long, default_value_t = true)]
        dry_run: bool,

        /// Incluir sugestões de renomeação ABNT-like
        #[arg(long, default_value_t = false)]
        rename_suggestions: bool,

        /// Incluir sugestões de taxonomia (classificação de pastas)
        #[arg(long, default_value_t = false)]
        taxonomy_suggestions: bool,

        /// Caminho opcional para arquivo de taxonomia TOML
        #[arg(long)]
        taxonomy_config: Option<String>,

        /// Permitir análise e hash de arquivos maiores de 2 GiB
        #[arg(long, default_value_t = false)]
        include_large_files: bool,

        /// Filtrar apenas por propostas 100% seguras (sem mídia, risco baixo, sem conflitos)
        #[arg(long, default_value_t = false)]
        safe_only: bool,

        /// Filtrar apenas por propostas que precisam de revisão humana
        #[arg(long, default_value_t = false)]
        review_only: bool,

        /// Exibir apenas propostas de projetos
        #[arg(long, default_value_t = false)]
        only_projects: bool,

        /// Limite de propostas exibidas no relatório
        #[arg(long)]
        limit: Option<usize>,

        /// Exibir apenas o resumo/dashboard, sem lista de arquivos
        #[arg(long, default_value_t = false)]
        summary: bool,

        /// Exibir explicações detalhadas por proposta no relatório de texto
        #[arg(long, default_value_t = false)]
        why: bool,

        /// Realiza o plano assumindo um scan completo
        #[arg(long, default_value_t = false)]
        full_home: bool,

        /// Utiliza o Ollama Advisor para classificação de taxonomia
        #[arg(long, default_value_t = false)]
        ollama: bool,

        /// Realiza o planejamento considerando o conteúdo dos arquivos
        #[arg(long, default_value_t = false)]
        content_aware: bool,

        /// Realiza o planejamento considerando o contexto do projeto
        #[arg(long, default_value_t = false)]
        context_aware: bool,
    },
    /// Gerencia os manifestos de ações
    Manifest {
        #[command(subcommand)]
        command: ManifestCommands,
    },
    /// Aplica as ações do último manifesto
    Apply {
        /// Apenas simula as ações
        #[arg(long, default_value_t = false)]
        dry_run: bool,

        /// Confirma e executa as ações (move/rename)
        #[arg(long, default_value_t = false)]
        confirm: bool,

        /// Exibir o que será feito antes de pedir confirmação interativa
        #[arg(long, default_value_t = false)]
        interactive_preview: bool,
    },
    /// Reverte o último apply executado
    Rollback,
    /// Exibe um dashboard humano com resumo da Home e propostas
    Dashboard {
        /// Caminho opcional para arquivo de taxonomia TOML
        #[arg(long)]
        taxonomy_config: Option<String>,

        /// Exibe saída em formato JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Foca na organização de Downloads e Área de Trabalho
    Inbox {
        /// Caminho opcional para arquivo de taxonomia TOML
        #[arg(long)]
        taxonomy_config: Option<String>,

        /// Exibe saída em formato JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Inicia a fila de revisão assistida para propostas
    Review {
        /// Filtrar por categoria (ex: downloads, financeiro)
        #[arg(long)]
        category: Option<String>,

        /// Filtrar por nível de risco máximo (low, medium, high)
        #[arg(long, aliases = ["risk"])]
        max_risk: Option<String>,

        /// Confiança mínima (0-100)
        #[arg(long, default_value_t = 0)]
        min_confidence: u8,
    },
    /// Exporta os eventos do Home Brain em formato JSONL
    #[command(name = "export-memory")]
    ExportMemory {
        /// Apenas simula a exportação sem salvar no arquivo local
        #[arg(long, default_value_t = false)]
        dry_run: bool,

        /// Imprime cada linha do evento JSONL na saída padrão (stdout)
        #[arg(long, default_value_t = false)]
        jsonl: bool,

        /// Escolhe a fonte de dados (latest-scan, latest-plan, latest-manifest, latest-audit)
        #[arg(long, default_value = "latest-plan")]
        from: String,
    },
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan {
            full_home,
            metadata_only,
            safe_content,
        } => {
            let scan = scanner::run_scan_options(full_home, metadata_only, safe_content)?;
            scanner::save_scan(&scan)?;
            report::print_scan_summary(&scan);
            eprintln!("\nNenhuma alteração foi feita.");
        }
        Commands::Report => {
            let scan = scanner::load_latest_scan()?;
            report::print_full_report(&scan);
            eprintln!("\nNenhuma alteração foi feita.");
        }
        Commands::Duplicates => {
            let scan = scanner::load_latest_scan()?;
            let groups = hashing::find_duplicates(&scan)?;
            report::print_duplicates(&groups);
            eprintln!("\nNenhuma alteração foi feita.");
        }
        Commands::Projects { json } => {
            let scan = scanner::load_latest_scan()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&scan.projects)?);
            } else {
                report::print_projects(&scan);
                eprintln!("\nNenhuma alteração foi feita.");
            }
        }
        Commands::Categories {
            json,
            taxonomy_config,
        } => {
            let config = crate::taxonomy::load_taxonomy_config(taxonomy_config.as_deref());
            if json {
                println!("{}", serde_json::to_string_pretty(&config)?);
            } else {
                println!("=========================================");
                println!("Perfil de Taxonomia Ativo:  {}", config.profile);
                println!("Diretório de Fallback:      {}", config.fallback_dir);
                println!("=========================================");
                println!("\nCategorias Configuradas:");
                for cat in &config.categories {
                    println!("\n▶ [{}] {}", cat.id, cat.label);
                    println!("  Destino:    {}", cat.dir);
                    println!("  Keywords:   {}", cat.keywords.join(", "));
                    if let Some(ref exts) = cat.extensions {
                        println!("  Extensões:  {}", exts.join(", "));
                    }
                    if let Some(ref risk) = cat.risk {
                        println!("  Risco:      {}", risk);
                    }
                }
            }
        }
        Commands::Explain {
            path,
            taxonomy_config,
            content_aware,
            json,
        } => {
            let path_ref = Path::new(&path);
            let is_protected = crate::metadata::is_protected_path(path_ref);

            if is_protected.is_some() && content_aware {
                if json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "status": "error",
                            "message": "PROTECTED: conteúdo não inspecionado por política de segurança."
                        })
                    );
                } else {
                    println!("PROTECTED: conteúdo não inspecionado por política de segurança.");
                }
                return Ok(());
            }

            let file_meta = crate::metadata::collect(path_ref, content_aware);
            let config = crate::taxonomy::load_taxonomy_config(taxonomy_config.as_deref());
            let cat = crate::taxonomy::suggest_category_config(&file_meta, &config);

            if json {
                println!("{}", serde_json::to_string_pretty(&cat)?);
            } else {
                println!("=========================================");
                println!("  Explicação de Classificação de Arquivo ");
                println!("=========================================");
                if let Some(ref reason) = is_protected {
                    println!("🛡️ PROTEGIDO: {}", reason);
                }
                println!("Caminho:     {}", file_meta.path);
                println!("Nome:        {}", file_meta.filename);
                println!("Extensão:    {}", file_meta.extension);
                println!("MIME:        {}", file_meta.mime);
                println!("Tamanho:     {} bytes", file_meta.size_bytes);
                println!("-----------------------------------------");
                println!("Categoria:   {} ({})", cat.label, cat.id);
                println!("Destino:     {}", cat.relative_dir.display());
                println!("Confiança:   {:.2}", cat.confidence);
                println!("Risco:       {}", cat.risk);
                println!(
                    "Revisão:     {}",
                    if cat.needs_review || is_protected.is_some() || path.contains("Obsidian Vault")
                    {
                        "Requer revisão humana (Mandatório)"
                    } else {
                        "Automatizado"
                    }
                );
                println!("Keywords:    {}", cat.matched_keywords.join(", "));
                println!("Motivo:      {}", cat.reason);
                if let Some(ref candidates) = cat.candidate_categories {
                    println!("Conflitos:   {}", candidates.join(", "));
                }
                println!("=========================================");
            }
        }
        Commands::Diagnose {
            path,
            json,
            content_aware,
            ollama,
        } => {
            let file_meta = crate::metadata::collect(Path::new(&path), content_aware);
            let config = crate::taxonomy::load_taxonomy_config(None);
            let cat = if ollama {
                let sug = crate::ollama::get_advisor_suggestion(&file_meta);
                crate::taxonomy::TaxonomyCategory {
                    id: sug.category_id.clone(),
                    label: sug.category_id.clone(),
                    relative_dir: std::path::PathBuf::from("Revisar"),
                    confidence: sug.confidence,
                    risk: "medium".to_string(),
                    needs_review: true,
                    reason: sug.reason,
                    rules_applied: vec!["ollama_advisor".to_string()],
                    matched_keywords: vec![],
                    candidate_categories: None,
                    already_organized: false,
                }
            } else {
                crate::taxonomy::suggest_category_config(&file_meta, &config)
            };
            let rename_sug = crate::naming::suggest_rename(&file_meta);

            if json {
                let diagnostic_json = serde_json::json!({
                    "path": file_meta.path,
                    "filename": file_meta.filename,
                    "extension": file_meta.extension,
                    "mime": file_meta.mime,
                    "size_bytes": file_meta.size_bytes,
                    "metadata_only": file_meta.metadata_only,
                    "protected_reason": file_meta.protected_reason,
                    "readable": file_meta.readable,
                    "source_zone": file_meta.source_zone,
                    "taxonomy": {
                        "category_id": cat.id,
                        "label": cat.label,
                        "target_dir": cat.relative_dir.to_string_lossy(),
                        "confidence": cat.confidence,
                        "risk": cat.risk,
                        "needs_review": cat.needs_review,
                        "reason": cat.reason,
                        "matched_keywords": cat.matched_keywords,
                    },
                    "naming": {
                        "suggested_filename": rename_sug.as_ref().map(|s| &s.suggested_filename),
                        "reason": rename_sug.as_ref().map(|s| &s.reason),
                    }
                });
                println!("{}", serde_json::to_string_pretty(&diagnostic_json)?);
            } else {
                println!(
                    "╭──────────────────────────────────────────────────────────────────────────╮"
                );
                println!(
                    "│                      🔍 DIAGNÓSTICO DE ARQUIVO                           │"
                );
                println!(
                    "├──────────────────────────────────────────────────────────────────────────┤"
                );
                println!("│ 📁 Caminho:   {}", file_meta.path);
                println!("│ 📄 Nome:      {}", file_meta.filename);
                println!(
                    "│ 🔌 Ext/MIME:  {} | {}",
                    file_meta.extension, file_meta.mime
                );
                println!("│ ⚖️ Tamanho:   {} bytes", file_meta.size_bytes);
                println!(
                    "│ 🔒 Protegido: {}",
                    if file_meta.metadata_only {
                        format!(
                            "Sim (Motivo: {})",
                            file_meta
                                .protected_reason
                                .as_deref()
                                .unwrap_or("Confidencial")
                        )
                    } else {
                        "Não".to_string()
                    }
                );
                println!(
                    "│ 🌐 Origem:    {}",
                    file_meta.source_zone.as_deref().unwrap_or("unknown")
                );
                println!(
                    "├──────────────────────────────────────────────────────────────────────────┤"
                );
                println!(
                    "│                      🏷️ CLASSIFICAÇÃO TAXONÔMICA                         │"
                );
                println!(
                    "├──────────────────────────────────────────────────────────────────────────┤"
                );
                println!("│ Categoria:   {} ({})", cat.label, cat.id);
                println!("│ Destino:     {}", cat.relative_dir.display());
                println!(
                    "│ Confiança:   {:.2} | Risco: {} | Revisão: {}",
                    cat.confidence,
                    cat.risk,
                    if cat.needs_review { "Sim" } else { "Não" }
                );
                println!("│ Motivo:      {}", cat.reason);
                if !cat.matched_keywords.is_empty() {
                    println!("│ Palavras:    {}", cat.matched_keywords.join(", "));
                }
                println!(
                    "├──────────────────────────────────────────────────────────────────────────┤"
                );
                println!(
                    "│                      📝 SUGESTÃO DE NOME (ABNT)                          │"
                );
                println!(
                    "├──────────────────────────────────────────────────────────────────────────┤"
                );
                if let Some(rename) = rename_sug {
                    println!("│ Novo Nome:   {}", rename.suggested_filename);
                    println!("│ Motivo:      {}", rename.reason);
                } else {
                    println!("│ Nome sugerido: Nenhuma alteração recomendada.");
                }
                println!(
                    "╰──────────────────────────────────────────────────────────────────────────╯"
                );
            }
        }
        Commands::Plan {
            json,
            rename_suggestions,
            taxonomy_suggestions,
            taxonomy_config,
            include_large_files,
            safe_only,
            review_only,
            only_projects,
            limit,
            summary,
            why,
            full_home,
            ollama,
            content_aware,
            context_aware,
            dry_run: _,
        } => {
            let scan = if full_home {
                scanner::run_scan_options(true, false, true)?
            } else {
                scanner::load_latest_scan()?
            };
            let options = planner::PlanOptions {
                rename_suggestions,
                taxonomy_suggestions,
                taxonomy_config_path: taxonomy_config.as_deref(),
                include_large_files,
                safe_only,
                review_only,
                projects_only: only_projects,
                limit,
                ollama,
                full_home,
                content_aware,
                context_aware,
            };
            let plan = planner::generate_plan(&scan, &options);
            if json {
                println!("{}", serde_json::to_string_pretty(&plan)?);
            } else {
                if summary {
                    report::print_plan_dashboard(&plan);
                } else {
                    report::print_plan(&plan);
                }
                if why {
                    println!("\n=== Detalhamento Explicativo das Propostas ===");
                    for prop in &plan.proposals {
                        println!("\nArquivo: {}", prop.old_path);
                        println!("  Ação:   {} -> {}", prop.action, prop.new_dir);
                        if let Some(ref nf) = prop.new_filename {
                            println!("  Nome:   {}", nf);
                        }
                        println!("  Motivo: {}", prop.reason);
                        println!(
                            "  Risco:  {} | Confiança: {:.2} | Revisão: {}",
                            prop.risk, prop.confidence, prop.needs_review
                        );
                    }
                }
                eprintln!("\nNenhuma alteração foi feita. Modo: dry-run.");
            }
        }
        Commands::Manifest { command } => match command {
            ManifestCommands::Create {
                rename_suggestions,
                taxonomy_suggestions,
                taxonomy_config,
                include_large_files,
                safe_only,
                review_only,
                ollama,
            } => {
                let scan = scanner::load_latest_scan()?;
                let options = planner::PlanOptions {
                    rename_suggestions,
                    taxonomy_suggestions,
                    taxonomy_config_path: taxonomy_config.as_deref(),
                    include_large_files,
                    safe_only,
                    review_only,
                    projects_only: false,
                    limit: None,
                    ollama,
                    full_home: scan.full_home,
                    content_aware: false, // Por enquanto não exposto no manifest create
                    context_aware: false, // Por enquanto não exposto no manifest create
                };
                let plan = planner::generate_plan(&scan, &options);
                manifest::create_manifest(&plan, &scan)?;
            }
            ManifestCommands::Show => {
                let m = manifest::get_latest_manifest()?;
                manifest::show_manifest(&m);
            }
        },
        Commands::Apply {
            dry_run,
            confirm,
            interactive_preview,
        } => {
            if !dry_run && !confirm {
                eprintln!("Por segurança, você deve passar --dry-run ou --confirm para apply.");
                std::process::exit(1);
            }

            if interactive_preview {
                let m = manifest::get_latest_manifest()?;
                manifest::show_manifest(&m);
                println!("\nVocê deseja prosseguir com o apply acima? [s/N]");
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if input.trim().to_lowercase() != "s" {
                    println!("Cancelado pelo usuário.");
                    return Ok(());
                }
            }

            // if both are passed, we prioritize dry_run as safety
            let actual_dry_run = dry_run || !confirm;
            let mut m = manifest::get_latest_manifest()?;
            apply::run_apply(&mut m, actual_dry_run)?;
        }
        Commands::Rollback => {
            rollback::run_rollback()?;
        }
        Commands::Dashboard {
            taxonomy_config,
            json,
        } => {
            let scan = match scanner::load_latest_scan() {
                Ok(s) => s,
                Err(_) => {
                    let scan = scanner::run_scan_options(false, false, false)?;
                    let _ = scanner::save_scan(&scan);
                    scan
                }
            };
            let options = planner::PlanOptions {
                taxonomy_config_path: taxonomy_config.as_deref(),
                ..Default::default()
            };
            let plan = planner::generate_plan(&scan, &options);
            if json {
                println!("{}", serde_json::to_string_pretty(&plan)?);
            } else {
                report::print_plan_dashboard(&plan);
            }
        }
        Commands::Inbox {
            taxonomy_config,
            json,
        } => {
            let scan = match scanner::load_latest_scan() {
                Ok(s) => s,
                Err(_) => {
                    let scan = scanner::run_scan_options(false, false, false)?;
                    let _ = scanner::save_scan(&scan);
                    scan
                }
            };
            let options = planner::PlanOptions {
                taxonomy_config_path: taxonomy_config.as_deref(),
                taxonomy_suggestions: true,
                ..Default::default()
            };
            let plan = planner::generate_plan(&scan, &options);
            if json {
                println!("{}", serde_json::to_string_pretty(&plan)?);
            } else {
                report::print_inbox_report(&plan);
            }
        }
        Commands::Review {
            category,
            max_risk,
            min_confidence,
        } => {
            let mut m = manifest::get_latest_manifest()?;
            let options = review::ReviewOptions {
                category,
                max_risk,
                min_confidence,
            };
            review::run_review(&mut m, options)?;
        }
        Commands::ExportMemory {
            dry_run,
            jsonl,
            from,
        } => {
            crate::export::export_memory(&from, jsonl, dry_run)?;
        }
    }

    Ok(())
}
