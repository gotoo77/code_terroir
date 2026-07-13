#!/usr/bin/env bash
set -euo pipefail

if ! command -v html5validator >/dev/null 2>&1; then
  echo "html5validator est requis : python -m pip install -r requirements-dev.txt" >&2
  exit 1
fi

tmp_dir="$(mktemp -d)"
server_pid=""

cleanup() {
  if [[ -n "$server_pid" ]]; then
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

python3 web/server.py >"$tmp_dir/server.log" 2>&1 &
server_pid="$!"

for _ in {1..10}; do
  if curl --fail --silent --show-error http://localhost:8081/ >"$tmp_dir/index.html"; then
    break
  fi
  sleep 1
done

if [[ ! -s "$tmp_dir/index.html" ]]; then
  echo "Impossible de rendre l'interface HTML." >&2
  cat "$tmp_dir/server.log" >&2
  exit 1
fi

html5validator --root "$tmp_dir" --match index.html
echo "HTML rendu valide selon le validateur Nu du W3C."
