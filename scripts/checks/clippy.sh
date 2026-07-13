#!/usr/bin/env bash
set -euo pipefail

cargo clippy --locked --all-targets -- -D warnings
echo "Clippy : aucun avertissement."
