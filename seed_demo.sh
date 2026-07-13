#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

set -a
source .env
set +a

psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f seeds/demo_data.sql

echo "Jeu de données de démo chargé."
echo "Utilisateur: atelier@apothicaire.example.com"
echo "Mot de passe: demo12345"
echo "Slug public: veldemo1"
