# Kryonix Home Brain

Kryonix Home Brain é o motor seguro, declarativo e determinístico de organização da Home do Kryonix.

## Recursos

- **Scan Seguro da Home**: Verifica arquivos em diretórios típicos de entrada (como `Downloads`) sem realizar qualquer mutação.
- **Plano Dry-Run**: Gera uma simulação completa das movimentações propostas antes de qualquer escrita física.
- **Detecção de Duplicatas por SHA-256**: Identifica e pula arquivos duplicados automaticamente para evitar redundância.
- **Manifesto Auditável**: Armazena todas as decisões de movimentação em arquivos JSON estruturados.
- **Apply com Confirmação Explícita**: Aplica as mudanças somente quando confirmado expressamente pelo usuário.
- **Rollback de 100% de Fidelidade**: Permite reverter as ações de movimentação a qualquer momento, restaurando os caminhos e nomes originais.
- **Renomeação ABNT-like**: Normaliza nomes de arquivos para o formato padronizado `YYYY-MM-DD_Nome_vN.ext`.
- **Taxonomia Determinística**: Heurísticas robustas para classificar arquivos de acordo com o seu conteúdo sem necessidade de IA local.
- **Configuração via TOML**: Carrega preferências e regras personalizadas de mapeamento a partir de um arquivo declarativo.
- **Explicabilidade por Categoria**: Informa a pontuação (`score`), palavras-chave correspondentes e motivos que levaram a cada classificação.

## Comandos

```bash
kryonix home scan
kryonix home report
kryonix home duplicates
kryonix home plan --taxonomy-suggestions --rename-suggestions --why
kryonix home categories
kryonix home categories --json
kryonix home explain Downloads/arquivo.pdf
kryonix home manifest create --taxonomy-suggestions --rename-suggestions
kryonix home manifest show
kryonix home apply --dry-run
kryonix home apply --confirm
kryonix home rollback
```

## Segurança

- Nenhum arquivo é movido sem `apply --confirm`.
- `apply --confirm` deve ser revisado previamente através do manifesto.
- Rollback é garantido e seguro, com recuperação total de estado.
- Um destino existente nunca é sobrescrito (`destination_exists` bloqueia a operação).
- Arquivos idênticos por assinatura hash SHA-256 podem ser pulados (`skipped`) de forma segura.
- Arquivos ambíguos ou com empates de score são roteados para a pasta de conflitos.
- Arquivos ou pastas ocultas/de configuração nunca são afetados por padrão.

## Taxonomia

O arquivo opcional de taxonomia personalizada fica localizado em:

```txt
~/.config/kryonix/home-taxonomy.toml
```

Exemplo de configuração:

```toml
[profile]
name = "kryonix-home-taxonomy-custom"
fallback_dir = "Documentos/00_Inbox/Revisar"

[[category]]
id = "financeiro.bancos"
label = "Financeiro / Bancos"
dir = "Documentos/Financeiro/Bancos"
keywords = ["pix", "banco", "comprovante"]
extensions = ["pdf", "txt", "jpg", "png"]
risk = "medium"
```

## Desenvolvimento

Para testar e compilar localmente:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo build
```
