use crate::decision::DecisionClass;
use crate::planner::{Plan, PlanProposal};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutopilotConfig {
    pub enabled: bool,
    #[serde(default = "default_min_confidence")]
    pub min_confidence: f64,
    #[serde(default = "default_max_actions")]
    pub max_actions: usize,
    #[serde(default = "default_true")]
    pub dry_run: bool,
    #[serde(default = "default_false")]
    pub staging_only: bool,
    #[serde(default = "default_blacklist_extensions")]
    pub blacklist_extensions: Vec<String>,
    #[serde(default = "default_blacklist_folders")]
    pub blacklist_folders: Vec<String>,
}

fn default_min_confidence() -> f64 {
    0.95
}
fn default_max_actions() -> usize {
    100
}
fn default_true() -> bool {
    true
}
fn default_false() -> bool {
    false
}

fn default_blacklist_extensions() -> Vec<String> {
    vec![
        // Executables & scripts
        "exe".to_string(),
        "msi".to_string(),
        "sh".to_string(),
        "bat".to_string(),
        "ps1".to_string(),
        "bin".to_string(),
        "run".to_string(),
        // Virtual machines
        "qcow2".to_string(),
        "vmdk".to_string(),
        "vdi".to_string(),
        "vhd".to_string(),
        "vhdx".to_string(),
        // Databases
        "sqlite".to_string(),
        "db".to_string(),
        "sqlite3".to_string(),
        // Secrets & keys
        "env".to_string(),
        "token".to_string(),
        "secret".to_string(),
        "key".to_string(),
        "pem".to_string(),
    ]
}

fn default_blacklist_folders() -> Vec<String> {
    vec![
        "Obsidian Vault".to_string(),
        ".ssh".to_string(),
        ".gnupg".to_string(),
        ".config".to_string(),
        ".env".to_string(),
        ".password-store".to_string(),
        ".pki".to_string(),
        "VMs".to_string(),
        "libvirt".to_string(),
        ".local/share/gnome-boxes".to_string(),
        ".local/share/libvirt".to_string(),
    ]
}

impl Default for AutopilotConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            min_confidence: 0.95,
            max_actions: 100,
            dry_run: true,
            staging_only: false,
            blacklist_extensions: default_blacklist_extensions(),
            blacklist_folders: default_blacklist_folders(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutopilotConfigContainer {
    pub autopilot: Option<AutopilotConfig>,
}

pub fn load_autopilot_config() -> AutopilotConfig {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home/rocha"));
    let config_path = home.join(".config/kryonix/home-autopilot.toml");

    if !config_path.exists() {
        eprintln!("⚠️ Arquivo de configuração do piloto automático não encontrado em ~/.config/kryonix/home-autopilot.toml. Usando configurações padrão de segurança.");
        return AutopilotConfig::default();
    }

    if let Ok(content) = fs::read_to_string(&config_path) {
        if let Ok(container) = toml::from_str::<AutopilotConfigContainer>(&content) {
            if let Some(cfg) = container.autopilot {
                return cfg;
            }
        }
        if let Ok(cfg) = toml::from_str::<AutopilotConfig>(&content) {
            return cfg;
        }
    }
    eprintln!("⚠️ Falha ao analisar ~/.config/kryonix/home-autopilot.toml. Usando configurações padrão de segurança.");
    AutopilotConfig::default()
}

/// Returns the allowed hostnames for autopilot execution.
/// Only the Inspiron workstation should run autopilot on the user HOME.
/// Glacier is a server and must never organize user HOME files.
fn allowed_autopilot_hostnames() -> Vec<&'static str> {
    vec!["inspiron", "inspiron-nina"]
}

/// Checks if the current host is allowed to run the autopilot.
fn is_host_allowed() -> Result<bool> {
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_lowercase().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let allowed = allowed_autopilot_hostnames();
    // Allow if hostname starts with any allowed prefix, or if hostname is unknown (dev/sandbox)
    Ok(allowed.iter().any(|a| hostname.starts_with(a)) || hostname == "unknown")
}

