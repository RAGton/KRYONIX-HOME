mod apply;
mod cli;
pub mod content;
pub mod context;
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
mod rollback;
mod scanner;
pub mod taxonomy;

fn main() {
    if let Err(e) = cli::run() {
        eprintln!("Erro: {e:#}");
        std::process::exit(1);
    }
}
