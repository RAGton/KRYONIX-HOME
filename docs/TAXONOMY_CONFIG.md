# Kryonix Home Brain — Configuração de Taxonomia (TOML)

Este documento descreve como configurar e personalizar a taxonomia de pastas declarativa do Kryonix Home Brain através do arquivo de configuração TOML.

---

## 1. Localização do Arquivo de Configuração

O motor de classificação heurística busca dinamicamente o arquivo de taxonomia nos seguintes caminhos de precedência:

1.  **Caminho do Usuário**: `~/.config/kryonix/home-taxonomy.toml`
2.  **Caminho do Sistema**: `/etc/kryonix/config/home-taxonomy.toml`
3.  **Fallback Embutido**: Caso nenhum dos arquivos acima exista, o motor carrega a taxonomia padrão embutida em código (que mapeia pastas de Administração, Estudos, Projetos, etc.).

---

## 2. Esquema do Formato TOML

O arquivo TOML é composto por um bloco de perfil global (`[profile]`) seguido por uma coleção de tabelas para cada categoria (`[[category]]`).

### 2.1. Bloco `[profile]`
- `name`: Identificador único legível do perfil de taxonomia (ex: `"kryonix-home-taxonomy-v1"`).
- `fallback_dir`: Caminho relativo da pasta onde arquivos não classificados (sem match) serão despejados (ex: `"Documentos/00_Inbox/Revisar"`).

### 2.2. Coleção `[[category]]`
Cada categoria declarada na coleção define as regras de correspondência para um determinado diretório de destino:

- `id`: Identificador técnico em caixa baixa e separado por pontos (ex: `"financeiro.bancos"`).
- `label`: Nome de exibição formatado para humanos (ex: `"Financeiro / Bancos"`).
- `dir`: Caminho relativo do diretório de destino sob a Home (ex: `"Documentos/Financeiro/Bancos"`).
- `keywords`: Vetor de strings contendo palavras-chave para a heurística de correspondência. Se qualquer uma destas palavras estiver presente no nome do arquivo (em caixa baixa), ela pontuará para esta categoria.
- `extensions`: (Opcional) Vetor de extensões permitidas para a categoria. Se fornecido, apenas arquivos com estas extensões podem dar match nesta categoria.
- `risk`: (Opcional) Nível de risco estimado das movimentações nesta pasta (valores aceitos: `"low"`, `"medium"`, `"high"`). O padrão é `"low"`.

---

## 3. Exemplo Completo de Configuração

```toml
[profile]
name = "kryonix-home-taxonomy-personal"
fallback_dir = "Documentos/00_Inbox/Revisar"

[[category]]
id = "financeiro.bancos"
label = "Financeiro / Bancos"
dir = "Documentos/Financeiro/Bancos"
keywords = ["pix", "banco", "comprovante", "extrato", "transferencia", "agencia", "conta"]
extensions = ["pdf", "txt", "csv", "xlsx", "png", "jpg"]
risk = "medium"

[[category]]
id = "estudos.nixos"
label = "Estudos / NixOS"
dir = "Documentos/Estudos/NixOS"
keywords = ["nix", "nixos", "flake", "channel", "derivacao", "home-manager"]
extensions = ["txt", "md", "nix", "pdf"]
risk = "low"

[[category]]
id = "trabalho.relatorios"
label = "Trabalho / Relatórios"
dir = "Documentos/Trabalho/Relatorios"
keywords = ["relatorio", "report", "mensal", "status", "auditoria"]
extensions = ["pdf", "docx", "xlsx"]
risk = "medium"
```

---

## 4. Melhores Práticas para Palavras-Chave

Para obter o melhor desempenho e evitar empates ou falsos-positivos indesejados:

1.  **Evite Termos Genéricos demais**: Palavras como `"arquivo"`, `"documento"`, `"teste"` ou `"upload"` podem poluir o motor de busca heurística e direcionar quase tudo para uma mesma categoria de forma indesejada.
2.  **Use Termos Específicos do Domínio**: Prefira `"comprovante"`, `"boleto"`, `"nixos"`, `"contrato"`, `"recibo"`.
3.  **Lide com Extensões Corretamente**: Se uma categoria lida com estudos ou programação (como `"estudos.rust"`), certifique-se de associar as extensões `["rs", "toml", "md", "pdf"]` para evitar que arquivos de imagem pesados caiam lá por engano.
4.  **Teste com o Subcomando `explain`**: Antes de rodar um planejamento em massa, valide arquivos individuais usando `kryonix home explain <arquivo>` para verificar se a categoria sugerida, a pontuação de confiança e as palavras-chave correspondentes estão corretas.
