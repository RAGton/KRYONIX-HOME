use anyhow::Result;
use clap::{Parser, Subcommand};

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
    /// Cria um manifesto a partir do último plano gerado
    Create,
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
    /// Gera plano de organização (dry-run por padrão)
    Plan {
        /// Emitir saída em JSON ao invés de texto
        #[arg(long)]
        json: bool,

        /// Modo dry-run (padrão; existe para documentação)
        #[arg(long, default_value_t = true)]
        dry_run: bool,
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
    },
    /// Reverte o último apply executado
    Rollback,
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
        Commands::Plan { json, .. } => {
            let scan = scanner::load_latest_scan()?;
            let plan = planner::generate_plan(&scan);
            if json {
                println!("{}", serde_json::to_string_pretty(&plan)?);
            } else {
                report::print_plan(&plan);
                eprintln!("\nNenhuma alteração foi feita. Modo: dry-run.");
            }
        }
        Commands::Manifest { command } => match command {
            ManifestCommands::Create => {
                let scan = scanner::load_latest_scan()?;
                let plan = planner::generate_plan(&scan);
                manifest::create_manifest(&plan, &scan)?;
            }
            ManifestCommands::Show => {
                let m = manifest::get_latest_manifest()?;
                manifest::show_manifest(&m);
            }
        },
        Commands::Apply { dry_run, confirm } => {
            if !dry_run && !confirm {
                eprintln!("Por segurança, você deve passar --dry-run ou --confirm para apply.");
                std::process::exit(1);
            }

            // if both are passed, we prioritize dry_run as safety
            let actual_dry_run = dry_run || !confirm;
            let mut m = manifest::get_latest_manifest()?;
            apply::run_apply(&mut m, actual_dry_run)?;
        }
        Commands::Rollback => {
            rollback::run_rollback()?;
        }
    }

    Ok(())
}
