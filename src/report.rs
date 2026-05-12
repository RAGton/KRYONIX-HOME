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

    println!(
        "\x1b[1mProjetos Detectados ({})\x1b[0m",
        scan.projects.len()
    );
    println!("──────────────────────────────────────────────────────────");

    for p in &scan.projects {
        let review = if p.needs_review {
            " [\x1b[33mREVISAR\x1b[0m]"
        } else {
            ""
        };
        println!("▶ \x1b[1m{}\x1b[0m", p.name);
        println!("  Caminho:    {}", p.root_path);
        println!("  Categoria:  {}", p.category_id);
        println!("  Marcadores: {}", p.markers.join(", "));
        println!(
            "  Tamanho:    {} ({} arquivos)",
            format_size(p.total_size_bytes),
            p.file_count
        );
        println!("  Risco:      {} | Motivo: {}{review}", p.risk, p.reason);
        println!();
    }
}

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

    let mut modes = Vec::new();
    if plan.full_home {
        modes.push("full-home");
    }
    if plan.content_aware {
        modes.push("content-aware");
    }
    if plan.context_aware {
        modes.push("context-aware");
    }
    let modes_str = if modes.is_empty() {
        "standard".to_string()
    } else {
        modes.join(" + ")
    };

    println!("\x1b[1m╭────────────────────────────────────────────────────────────╮\x1b[0m");
    println!("\x1b[1m│ 🧊 Kryonix Home Plan Dashboard                             │\x1b[0m");
    println!("\x1b[1m├────────────────────────────────────────────────────────────┤\x1b[0m");
    println!("│ Home: {:<52} │", plan.home_dir);
    println!("│ Modo: {:<52} │", modes_str);
    println!("│ Arquivos vistos: {:<41} │", plan.files_seen);
    println!("│ Projetos vistos: {:<41} │", plan.projects_seen);

    let downloads_count = plan
        .proposals
        .iter()
        .filter(|p| {
            let path_lower = p.old_path.to_lowercase();
            path_lower.contains("/downloads/")
                || path_lower.contains("/desktop/")
                || path_lower.contains("/área de trabalho/")
        })
        .count();
    println!("│ Inbox (Pendentes): {:<38} │", downloads_count);
    println!("│ Revisão necessária: {:<38} │", review_count);
    println!("│ Itens protegidos: {:<40} │", plan.protected_files.len());
    println!("\x1b[1m╰────────────────────────────────────────────────────────────╯\x1b[0m");

    println!("\n\x1b[1mResumo de Ações:\x1b[0m");
    println!("  \x1b[32m✅ Ações Seguras:\x1b[0m      {}", safe_count);
    println!("  \x1b[33m⚠️ Precisam de Revisão:\x1b[0m {}", review_count);
    println!("  \x1b[31m❌ Conflitos/Risco:\x1b[0m    {}", conflict_count);
    println!("  ──────────────────────────");
    println!("  Projetos a mover:   {}", project_moves);
    println!("  Arquivos a mover:   {}", file_moves);
    println!("  Arquivos a renomear: {}", renames);
    println!(
        "  \x1b[33m🛡️ Itens protegidos:\x1b[0m  {} (ignorados/segurança)",
        plan.protected_files.len()
    );
    println!(
        "  \x1b[1mTotal de Propostas:\x1b[0m  {}",
        plan.proposals.len()
    );

    // Calcular Top 5 Categorias
    let mut category_counts = HashMap::new();
    for p in &plan.proposals {
        let cat = p
            .category_label
            .clone()
            .unwrap_or_else(|| "Incerto".to_string());
        *category_counts.entry(cat).or_insert(0) += 1;
    }
    let mut categories_sorted: Vec<(String, usize)> = category_counts.into_iter().collect();
    categories_sorted.sort_by(|a, b| b.1.cmp(&a.1));

    // Calcular Top 5 Diretórios de Destino
    let mut dir_counts = HashMap::new();
    for p in &plan.proposals {
        let dir = p.new_dir.clone();
        *dir_counts.entry(dir).or_insert(0) += 1;
    }
    let mut dirs_sorted: Vec<(String, usize)> = dir_counts.into_iter().collect();
    dirs_sorted.sort_by(|a, b| b.1.cmp(&a.1));

    println!("\n\x1b[1m📂 Top 5 Categorias Sugeridas:\x1b[0m");
    for (cat, count) in categories_sorted.iter().take(5) {
        println!("  - {:<25} ({:>3} propostas)", cat, count);
    }

    println!("\n\x1b[1m📍 Top 5 Diretórios de Destino:\x1b[0m");
    for (dir, count) in dirs_sorted.iter().take(5) {
        let truncated_dir = truncate_path(dir, 35);
        println!("  - {:<35} ({:>3} propostas)", truncated_dir, count);
    }

    println!("\n\x1b[1m💡 Passos Recomendados:\x1b[0m");
    println!("  1. Analisar a caixa de entrada: \x1b[1mkryonix home inbox\x1b[0m");
    println!("  2. Revisar propostas interativamente: \x1b[1mkryonix home review\x1b[0m");
    println!("  3. Executar simulação segura: \x1b[1mkryonix home apply --dry-run\x1b[0m");
    println!("  4. Confirmar e aplicar: \x1b[1mkryonix home apply --confirm\x1b[0m");
    println!();
}