pub fn run_autopilot(
    execute_flag: bool,
    dry_run_flag: bool,
    inbox_only: bool,
    max_actions_override: Option<usize>,
    min_confidence_override: Option<f64>,
) -> Result<()> {
    println!("🤖 Iniciando Kryonix Home Brain - Safe Autonomous Autopilot");

    // Gate multi-host: bloquear execução em hosts não permitidos (ex: Glacier)
    if !is_host_allowed()? {
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        anyhow::bail!(
            "❌ BLOQUEADO: O autopilot de Home não pode ser executado no host '{}'.\n\
             O autopilot de organização da HOME é permitido apenas em workstations\n\
             cliente (inspiron, inspiron-nina). O Glacier é um servidor de IA e\n\
             não deve organizar arquivos de HOME do usuário.\n\n\
             Hosts permitidos: {:?}",
            hostname,
            allowed_autopilot_hostnames()
        );
    }

    // 1. Carregar Configuração do Autopilot
    let mut config = load_autopilot_config();

    // Sobrescritas via CLI
    if let Some(m) = max_actions_override {
        config.max_actions = m;
    }
    if let Some(c) = min_confidence_override {
        config.min_confidence = c;
    }

    // Se o usuário passou --execute mas dry_run no CLI é explicitado ou a config diz dry_run, prioriza segurança
    let is_dry_run = if dry_run_flag {
        true
    } else if execute_flag {
        false
    } else {
        config.dry_run
    };

    println!(
        "   Modo: {}",
        if is_dry_run {
            "Dry-Run (Simulação)"
        } else {
            "EXECUÇÃO AUTÔNOMA"
        }
    );
    println!("   Confiança Mínima: {:.0}%", config.min_confidence * 100.0);
    println!("   Limite Máximo de Ações: {}", config.max_actions);

    // 2. Gate de Segurança Crítico: Se for EXECUÇÃO real, a config DEVE ter enabled = true
    if !is_dry_run && !config.enabled {
        anyhow::bail!(
            "❌ ERRO DE SEGURANÇA: O piloto automático está desabilitado na configuração!\n\
             Para permitir a execução autônoma, você deve criar/editar o arquivo:\n\
             ~/.config/kryonix/home-autopilot.toml\n\n\
             E definir:\n\
             [autopilot]\n\
             enabled = true"
        );
    }

    // 3. Executar o Scan Inteligente (Content-Aware, Redaction-Safe)
    println!("🧠 Realizando varredura inteligente do sistema de arquivos...");
    let scan = crate::scanner::run_scan_options(false, false, true, inbox_only)?;
    println!(
        "   Varredura concluída. Arquivos analisados: {}, Projetos detectados: {}",
        scan.files_analyzed, scan.project_count
    );

    // 4. Carregar taxonomia e gerar o Plano Inicial
    let _taxonomy_config = crate::taxonomy::load_taxonomy_config(None);
    let plan_options = crate::planner::PlanOptions {
        rename_suggestions: true,
        taxonomy_suggestions: true,
        taxonomy_config_path: None,
        include_large_files: true,
        safe_only: false, // Nós vamos filtrar no Autopilot de forma extremamente rígida
        review_only: false,
        projects_only: false,
        limit: None,
        ollama: false,
        full_home: false,
        content_aware: true,
        context_aware: true,
        min_confidence: Some(config.min_confidence),
    };

    let plan = crate::planner::generate_plan(&scan, &plan_options);

    // 5. Filtragem Autopilot Baseada em Políticas Rígidas de Segurança
    let mut auto_move_certified = Vec::new();
    let mut needs_human_review = Vec::new();
    let mut blocked_unsafe = Vec::new();
    let mut ignored_noise = Vec::new();
    let mut keep_in_place = Vec::new();

    for mut proposal in plan.proposals {
        let extension = Path::new(&proposal.old_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let path_lower = proposal.old_path.to_lowercase();

        // Aplicar blacklists estritas da configuração do autopilot
        let is_blacklisted_extension = config
            .blacklist_extensions
            .iter()
            .any(|ext| ext.to_lowercase() == extension);
        let is_blacklisted_folder = config
            .blacklist_folders
            .iter()
            .any(|f| path_lower.contains(&f.to_lowercase()));

        if is_blacklisted_extension || is_blacklisted_folder {
            proposal.decision_class = DecisionClass::BlockedUnsafe;
            proposal.blocked_from_apply = true;
            proposal.auto_apply_allowed = false;
            proposal
                .safety_flags
                .push("blacklisted_by_autopilot_config".to_string());
        }

        // Se a confiança calculada for menor que o limite mínimo de segurança do autopilot (mínimo absoluto 0.95), rebaixar incondicionalmente
        if proposal.decision_class == DecisionClass::AutoMoveCertified
            && (proposal.confidence < 0.95
                || proposal.confidence < config.min_confidence
                || proposal.risk != "low")
        {
            proposal.decision_class = DecisionClass::NeedsHumanReview;
            proposal.auto_apply_allowed = false;
            proposal
                .safety_flags
                .push("confidence_below_autopilot_threshold".to_string());
        }

        // Se estiver configurado apenas para Staging (StagingOnly), marcar
        if config.staging_only && proposal.decision_class == DecisionClass::AutoMoveCertified {
            proposal.staging_only = true;
            proposal.new_dir = "Documentos/00_Inbox/Downloads/Revisar".to_string();
            proposal.destination = Path::new(&scan.home_dir)
                .join(&proposal.new_dir)
                .join(proposal.new_filename.as_deref().unwrap_or_else(|| {
                    Path::new(&proposal.old_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                }))
                .to_string_lossy()
                .to_string();
        }

        // Separar em baldes/classes para reporte claro
        match proposal.decision_class {
            DecisionClass::AutoMoveCertified => {
                auto_move_certified.push(proposal);
            }
            DecisionClass::NeedsHumanReview => {
                needs_human_review.push(proposal);
            }
            DecisionClass::BlockedUnsafe => {
                blocked_unsafe.push(proposal);
            }
            DecisionClass::IgnoreNoise => {
                ignored_noise.push(proposal);
            }
            DecisionClass::KeepInPlace => {
                keep_in_place.push(proposal);
            }
        }
    }

    // Ordenar certificados de movimentação por confiança (decrescente)
    auto_move_certified.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Aplicar limite máximo de ações (max_actions)
    let mut final_auto_apply = Vec::new();
    for (idx, prop) in auto_move_certified.into_iter().enumerate() {
        if idx < config.max_actions {
            final_auto_apply.push(prop);
        } else {
            let mut demoted = prop;
            demoted.decision_class = DecisionClass::NeedsHumanReview;
            demoted.auto_apply_allowed = false;
            demoted
                .safety_flags
                .push("demoted_due_to_max_actions_limit".to_string());
            needs_human_review.push(demoted);
        }
    }

    // 6. Gerar o Relatório Final Consolidado em Formato Markdown & Impressão Visual
    print_autopilot_summary(
        &final_auto_apply,
        &needs_human_review,
        &blocked_unsafe,
        &ignored_noise,
        &keep_in_place,
    );

    if final_auto_apply.is_empty() {
        println!(
            "\n✨ Nenhuma ação certificada para movimentação automática encontrada ou permitida."
        );
        if !needs_human_review.is_empty() {
            println!("💡 Há {} itens na fila de revisão humana. Execute 'kryonix home review' para tratá-los.", needs_human_review.len());
        }
        return Ok(());
    }

    // 7. Criar e Aplicar Manifesto para Ações Certificadas
    let filtered_plan = Plan {
        run_id: plan.run_id.clone(),
        mode: "autopilot".to_string(),
        home_dir: plan.home_dir.clone(),
        files_seen: plan.files_seen,
        projects_seen: plan.projects_seen,
        proposals: final_auto_apply.clone(),
        protected_files: plan.protected_files.clone(),
        content_aware: plan.content_aware,
        context_aware: plan.context_aware,
        full_home: plan.full_home,
        schema_version: plan.schema_version.clone(),
    };

    println!("\n📦 Gerando manifesto oficial para ações seguras...");
    let mut manifest = crate::manifest::create_manifest(&filtered_plan, &scan)?;

    // Modificar o status das ações que não devem ser aplicadas automaticamente no manifesto, apenas por precaução
    for action in &mut manifest.actions {
        if action.decision_class != "AutoMoveCertified"
            || !action.auto_apply_allowed
            || action.blocked_from_apply
        {
            action.status = "skipped_policy_gate".to_string();
        }
    }

    // Executar apply (seja dry_run ou real)
    crate::apply::run_apply(&mut manifest, is_dry_run)?;

    // Salvar relatório estruturado do dry-run para auditoria
    if is_dry_run {
        save_dry_run_audit(
            &manifest,
            &final_auto_apply,
            &needs_human_review,
            &blocked_unsafe,
        )?;
    }

    if !is_dry_run {
        println!("\n🚀 Operação de Autopiloto Seguro concluída com absoluto SUCESSO!");
        println!("   Se necessário desfazer as ações realizadas, execute:");
        println!("   -> kryonix home autopilot --undo-last");
    } else {
        println!("\n💡 Simulação do piloto automático concluída. Nenhuma alteração foi efetuada.");
    }

    Ok(())
}

fn print_autopilot_summary(
    certified: &[PlanProposal],
    review: &[PlanProposal],
    blocked: &[PlanProposal],
    ignored: &[PlanProposal],
    keep: &[PlanProposal],
) {
    println!("\n==================================================");
    println!("       RELATÓRIO DE DECISÃO DE AUTOPILOTO");
    println!("==================================================");
    println!(
        " ✅ AutoMoveCertified:  {} itens (Movimentação Automática Segura)",
        certified.len()
    );
    println!(
        " 🔍 NeedsHumanReview:   {} itens (Requer Aprovação Humana)",
        review.len()
    );
    println!(
        " ❌ BlockedUnsafe:      {} itens (Arquivos Sensíveis / Bloqueados)",
        blocked.len()
    );
    println!(
        " ⏭️ IgnoreNoise:        {} itens (Arquivos Temporários / Ruído)",
        ignored.len()
    );
    println!(
        " 📌 KeepInPlace:         {} itens (Já no local correto)",
        keep.len()
    );
    println!("--------------------------------------------------");

    if !certified.is_empty() {
        println!("\n🚀 ITENS CERTIFICADOS PARA MOVIMENTAÇÃO AUTOMÁTICA:");
        for action in certified {
            let filename = Path::new(&action.old_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            let dest_name = action.new_filename.as_deref().unwrap_or(filename);
            println!(
                "  • [{:.0}%] {} -> {}/{}",
                action.confidence * 100.0,
                filename,
                action.new_dir,
                dest_name
            );
        }
    }

    if !review.is_empty() {
        println!("\n🔍 ITENS BLOQUEADOS PARA FILA DE REVISÃO HUMANA (AMBÍGUOS OU OUTROS):");
        for action in review.iter().take(10) {
            let filename = Path::new(&action.old_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            println!(
                "  • [{:.0}%] {} -> {} (Motivo: {})",
                action.confidence * 100.0,
                filename,
                action.new_dir,
                action.reason
            );
        }
        if review.len() > 10 {
            println!("  ... e mais {} itens.", review.len() - 10);
        }
    }

    if !blocked.is_empty() {
        println!("\n❌ ITENS BLOQUEADOS POR RISCO DE SEGURANÇA (EXCLUSÃO ESTRITA):");
        for action in blocked.iter().take(10) {
            let filename = Path::new(&action.old_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            let warning_str = action.safety_flags.join(", ");
            println!(
                "  • {} (Aviso: {})",
                filename,
                if warning_str.is_empty() {
                    "Filtro de segurança estrito"
                } else {
                    &warning_str
                }
            );
        }
        if blocked.len() > 10 {
            println!("  ... e mais {} itens.", blocked.len() - 10);
        }
    }
}

/// Salva um relatório JSON estruturado do dry-run para auditoria.
fn save_dry_run_audit(
    manifest: &crate::manifest::Manifest,
    certified: &[PlanProposal],
    review: &[PlanProposal],
    blocked: &[PlanProposal],
) -> Result<()> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home/rocha"));
    let dir = home.join(".local/state/kryonix/home-brain/dry-run");
    fs::create_dir_all(&dir)?;

    let audit = serde_json::json!({
        "type": "dry_run_audit",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "hostname": hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string()),
        "run_id": manifest.run_id,
        "schema_version": manifest.schema_version,
        "summary": {
            "auto_move_certified": certified.len(),
            "needs_human_review": review.len(),
            "blocked_unsafe": blocked.len(),
            "total_actions": manifest.actions.len(),
        },
        "certified_items": certified.iter().map(|p| {
            serde_json::json!({
                "source": p.old_path,
                "destination": p.destination,
                "confidence": p.confidence,
                "risk": p.risk,
                "decision_class": format!("{:?}", p.decision_class),
                "reason": p.reason,
            })
        }).collect::<Vec<_>>(),
        "blocked_items": blocked.iter().map(|p| {
            serde_json::json!({
                "source": p.old_path,
                "safety_flags": p.safety_flags,
                "decision_class": format!("{:?}", p.decision_class),
            })
        }).collect::<Vec<_>>(),
    });

    let filename = format!(
        "dry_run_{}.json",
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    );
    let path = dir.join(&filename);
    fs::write(&path, serde_json::to_string_pretty(&audit)?)?;
    println!("📋 Relatório de dry-run salvo em: {}", path.display());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autopilot_config_defaults() {
        let config = AutopilotConfig::default();
        assert!(!config.enabled, "Autopilot must be disabled by default");
        assert!(config.dry_run, "Dry-run must be true by default");
        assert!(
            (config.min_confidence - 0.95).abs() < f64::EPSILON,
            "Min confidence must be 0.95 by default"
        );
        assert_eq!(config.max_actions, 100);
        assert!(!config.staging_only);
    }

    #[test]
    fn test_autopilot_blacklist_vm_files() {
        let config = AutopilotConfig::default();
        let vm_exts = ["qcow2", "vmdk", "vdi", "vhd", "vhdx"];
        for ext in &vm_exts {
            assert!(
                config.blacklist_extensions.iter().any(|e| e == ext),
                "VM extension '{}' must be in default blacklist",
                ext
            );
        }
    }

    #[test]
    fn test_autopilot_blacklist_databases() {
        let config = AutopilotConfig::default();
        let db_exts = ["sqlite", "db", "sqlite3"];
        for ext in &db_exts {
            assert!(
                config.blacklist_extensions.iter().any(|e| e == ext),
                "Database extension '{}' must be in default blacklist",
                ext
            );
        }
    }

    #[test]
    fn test_autopilot_blacklist_secrets() {
        let config = AutopilotConfig::default();
        let secret_exts = ["env", "token", "secret", "key", "pem"];
        for ext in &secret_exts {
            assert!(
                config.blacklist_extensions.iter().any(|e| e == ext),
                "Secret extension '{}' must be in default blacklist",
                ext
            );
        }
    }

    #[test]
    fn test_autopilot_blacklist_folders() {
        let config = AutopilotConfig::default();
        let sensitive_folders = [".ssh", ".gnupg", ".config", ".password-store", ".pki"];
        for folder in &sensitive_folders {
            assert!(
                config.blacklist_folders.iter().any(|f| f == folder),
                "Sensitive folder '{}' must be in default blacklist",
                folder
            );
        }
    }

    #[test]
    fn test_autopilot_load_default_when_no_config() {
        // When config file doesn't exist, should return safe defaults
        let config = load_autopilot_config();
        assert!(!config.enabled);
        assert!(config.dry_run);
        assert!((config.min_confidence - 0.95).abs() < f64::EPSILON);
    }
}
