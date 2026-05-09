mod apply;
mod cli;
mod hashing;
mod ignore;
mod manifest;
mod metadata;
mod planner;
mod report;
mod rollback;
mod scanner;

fn main() {
    if let Err(e) = cli::run() {
        eprintln!("Erro: {e:#}");
        std::process::exit(1);
    }
}