/// Imprime o plano em formato legível com tabela visual.
pub fn print_plan(plan: &Plan) {
    print_plan_dashboard(plan);

    if plan.proposals.is_empty() {
        println!("Nenhuma proposta de organização.");
        return;
    }

    println!("\x1b[1mTabela de Organização (Top 15):\x1b[0m");
    println!("────────────────────────────────────────────────────────────────────────────────────────────────────");
    println!(
        "\x1b[1m  {:<6} | {:<30} -> {:<30} | MOTIVO\x1b[0m",
        "RISCO", "ORIGEM (DE ONDE ESTÁ)", "DESTINO (PARA ONDE VAI)"
    );
    println!("────────────────────────────────────────────────────────────────────────────────────────────────────");

    for p in plan.proposals.iter().take(15) {
        let risk_color = match p.risk.as_str() {
            "low" => "\x1b[32m",    // Verde
            "medium" => "\x1b[33m", // Amarelo
            "high" => "\x1b[31m",   // Vermelho
            _ => "",
        };

        let old_path = truncate_path(&p.old_path, 30);
        let new_dir = truncate_path(&p.new_dir, 30);
        let review_flag = if p.needs_review { " ⚠️" } else { "" };

        println!(
            "  {risk_color}{:<6}\x1b[0m | {:<30} -> {:<30} | {}{}",
            p.risk.to_uppercase(),
            old_path,
            new_dir,
            p.reason,
            review_flag
        );

        if let Some(ref nf) = p.new_filename {
            println!("           | \x1b[2mRenomear para: {}\x1b[0m", nf);
        }
    }

    println!("────────────────────────────────────────────────────────────────────────────────────────────────────");

    if plan.proposals.len() > 15 {
        println!();
        println!("  ... e mais {} propostas.", plan.proposals.len() - 15);
        println!("  Use \x1b[1mkryonix home plan --limit 100\x1b[0m para ver mais detalhes.");
    }

    if !plan.protected_files.is_empty() {
        println!();
        println!("\x1b[1m🛡️ Resumo de Itens Protegidos (ignorados por segurança):\x1b[0m");
        println!("────────────────────────────────────────────────────────────────────────────────────────────────────");

        let mut reasons_map: HashMap<String, usize> = HashMap::new();
        for f in &plan.protected_files {
            let reason = f
                .protected_reason
                .as_deref()
                .unwrap_or("Proteção padrão")
                .to_string();
            *reasons_map.entry(reason).or_default() += 1;
        }

        for (reason, count) in reasons_map {
            println!("  • {:<50} | {} itens", reason, count);
        }

        println!("\n  Nota: Nomes individuais de arquivos em paths protegidos (.ssh, .gnupg, etc) foram omitidos.");
        println!("────────────────────────────────────────────────────────────────────────────────────────────────────");
    }
}

