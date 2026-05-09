# Kryonix Home Brain — Fase 3B: Taxonomia Declarativa e Auditável

Este documento detalha os objetivos, arquitetura, lógica de classificação heurística e campos de auditoria introduzidos na **Fase 3B** do **Kryonix Home Brain (`kryonix-home`)**.

---

## 1. Objetivos da Fase 3B

A Fase 3B implementa a inteligência organizacional do Kryonix Home Brain de forma declarativa e determinística. Em vez de depender de uma API de IA local pesada para tomar decisões triviais, o motor utiliza uma taxonomia robusta baseada em regras de pontuação de palavras-chave, restrições de extensões e limites de risco.

Os principais pilares são:
- **Determinismo**: Mapeamento consistente de arquivos com base em regras configuráveis em TOML.
- **Higiene e Explicabilidade**: Saber exatamente *o porquê* de cada arquivo ter sido enviado para seu respectivo destino.
- **Segurança Absoluta**: Evitar perda de dados através de validação criptográfica de hashes de destino, controle de colisões e isolamento físico.

---

## 2. Arquitetura da Taxonomia e Pontuação

O motor de taxonomia lê um conjunto de regras (embutido como padrão ou estendido via arquivo TOML) e avalia cada arquivo do seguinte modo:

### 2.1. Normalização do Nome do Arquivo para Match
O arquivo tem seu nome normalizado para caixa baixa e caracteres especiais/pontuações limpos para maximizar a precisão da contagem de palavras-chave.

### 2.2. Cálculo do Score
Cada categoria elegível possui um conjunto de `keywords`. A pontuação de um arquivo para uma determinada categoria é calculada como:

$$\text{Score} = \frac{\text{Quantidade de palavras-chave encontradas}}{\text{Quantidade total de palavras-chave na categoria}}$$

Se um arquivo contiver extensões proibidas para aquela categoria ou se o score geral não atingir faixas adequadas, a classificação é penalizada.

### 2.3. Faixas de Confiança

- **Score [0.90 - 1.00] (Excelente)**: O arquivo é movido diretamente para a subpasta da categoria, com `needs_review: false` por padrão (a menos que seja um formato sensível como PDF/mídia).
- **Score [0.75 - 0.89] (Alta Confiança)**: O arquivo é movido para o diretório de destino. Arquivos leves de texto (`.txt`, `.md`, `.csv`) são marcados como livres de revisão (`needs_review: false`). PDFs, DOCX e imagens continuam exigindo revisão humana (`needs_review: true`).
- **Score [0.45 - 0.74] (Confiança Média)**: O arquivo é roteado para a pasta da categoria correspondente, mas com a flag `needs_review: true` habilitada no manifesto.
- **Score [0.00 - 0.44] (Baixa Confiança)**: O arquivo é considerado ambíguo ou fracamente classificado e é desviado para a pasta de entrada dedicada `Documentos/00_Inbox/Baixa_Confianca` com `needs_review: true`.
- **Sem Correspondência**: Se nenhuma palavra-chave der match, o arquivo vai para o fallback apropriado de acordo com seu tipo MIME/extensão (ex: `Imagens/Revisar`, `Documentos/00_Inbox/Revisar`, etc.).

---

## 3. Campos de Explicabilidade no Planejamento e Manifesto

Estendemos os modelos de dados `PlanProposal` e `ManifestAction` com campos ricos de auditoria para garantir que o usuário e a CLI possam debugar as decisões de taxonomia:

- `category_id`: ID técnica correspondente à categoria encontrada (ex: `"financeiro.bancos"`).
- `category_label`: Nome amigável legível por humanos (ex: `"Financeiro / Bancos"`).
- `category_dir`: Caminho do diretório sugerido (ex: `"Documentos/Financeiro/Bancos"`).
- `taxonomy_score`: O score exato de classificação calculado em ponto flutuante.
- `matched_keywords`: Lista das palavras-chave específicas que deram match no nome do arquivo.
- `taxonomy_reason`: Texto explicativo resumindo a lógica da classificação.
- `candidate_categories`: Vetor contendo IDs de outras categorias elegíveis caso tenha ocorrido um empate.
- `already_organized`: Indica se o arquivo já está posicionado em seu destino ideal.
- `needs_review`: Flag indicando se a ação requer validação visual do usuário antes de aplicar.

---

## 4. Casos Especiais de Controle

### 4.1. Empates de Pontuação (Tie-Breaking)
Se um arquivo gera a mesma pontuação exata para categorias diferentes (por exemplo, `comprovante_banco_trabalho.pdf` que pontua igual para `admin.comprovantes` e `trabalho.geral`), o motor detecta a colisão de pontuação e:
1. Direciona o arquivo para `Documentos/00_Inbox/Conflitos`.
2. Popula o vetor `candidate_categories` com todas as opções candidatas.
3. Define o motivo da taxonomia detalhando o conflito.

### 4.2. Destino Existente e Controle de Colisão
Durante o `apply`, se o caminho de destino já contiver um arquivo fisicamente presente:
- **Mesmo Hash (SHA-256 idêntico)**: A operação de movimentação é evitada e marcada como `skipped` (pulada), pois o arquivo já existe no destino.
- **Hash Divergente**: A operação é imediatamente bloqueada com status `blocked` / `destination_exists` e o arquivo é retido na origem. **Nenhuma perda de dados ocorre.**
