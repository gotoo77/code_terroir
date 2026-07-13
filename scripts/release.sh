#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

version="${1:-}"
if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
  echo "Usage: $0 VERSION (exemple: 0.2.0-alpha.1)" >&2
  exit 1
fi

for command in git gh cargo; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "Commande requise absente: $command" >&2
    exit 1
  fi
done

if [[ -n "$(git status --porcelain)" ]]; then
  echo "Le dépôt doit être propre avant une release." >&2
  exit 1
fi

branch="$(git branch --show-current)"
if [[ "$branch" != "main" ]]; then
  echo "Les releases doivent être créées depuis main, branche actuelle: $branch" >&2
  exit 1
fi

cargo_version="$(awk -F '"' '/^version = "/ { print $2; exit }' Cargo.toml)"
if [[ "$cargo_version" != "$version" ]]; then
  echo "Cargo.toml annonce $cargo_version au lieu de $version." >&2
  exit 1
fi

if ! grep -Fq "## [$version] - " CHANGELOG.md; then
  echo "CHANGELOG.md ne contient pas de section publiée pour $version." >&2
  exit 1
fi

git fetch origin main --tags
head_commit="$(git rev-parse HEAD)"
origin_main="$(git rev-parse origin/main)"
if [[ "$head_commit" != "$origin_main" ]]; then
  echo "main local doit être exactement synchronisée avec origin/main." >&2
  exit 1
fi

conclusion="$(
  gh run list --workflow CI --branch main --commit "$head_commit" --limit 1 \
    --json conclusion --jq '.[0].conclusion // ""'
)"
if [[ "$conclusion" != "success" ]]; then
  echo "La CI du commit $head_commit n'est pas verte (état: ${conclusion:-absent})." >&2
  exit 1
fi

tag="v$version"
if git rev-parse --verify --quiet "refs/tags/$tag" >/dev/null; then
  echo "Le tag $tag existe déjà localement." >&2
  exit 1
fi
if git ls-remote --exit-code --tags origin "refs/tags/$tag" >/dev/null 2>&1; then
  echo "Le tag $tag existe déjà sur origin." >&2
  exit 1
fi

notes_file="$(mktemp)"
trap 'rm -f "$notes_file"' EXIT
awk -v version="$version" '
  index($0, "## [" version "] - ") == 1 { capture = 1; next }
  capture && /^## \[/ { exit }
  capture { print }
' CHANGELOG.md >"$notes_file"

if [[ ! -s "$notes_file" ]]; then
  echo "La section CHANGELOG de $version est vide." >&2
  exit 1
fi

git tag --annotate "$tag" --message "Code Terroir $version"
git push origin "$tag"

release_options=()
if [[ "$version" == *-* ]]; then
  release_options+=(--prerelease)
fi

gh release create "$tag" \
  --verify-tag \
  --title "Code Terroir $version" \
  --notes-file "$notes_file" \
  "${release_options[@]}"

echo "Release $tag publiée avec succès."
