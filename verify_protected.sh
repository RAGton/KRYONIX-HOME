#!/usr/bin/env bash
set -euo pipefail

cd /etc/kryonix

echo "--- Setting up Sandbox ---"
tmp="$(mktemp -d)"
mkdir -p "$tmp/Downloads"
mkdir -p "$tmp/.ssh"
mkdir -p "$tmp/.config/app"
mkdir -p "$tmp/.gnupg"
mkdir -p "$tmp/PastaForaDaListaAntiga"

printf "comprovante pix banco inter valor pago" > "$tmp/Downloads/comprovante.txt"
printf "SECRET_KEY_TESTE_NAO_PODE_VAZAR" > "$tmp/.ssh/id_rsa"
printf "CONFIG_SECRET_TESTE_NAO_PODE_VAZAR" > "$tmp/.config/app/config.toml"
printf "GPG_SECRET_TESTE_NAO_PODE_VAZAR" > "$tmp/.gnupg/private.key"
printf "arquivo fora da lista antiga" > "$tmp/PastaForaDaListaAntiga/nota.txt"

echo "--- Running Scan ---"
HOME="$tmp" nix run .#kryonix-home -- scan --full-home

echo "--- Running Plan ---"
HOME="$tmp" nix run .#kryonix-home -- plan --content-aware --context-aware --summary
HOME="$tmp" nix run .#kryonix-home -- plan --content-aware --context-aware --json > /tmp/plan.json

echo "--- Verifying results ---"

echo "=== secret não pode vazar ==="
if rg -q "SECRET_KEY_TESTE_NAO_PODE_VAZAR|CONFIG_SECRET_TESTE_NAO_PODE_VAZAR|GPG_SECRET_TESTE_NAO_PODE_VAZAR" \
  /tmp/plan.json "$tmp/.local/state/kryonix/home-brain/latest-scan.json"; then
    echo "❌ FAIL: Secrets leaked into JSON!"
    exit 1
else
    echo "✅ PASS: No secrets leaked."
fi

echo "=== protected path não pode virar ação de move ==="
# Verificar se existem propostas para caminhos protegidos com ações de move/rename/move_project
if jq -e '
  .proposals[] | 
  select(
    (.old_path | test("\\.ssh|\\.config|\\.gnupg|\\.local|\\.cache|\\.env"))
    and 
    (.action | test("move|rename|move_project"))
  )
' /tmp/plan.json > /dev/null; then
    echo "❌ FAIL: Protected path found in actionable proposals!"
    jq '
      .proposals[] | 
      select(
        (.old_path | test("\\.ssh|\\.config|\\.gnupg|\\.local|\\.cache|\\.env"))
        and 
        (.action | test("move|rename|move_project"))
      )
    ' /tmp/plan.json
    exit 1
else
    echo "✅ PASS: No protected paths in actionable proposals."
fi

echo "=== arquivo normal de Downloads deve gerar proposta ==="
if rg -q "comprovante|Financeiro|Downloads|Revisar" /tmp/plan.json; then
    echo "✅ PASS: Normal file proposal generated."
else
    echo "❌ FAIL: Normal file proposal missing!"
    exit 1
fi

echo "=== full-home deve inventariar pasta fora da lista antiga ==="
if rg -q "PastaForaDaListaAntiga" "$tmp/.local/state/kryonix/home-brain/latest-scan.json"; then
    echo "✅ PASS: full-home detected extra folder."
else
    echo "❌ FAIL: full-home missed extra folder!"
    exit 1
fi

rm -rf "$tmp"
echo "--- Sandbox Cleanup Done ---"
