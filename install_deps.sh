#!/usr/bin/env bash
#
# install_deps.sh — installe les dépendances et initialise PostgreSQL pour Code Terroir
#

set -e

DB_USER="${DB_USER:-code_terroir}"
DB_PASS="${DB_PASS:?Définissez DB_PASS avant de lancer ce script}"
DB_NAME="${DB_NAME:-code_terroir_dev}"
PG_SERVICE="postgresql"

echo "🌱 Détection du système…"
if command -v apt >/dev/null 2>&1; then
    PKG_MANAGER="apt"
    UPDATE_CMD="sudo apt update -y"
    INSTALL_CMD="sudo apt install -y"
elif command -v dnf >/dev/null 2>&1; then
    PKG_MANAGER="dnf"
    UPDATE_CMD="sudo dnf update -y"
    INSTALL_CMD="sudo dnf install -y"
else
    echo "❌ Aucun gestionnaire de paquets compatible trouvé."
    exit 1
fi

echo "✅ Gestionnaire détecté : $PKG_MANAGER"
echo "────────────────────────────────────────────"
echo "🧰 Mise à jour des paquets…"
$UPDATE_CMD

echo "────────────────────────────────────────────"
echo "📦 Installation des dépendances système…"
$INSTALL_CMD curl wget git unzip tar pkg-config build-essential cmake

if [ "$PKG_MANAGER" = "dnf" ]; then
    $INSTALL_CMD gcc-c++ make python3-pip openssl-devel libpq-devel
else
    $INSTALL_CMD g++ make python3-pip libssl-dev libpq-dev
fi

echo "────────────────────────────────────────────"
echo "🐘 Installation de PostgreSQL…"
if ! command -v psql >/dev/null 2>&1; then
    $INSTALL_CMD postgresql postgresql-contrib
    sudo systemctl enable --now $PG_SERVICE || true
else
    echo "✅ PostgreSQL déjà installé"
fi

echo "────────────────────────────────────────────"
echo "🔥 Installation de Redis…"
if ! command -v redis-server >/dev/null 2>&1; then
    $INSTALL_CMD redis redis-server || $INSTALL_CMD redis
    sudo systemctl enable --now redis || sudo systemctl enable --now redis-server || true
else
    echo "✅ Redis déjà installé"
fi

echo "────────────────────────────────────────────"
echo "🐍 Installation Python et GUI…"
$INSTALL_CMD python3 python3-pip
pip3 install --upgrade pip
pip3 install flask requests

echo "────────────────────────────────────────────"
echo "🦀 Installation de Rust (si absent)…"
if ! command -v cargo >/dev/null 2>&1; then
    echo "📥 Installation de Rust via rustup…"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo "✅ Rust déjà installé"
fi

echo "────────────────────────────────────────────"
echo "🗄️  Configuration PostgreSQL locale…"
sudo systemctl start $PG_SERVICE || true

# Création de l’utilisateur et de la base
sudo -i -u postgres psql <<SQL
DO
\$do\$
BEGIN
   IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = '$DB_USER') THEN
      CREATE ROLE $DB_USER LOGIN PASSWORD '$DB_PASS';
   END IF;
END
\$do\$;

CREATE DATABASE $DB_NAME OWNER $DB_USER TEMPLATE template1
    ENCODING 'UTF8'
    LC_COLLATE='C'
    LC_CTYPE='C';

GRANT ALL PRIVILEGES ON DATABASE $DB_NAME TO $DB_USER;
SQL

echo "✅ Base et utilisateur PostgreSQL configurés :"
echo "   → utilisateur : $DB_USER"
echo "   → base        : $DB_NAME"
echo "   → mot de passe: $DB_PASS"

# Vérification
PG_CONN="postgresql://$DB_USER:$DB_PASS@localhost:5432/$DB_NAME"
echo "🔍 Test de connexion :"
psql "$PG_CONN" -c "SELECT '✅ Connexion réussie à $DB_NAME';"

echo "────────────────────────────────────────────"
echo "🧱 (Optionnel) Migration SQLx si disponible…"
if [ -d "./migrations" ]; then
    if command -v sqlx >/dev/null 2>&1; then
        sqlx migrate run
        echo "✅ Migrations SQLx appliquées."
    else
        echo "⚠️  sqlx-cli non installé. Pour l’installer : cargo install sqlx-cli"
    fi
else
    echo "ℹ️  Aucun dossier migrations/ trouvé (pas de migration appliquée)."
fi

echo "────────────────────────────────────────────"
echo "🌾 Installation terminée !"
echo ""
echo "➡️  PostgreSQL : sudo systemctl status $PG_SERVICE"
echo "➡️  Redis       : sudo systemctl status redis"
echo "➡️  Test DB     : psql $PG_CONN"
echo ""
echo "Vous pouvez maintenant exécuter :"
echo "   ./admin.sh start"
echo "ou"
echo "   cargo run"
