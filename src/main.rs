mod apply;
pub mod autopilot;
mod cli;
pub mod content;
pub mod context;
pub mod decision;
mod export;
mod hashing;
mod ignore;
mod manifest;
mod metadata;
pub mod naming;
pub mod ollama;
mod planner;
mod project;
mod report;
mod review;
mod rollback;
mod scanner;
pub mod state;
pub mod taxonomy;

fn main() {
    if let Err(e) = cli::run() {
        eprintln!("Erro: {e:#}");
        std::process::exit(1);
    }
}
