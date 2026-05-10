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

    if full_home {
        // No full_home, queremos inventariar diretórios ocultos importantes (.ssh, .config, .local),
        // mas ainda ignorar diretórios de build/dependências gigantes e redundantes.
        if name == "node_modules"
            || name == "target"
            || name == "result"
            || name == ".direnv"
            || name == ".git"
            || name == ".cache"
        {
            return true;
        }
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
