use anyhow::{Context, Result};
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};

pub fn state_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Não foi possível determinar o diretório home")?;
    let dir = home.join(".local/state/kryonix/home-brain");
    Ok(dir)
}

/// Executa o diagnóstico completo do diretório de estado
pub fn run_doctor() -> Result<()> {
    let dir = state_dir()?;
    println!("=== Kryonix Home Brain - State Doctor ===");
    println!("Diretório de estado: {}", dir.display());

    if !dir.exists() {
        println!("⚠️ Diretório de estado não existe. Criando...");
        fs::create_dir_all(&dir)?;
    } else {
        println!("✅ Diretório de estado existe e está acessível.");
    }

    let subdirs = ["runs", "manifests", "audit", "memory", "backups"];
    for sub in &subdirs {
        let path = dir.join(sub);
        if path.exists() {
            let files_count = fs::read_dir(&path)?
                .filter_map(Result::ok)
                .filter(|e| e.path().is_file())
                .count();
            println!("✅ Subdiretório '{sub}': existente, contendo {files_count} arquivos.");
        } else {
            println!("ℹ️ Subdiretório '{sub}': não criado ainda.");
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if let Ok(meta) = fs::metadata(&dir) {
            let mode = meta.mode() & 0o777;
            println!("✅ Permissões da pasta de estado: {:o}", mode);
        }
    }

    println!("✅ Diagnóstico de estado concluído.");
    Ok(())
}

/// Executa a limpeza segura de versões incompatíveis de esquemas
pub fn run_clean(old_schema: bool) -> Result<()> {
    if !old_schema {
        println!("Nenhuma opção especificada para limpeza. Use --old-schema.");
        return Ok(());
    }

    let dir = state_dir()?;
    println!("Iniciando limpeza de esquemas antigos...");

    // Limpar manifestos incompatíveis na pasta 'manifests'
    let manifests_dir = dir.join("manifests");
    if manifests_dir.exists() {
        let entries = fs::read_dir(&manifests_dir)?
            .filter_map(Result::ok)
            .filter(|e| e.path().is_file() && e.path().extension().is_some_and(|ext| ext == "json"))
            .collect::<Vec<_>>();

        let mut cleaned_count = 0;
        for entry in entries {
            let content = fs::read_to_string(entry.path())?;
            if serde_json::from_str::<crate::manifest::Manifest>(&content).is_err() {
                let _ = fs::remove_file(entry.path());
                cleaned_count += 1;
            }
        }
        println!("✅ Limpos {cleaned_count} manifestos antigos/incompatíveis.");
    }

    // Limpar runs incompatíveis na pasta 'runs'
    let runs_dir = dir.join("runs");
    if runs_dir.exists() {
        let entries = fs::read_dir(&runs_dir)?
            .filter_map(Result::ok)
            .filter(|e| e.path().is_dir())
            .collect::<Vec<_>>();

        let mut cleaned_runs = 0;
        for entry in entries {
            let scan_file = entry.path().join("scan_result.json");
            if scan_file.exists() {
                let content = fs::read_to_string(&scan_file)?;
                if serde_json::from_str::<crate::scanner::ScanResult>(&content).is_err() {
                    let _ = fs::remove_dir_all(entry.path());
                    cleaned_runs += 1;
                }
            } else {
                let _ = fs::remove_dir_all(entry.path());
                cleaned_runs += 1;
            }
        }
        println!("✅ Lentas {cleaned_runs} pastas de runs antigas/incompatíveis.");
    }

    Ok(())
}

/// Executa o reset seguro do cache temporário de escaneamento
pub fn run_reset(only_cache: bool) -> Result<()> {
    if !only_cache {
        println!("Apenas --only-cache é suportado para reset seguro.");
        return Ok(());
    }

    let dir = state_dir()?;
    let runs_dir = dir.join("runs");
    if runs_dir.exists() {
        println!(
            "⚠️ Removendo todo o cache de escaneamentos em {}...",
            runs_dir.display()
        );
        let _ = fs::remove_dir_all(&runs_dir);
        let _ = fs::create_dir_all(&runs_dir);
        println!("✅ Cache de runs limpo com sucesso.");
    } else {
        println!("✅ Nenhum cache de runs encontrado.");
    }

    Ok(())
}
