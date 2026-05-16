#!/bin/bash
set -e

# Caminho absoluto para o sandbox dentro do workspace
SANDBOX="/etc/kryonix/packages/kryonix-home/sandbox"

echo "🧹 Limpando sandbox anterior..."
rm -rf "$SANDBOX"

echo "📂 Criando estrutura do sandbox..."
mkdir -p "$SANDBOX/.config/kryonix"
mkdir -p "$SANDBOX/.local/share/kryonix/home"
mkdir -p "$SANDBOX/.local/state/kryonix/home-brain/dry-run"
mkdir -p "$SANDBOX/Downloads"
mkdir -p "$SANDBOX/Desktop"
mkdir -p "$SANDBOX/Downloads/project_folder/.git"

# Criar destinos de organização esperados no sandbox
mkdir -p "$SANDBOX/Documentos/Financeiro/Boletos"
mkdir -p "$SANDBOX/Documentos/Financeiro/Comprovantes"

echo "⚙️ Criando arquivo de configuração ~/.config/kryonix/home-autopilot.toml..."
cat <<EOF > "$SANDBOX/.config/kryonix/home-autopilot.toml"
[autopilot]
enabled = true
min_confidence = 0.60
max_actions = 10
dry_run = false
EOF

echo "📄 Criando arquivos simulados..."

# 1. Boleto (AutoMoveCertified se o score fosse mais alto, mas é de risco médio)
echo "BOLETO ENERGIA COBRANCA VALOR R$ 150,00 VENCIMENTO" > "$SANDBOX/Downloads/2026-05_boletos_luz.pdf"

# 2. Comprovante PIX
echo "COMPROVANTE DE PAGAMENTO PIX BANCO TRANSFERENCIA" > "$SANDBOX/Downloads/comprovantes_supermercado.png"

# 3. Nota Fiscal XML (AutoMoveCertified - baixo risco, multi-source score ~69% >= 60%)
echo "NF NFE DANFE NOTA FISCAL VALOR PRODUTO COMPRA" > "$SANDBOX/Downloads/notas_fiscais_danfe_compra.xml"

# 4. Executável MSI (BlockedUnsafe por extensão)
echo "MSI EXECUTABLE SETUP BINARY" > "$SANDBOX/Downloads/setup_game.msi"

# 5. Script Shell (BlockedUnsafe por extensão)
echo "#!/bin/sh" > "$SANDBOX/Downloads/installer.sh"
echo "echo 'instalar'" >> "$SANDBOX/Downloads/installer.sh"

# 6. Arquivo Sensível (BlockedUnsafe por conteúdo - Redacted)
echo "KRYONIX_BRAIN_API_KEY=abc123xyz" > "$SANDBOX/Downloads/credentials.env"
echo "PRIVATE KEY" >> "$SANDBOX/Downloads/credentials.env"

# 7. Projeto (BlockedUnsafe por ser pasta de projeto / Git)
echo "[core]" > "$SANDBOX/Downloads/project_folder/.git/config"
echo "repositoryformatversion = 0" >> "$SANDBOX/Downloads/project_folder/.git/config"
echo "cargo build" > "$SANDBOX/Downloads/project_folder/main.rs"

# 8. VM disk image (BlockedUnsafe - G1 new)
echo "QCOW2 VIRTUAL DISK" > "$SANDBOX/Downloads/test_vm.qcow2"

# 9. Database file (BlockedUnsafe - G1 new)
echo "SQLITE DATABASE" > "$SANDBOX/Downloads/app_data.sqlite"

# 10. Token file (BlockedUnsafe - G2 new)
echo "AUTH_TOKEN=test" > "$SANDBOX/Downloads/api_access.token"

# 11. PEM key file (BlockedUnsafe - G2 new)
echo "BEGIN PRIVATE KEY" > "$SANDBOX/Downloads/server.pem"

echo "✅ Sandbox estruturado com sucesso em $SANDBOX!"
echo ""
echo "=== Arquivos no sandbox ==="
find "$SANDBOX/Downloads" -type f | sort
echo ""
echo "=== Verificações de segurança ==="
echo "VM (.qcow2), DB (.sqlite), Token (.token), PEM (.pem) devem ser bloqueados pela blacklist expandida."
echo "MSI (.msi), SH (.sh), ENV (.env) devem ser bloqueados pela blacklist original."
echo "Projetos (.git) devem ser bloqueados por detecção de projeto."
