#!/usr/bin/env bash
set -euo pipefail

readonly BASELINE=67
output="$(mktemp)"
trap 'rm -f "$output"' EXIT

if ! cargo clippy --locked --all-targets --message-format=short 2>&1 | tee "$output"; then
  echo "Clippy n'a pas pu terminer." >&2
  exit 1
fi

warning_count="$(grep -c 'warning:' "$output" || true)"

if (( warning_count > BASELINE )); then
  echo "Dette Clippy aggravée : ${warning_count} avertissements, plafond ${BASELINE}." >&2
  exit 1
fi

echo "Dette Clippy contenue : ${warning_count}/${BASELINE}."

if (( warning_count < BASELINE )); then
  echo "Le plafond peut maintenant être abaissé à ${warning_count}."
fi
