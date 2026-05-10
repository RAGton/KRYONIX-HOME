use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::Path;

use crate::{apply, hashing, manifest, planner, report, rollback, scanner};

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
    },
    /// Mostra o resumo do manifesto mais recente
    Show,
}

#[derive(Subcommand)]
enum Commands {
    /// Escaneia a Home e salva resultado em JSON
    Scan,
    /// Mostra relatório do último scan
    Report,
    /// Lista duplicatas exatas (SHA256 idêntico)
    Duplicates,
    /// Lista raízes de projetos detectadas (Git, Rust, Nix, etc.)
    Projects,
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
    /// Executa planejamento focado apenas em limpar ~/Downloads
    Downloads {
        /// Emitir saída em JSON ao invés de texto
        #[arg(long)]
        json: bool,

        /// Caminho opcional para arquivo de taxonomia TOML
        #[arg(long)]
        taxonomy_config: Option<String>,

        /// Limite de propostas exibidas no relatório
        #[arg(long)]
        limit: Option<usize>,

        /// Exibir explicações detalhadas por proposta no relatório de texto
        #[arg(long, default_value_t = false)]
        why: bool,
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
        Commands::Scan => {
            let scan = scanner::run_scan()?;
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
        Commands::Projects => {
            let scan = scanner::load_latest_scan()?;
            report::print_projects(&scan);
            eprintln!("\nNenhuma alteração foi feita.");
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
        } => {
            let file_meta = crate::metadata::collect(Path::new(&path), false);
            let config = crate::taxonomy::load_taxonomy_config(taxonomy_config.as_deref());
            let cat = crate::taxonomy::suggest_category_config(&file_meta, &config);

            println!("=========================================");
            println!("  Explicação de Classificação de Arquivo ");
            println!("=========================================");
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
                if cat.needs_review {
                    "Requer revisão humana"
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
            ..
        } => {
            let scan = scanner::load_latest_scan()?;
            let options = planner::PlanOptions {
                rename_suggestions,
                taxonomy_suggestions,
                taxonomy_config_path: taxonomy_config.as_deref(),
                include_large_files,
                safe_only,
                review_only,
                projects_only: only_projects,
                limit,
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
        Commands::Downloads {
            json,
            taxonomy_config,
            limit,
            why,
        } => {
            let mut scan = scanner::load_latest_scan()?;
            let home_path = Path::new(&scan.home_dir);
            let downloads_path = home_path.join("Downloads");

            scan.files.retain(|f| {
                let p = Path::new(&f.path);
                p.starts_with(&downloads_path) || f.path.to_lowercase().contains("/downloads/")
            });
            scan.projects.retain(|p| {
                let path = Path::new(&p.root_path);
                path.starts_with(&downloads_path)
                    || p.root_path.to_lowercase().contains("/downloads/")
            });

            let options = planner::PlanOptions {
                rename_suggestions: true,
                taxonomy_suggestions: true,
                taxonomy_config_path: taxonomy_config.as_deref(),
                include_large_files: true,
                safe_only: false,
                review_only: false,
                projects_only: false,
                limit,
            };
            let plan = planner::generate_plan(&scan, &options);

            if json {
                println!("{}", serde_json::to_string_pretty(&plan)?);
            } else {
                report::print_plan(&plan);
                if why {
                    println!("\n=== Detalhamento Explicativo das Propostas (Downloads) ===");
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
                eprintln!("\nNenhuma alteração foi feita. Use 'manifest create' ou 'manifest' para registrar.");
            }
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
