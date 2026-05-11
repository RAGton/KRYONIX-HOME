use std::path::Path;

/// Nomes de diretórios que devem ser ignorados (hidden dirs, config, cache, secrets).
const IGNORED_DIRS: &[&str] = &[
    ".config",
    ".local",
    ".cache",
    ".ssh",
    ".gnupg",
    ".mozilla",
    ".thunderbird",
    ".var",
    ".nix-profile",
    ".nix-defexpr",
    "node_modules",
    "__pycache__",
    ".venv",
    "target",
    "result",
    ".direnv",
];

/// Extensões/nomes de arquivos secretos que nunca devem ser lidos.
const SECRET_FILES: &[&str] = &[
    ".env",
    "brain.env",
    "neo4j.env",
    "id_ed25519",
    "id_rsa",
    "id_ecdsa",
];

const SECRET_EXTENSIONS: &[&str] = &[".key", ".pem", ".secret", ".token"];

/// Retorna true se o diretório deve ser ignorado pelo scanner.
pub fn should_ignore_dir(path: &Path) -> bool {
    should_ignore_dir_options(path, false)
}

pub fn should_ignore_dir_options(path: &Path, full_home: bool) -> bool {
    let name = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return true,
    };

    // Poda imediata de áreas de cache, config, dependências ou compiladores em qualquer modo
    if name == ".cache"
        || name == ".config"
        || name == ".local"
        || name == ".mozilla"
        || name == ".var"
        || name == ".cargo"
        || name == ".rustup"
        || name == ".ssh"
        || name == ".gnupg"
        || name == "node_modules"
        || name == "target"
        || name == ".git"
        || name == ".venv"
        || name == "__pycache__"
        || name == "result"
        || name.starts_with("result-")
        || name == ".direnv"
        || name == "nix"
    {
        return true;
    }

    if full_home {
        return false;
    }

    // Ignorar diretórios ocultos (começam com .)
    if name.starts_with('.') {
        return true;
    }

    // Ignorar diretórios da lista negra
    if IGNORED_DIRS.contains(&name) {
        return true;
    }

    false
}

/// Retorna true se o arquivo deve ser ignorado pelo scanner.
pub fn should_ignore_file(path: &Path) -> bool {
    should_ignore_file_options(path, false)
}

pub fn should_ignore_file_options(path: &Path, full_home: bool) -> bool {
    let name = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return true,
    };

    if full_home {
        // No full_home, queremos inventariar arquivos ocultos
        return false;
    }

    // Ignorar arquivos ocultos (começam com .)
    if name.starts_with('.') {
        return true;
    }

    false
}

/// Retorna true se o arquivo é um secret e não deve ser processado.
pub fn is_secret_file(path: &Path) -> bool {
    let name = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return true,
    };

    if SECRET_FILES.contains(&name) {
        return true;
    }

    if SECRET_EXTENSIONS.iter().any(|ext| name.ends_with(ext)) {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_early_pruning_directories() {
        // Must ignore system/config/cache paths in BOTH standard and full_home modes
        for mode in &[false, true] {
            assert!(should_ignore_dir_options(
                Path::new("/home/user/.cache"),
                *mode
            ));
            assert!(should_ignore_dir_options(
                Path::new("/home/user/.config"),
                *mode
            ));
            assert!(should_ignore_dir_options(
                Path::new("/home/user/.local"),
                *mode
            ));
            assert!(should_ignore_dir_options(
                Path::new("/home/user/.ssh"),
                *mode
            ));
            assert!(should_ignore_dir_options(
                Path::new("/home/user/.gnupg"),
                *mode
            ));
            assert!(should_ignore_dir_options(
                Path::new("/home/user/.git"),
                *mode
            ));
            assert!(should_ignore_dir_options(
                Path::new("/home/user/node_modules"),
                *mode
            ));
            assert!(should_ignore_dir_options(
                Path::new("/home/user/target"),
                *mode
            ));
            assert!(should_ignore_dir_options(
                Path::new("/home/user/__pycache__"),
                *mode
            ));
        }
    }

    #[test]
    fn test_hidden_directories_modes() {
        // Hidden directories are ignored in standard mode
        assert!(should_ignore_dir_options(
            Path::new("/home/user/.custom_hidden"),
            false
        ));

        // Non-system hidden directories are NOT ignored in full_home mode
        assert!(!should_ignore_dir_options(
            Path::new("/home/user/.custom_hidden"),
            true
        ));
    }

    #[test]
    fn test_is_secret_file() {
        // Secret files
        assert!(is_secret_file(Path::new("/home/user/.env")));
        assert!(is_secret_file(Path::new("/home/user/id_rsa")));
        assert!(is_secret_file(Path::new("/home/user/id_ed25519")));
        assert!(is_secret_file(Path::new("/home/user/key.pem")));
        assert!(is_secret_file(Path::new("/home/user/token.secret")));

        // Normal files
        assert!(!is_secret_file(Path::new("/home/user/notes.txt")));
        assert!(!is_secret_file(Path::new("/home/user/invoice.pdf")));
    }
}
