use std::collections::HashMap;

use crate::hashing::DuplicateGroup;
use crate::metadata::FileStatus;
use crate::planner::Plan;
use crate::scanner::ScanResult;

/// Formata tamanho em bytes para formato legível.
fn format_size(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * KIB;
    const GIB: u64 = 1024 * MIB;
    const TIB: u64 = 1024 * GIB;

    if bytes >= TIB {
        format!("{:.1} TiB", bytes as f64 / TIB as f64)
    } else if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Imprime resumo rápido do scan.
pub fn print_scan_summary(scan: &ScanResult) {
    println!("\x1b[1mKryonix Home Scan\x1b[0m");
    println!("──────────────────────────────────────────────────────────");
    println!("  Run ID:             {}", scan.run_id);
    println!("  Root:               {}", scan.home_dir);
    println!("  Diretórios:         {}", scan.dirs_scanned.join(", "));
    println!("  Arquivos analisados: {}", scan.files_analyzed);
    println!("  Projetos detectados: {}", scan.projects.len());
    println!("  Arquivos ignorados:  {}", scan.files_ignored);
    println!("  Erros:              {}", scan.files_error);
    println!(
        "  Tamanho total:      {}",
        format_size(scan.total_size_bytes)
    );
}

/// Imprime relatório completo.
pub fn print_full_report(scan: &ScanResult) {
    print_scan_summary(scan);

    // Extensões mais comuns
    let mut ext_counts: HashMap<String, usize> = HashMap::new();
    for file in &scan.files {
        if file.status == FileStatus::Analyzed {
            let ext = if file.extension.is_empty() {
                "(sem extensão)".to_string()
            } else {
                file.extension.clone()
            };
            *ext_counts.entry(ext).or_default() += 1;
        }
    }

    let mut ext_sorted: Vec<_> = ext_counts.into_iter().collect();
    ext_sorted.sort_by_key(|b| std::cmp::Reverse(b.1));

    println!();
    println!("Tipos de arquivo (top 15):");
    for (ext, count) in ext_sorted.iter().take(15) {
        println!("  {ext:>15}: {count}");
    }

    // Maiores arquivos
    let mut analyzed: Vec<_> = scan
        .files
        .iter()
        .filter(|f| f.status == FileStatus::Analyzed)
        .collect();
    analyzed.sort_by_key(|b| std::cmp::Reverse(b.size_bytes));

    println!();
    println!("Maiores arquivos (top 10):");
    for file in analyzed.iter().take(10) {
        println!("  {} — {}", format_size(file.size_bytes), file.path);
    }

    // Tamanho por MIME
    let mut mime_sizes: HashMap<String, u64> = HashMap::new();
    for file in &scan.files {
        if file.status == FileStatus::Analyzed {
            *mime_sizes.entry(mime_category(&file.mime)).or_default() += file.size_bytes;
        }
    }
    let mut mime_sorted: Vec<_> = mime_sizes.into_iter().collect();
    mime_sorted.sort_by_key(|b| std::cmp::Reverse(b.1));

    println!();
    println!("Tamanho por categoria:");
    for (cat, size) in &mime_sorted {
        println!("  {cat:>15}: {}", format_size(*size));
    }
}

/// Categoria MIME simplificada.
fn mime_category(mime: &str) -> String {
    if mime.starts_with("image/") {
        "Imagens".to_string()
    } else if mime.starts_with("video/") {
        "Vídeos".to_string()
    } else if mime.starts_with("audio/") {
        "Áudio".to_string()
    } else if mime.starts_with("text/") {
        "Texto".to_string()
    } else if mime == "application/pdf" {
        "PDF".to_string()
    } else if mime.contains("zip")
        || mime.contains("tar")
        || mime.contains("compressed")
        || mime.contains("gzip")
    {
        "Compactados".to_string()
    } else {
        "Outros".to_string()
    }
}

/// Imprime lista de grupos de duplicatas.
pub fn print_duplicates(groups: &[DuplicateGroup]) {
    if groups.is_empty() {
        println!("Nenhuma duplicata exata encontrada.");
        return;
    }

    println!("Duplicatas exatas (SHA256 idêntico):");
    println!();
    println!("{} grupo(s) encontrado(s):", groups.len());
    println!();

    for (i, group) in groups.iter().enumerate() {
        println!(
            "  Grupo {} — {} ({} arquivos):",
            i + 1,
            format_size(group.size_bytes),
            group.files.len()
        );
        println!("  SHA256: {}", group.hash);
        for file in &group.files {
            println!("    • {file}");
        }
        println!();
    }

    let total_waste: u64 = groups
        .iter()
        .map(|g| g.size_bytes * (g.files.len() as u64 - 1))
        .sum();
    println!(
        "Espaço desperdiçado por duplicatas: {}",
        format_size(total_waste)
    );
}

/// Imprime lista de projetos detectados.
pub fn print_projects(scan: &ScanResult) {
    if scan.projects.is_empty() {
        println!("Nenhum projeto detectado.");
        return;
    }

    println!("\x1b[1mProjetos Detectados ({})\x1b[0m", scan.projects.len());
    println!("──────────────────────────────────────────────────────────");

    for p in &scan.projects {
        let review = if p.needs_review { " [\x1b[33mREVISAR\x1b[0m]" } else { "" };
        println!("▶ \x1b[1m{}\x1b[0m", p.name);
        println!("  Caminho:    {}", p.root_path);
        println!("  Categoria:  {}", p.category_id);
        println!("  Marcadores: {}", p.markers.join(", "));
        println!("  Tamanho:    {} ({} arquivos)", format_size(p.total_size_bytes), p.file_count);
        println!("  Risco:      {} | Motivo: {}{review}", p.risk, p.reason);
        println!();
    }
}

/// Imprime dashboard do plano.
pub fn print_plan_dashboard(plan: &Plan) {
    let mut safe_count = 0;
    let mut review_count = 0;
    let mut conflict_count = 0;
    let mut project_moves = 0;
    let mut file_moves = 0;
    let mut renames = 0;

    for p in &plan.proposals {
        if p.needs_review {
            review_count += 1;
        } else if p.risk == "low" {
            safe_count += 1;
        } else {
            conflict_count += 1;
        }

        if p.action == "move_project" {
            project_moves += 1;
        } else if p.action == "move" {
            file_moves += 1;
        } else if p.action == "rename" {
            renames += 1;
        }
    }

    println!("\x1b[1m📋 Dashboard de Organização (Dry-Run)\x1b[0m");
    println!("──────────────────────────────────────────────────────────");
    println!("  Run ID:           {}", plan.run_id);
    println!("  Arquivos vistos:  {}", plan.files_seen);
    println!("  Projetos vistos:  {}", plan.projects_seen);
    println!("──────────────────────────────────────────────────────────");
    println!("  \x1b[32m✅ Ações Seguras:\x1b[0m      {}", safe_count);
    println!("  \x1b[33m⚠️ Precisam de Revisão:\x1b[0m {}", review_count);
    println!("  \x1b[31m❌ Conflitos/Risco:\x1b[0m    {}", conflict_count);
    println!("──────────────────────────────────────────────────────────");
    println!("  Projetos a mover:   {}", project_moves);
    println!("  Arquivos a mover:   {}", file_moves);
    println!("  Arquivos a renomear: {}", renames);
    println!("──────────────────────────────────────────────────────────");
    println!("  \x1b[1mTotal de Propostas:\x1b[0m  {}", plan.proposals.len());
    println!();
}

/// Imprime o plano em formato legível.
pub fn print_plan(plan: &Plan) {
    print_plan_dashboard(plan);

    if plan.proposals.is_empty() {
        println!("Nenhuma proposta de organização.");
        return;
    }

    println!("\x1b[1mTop 10 Propostas:\x1b[0m");
    
    for p in plan.proposals.iter().take(10) {
        let review = if p.needs_review { " [\x1b[33mREVISAR\x1b[0m]" } else { "" };
        let icon = match p.action.as_str() {
            "move_project" => "📦",
            "rename" => "📝",
            _ => "📄",
        };

        let risk_color = match p.risk.as_str() {
            "low" => "\x1b[32m",
            "medium" => "\x1b[33m",
            "high" => "\x1b[31m",
            _ => "",
        };

        println!(
            "  {} {risk_color}[{:>6}]\x1b[0m {} -> {}",
            icon, p.risk.to_uppercase(), p.old_path, p.new_dir
        );
        if let Some(ref nf) = p.new_filename {
            println!("     └─ Novo Nome: {}", nf);
        }
        println!("     └─ Motivo: {}{review}", p.reason);
    }

    if plan.proposals.len() > 10 {
        println!();
        println!("  ... e mais {} propostas.", plan.proposals.len() - 10);
        println!("  Use \x1b[1mkryonix home plan --why\x1b[0m para ver todos os detalhes.");
    }
}