/// Trunca o caminho para caber na tabela, preservando o final.
fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        return path.to_string();
    }
    if max_len < 10 {
        return path[..max_len].to_string();
    }
    let half = (max_len - 3) / 2;
    format!("{}...{}", &path[..half], &path[path.len() - half..])
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Imprime relatório focado na "Inbox" (Downloads, Desktop, etc.)
pub fn print_inbox_report(plan: &Plan) {
    println!("\x1b[1m📥 Kryonix Home Inbox (Downloads & Desktop)\x1b[0m");
    println!("────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────");
    println!(
        "\x1b[1m  {:<30} | {:<30} | {:<15} | {:<9} | {:<6} | MOTIVO\x1b[0m",
        "DE ONDE ESTÁ", "PARA ONDE VAI", "CATEGORIA", "CONFIANÇA", "RISCO"
    );
    println!("────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────");

    let mut count = 0;
    let mut temp_files_count = 0;
    let mut temp_files_size = 0;
    let mut low_conf_count = 0;

    for p in &plan.proposals {
        let path_lower = p.old_path.to_lowercase();
        let is_inbox = path_lower.contains("/downloads/")
            || path_lower.contains("/desktop/")
            || path_lower.contains("/área de trabalho/");

        if is_inbox {
            // Verifica arquivos temporários de navegadores
            let filename = std::path::Path::new(&p.old_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            let is_temp = filename.starts_with(".org.chromium")
                || filename.starts_with("chromium.")
                || filename.ends_with(".crdownload")
                || filename.ends_with(".part")
                || filename.ends_with(".tmp")
                || filename.ends_with(".download");

            if is_temp {
                temp_files_count += 1;
                temp_files_size += std::fs::metadata(&p.old_path).map(|m| m.len()).unwrap_or(0);
                continue;
            }

            // Agrupa itens de baixíssima confiança (<= 0.20)
            if p.confidence <= 0.20 {
                low_conf_count += 1;
                continue;
            }

            let risk_color = match p.risk.as_str() {
                "low" => "\x1b[32m",
                "medium" => "\x1b[33m",
                "high" => "\x1b[31m",
                _ => "",
            };

            let old_path = truncate_path(&p.old_path, 30);
            let new_dir = truncate_path(&p.new_dir, 30);
            let cat = p.category_label.as_deref().unwrap_or("Incerto");
            let cat_trunc = truncate_string(cat, 15);
            let conf_str = format!("{:.2}", p.confidence);
            let risk = p.risk.to_uppercase();

            println!(
                "  {:<30} | {:<30} | {:<15} | {:<9} | {risk_color}{:<6}\x1b[0m | {}",
                old_path, new_dir, cat_trunc, conf_str, risk, p.reason
            );
            count += 1;
        }
    }

    // Exibe temporários de forma agregada
    if temp_files_count > 0 {
        let size_mb = temp_files_size as f64 / 1024.0 / 1024.0;
        let desc = format!(
            "Agrupamento de {} arquivos temporários/crdownload incompletos",
            temp_files_count
        );
        println!(
            "  {:<30} | {:<30} | {:<15} | {:<9} | \x1b[31m{:<6}\x1b[0m | {} ({:.1} MB)",
            "Downloads/Temporários/*",
            "Nenhum (Recomendado apagar)",
            "Temporários",
            "1.00",
            "HIGH",
            desc,
            size_mb
        );
        count += 1;
    }

    if count == 0 && low_conf_count == 0 {
        println!("  Nenhum arquivo encontrado em Downloads ou Desktop precisando de organização.");
    }

    if low_conf_count > 0 {
        println!(
            "  \x1b[33mℹ️ {} itens com baixíssima confiança (<= 0.20) foram agrupados e ocultados. Use 'kryonix home review' para tratá-los.\x1b[0m",
            low_conf_count
        );
    }

    let protected_inbox = plan
        .protected_files
        .iter()
        .filter(|f| {
            let p = f.path.to_lowercase();
            p.contains("/downloads/") || p.contains("/desktop/") || p.contains("/área de trabalho/")
        })
        .count();

    if protected_inbox > 0 {
        println!(
            "\n  \x1b[33m🛡️ {} itens protegidos ignorados nesta visualização.\x1b[0m",
            protected_inbox
        );
    }

    println!("────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────");
    println!(
        "\nTotal de itens na Inbox exibidos: {count} | Use \x1b[1mkryonix home review\x1b[0m para aprovar."
    );
}
