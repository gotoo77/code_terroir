#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

cargo_version="$(awk -F '"' '/^version = "/ { print $2; exit }' Cargo.toml)"
lock_version="$(awk -F '"' '
  $0 == "name = \"code-terroir\"" { package_found = 1; next }
  package_found && /^version = "/ { print $2; exit }
' Cargo.lock)"

if [[ -z "$cargo_version" || "$cargo_version" != "$lock_version" ]]; then
  echo "Versions incohérentes: Cargo.toml=$cargo_version Cargo.lock=$lock_version" >&2
  exit 1
fi

if ! grep -Fq "Version de développement ciblée : \`$cargo_version\`." CHANGELOG.md \
    && ! grep -Fq "## [$cargo_version] - " CHANGELOG.md; then
  echo "CHANGELOG.md ne cible ni ne publie la version $cargo_version." >&2
  exit 1
fi

echo "Version cohérente: $cargo_version"
