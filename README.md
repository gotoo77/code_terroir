# Code Terroir - API Backend

🌱 Plateforme de traçabilité pour producteurs artisanaux

## Architecture

- **Backend**: Rust + Warp + sqlx + PostgreSQL + Redis
- **Frontend Admin**: React + Vite + TanStack Query (PWA)
- **Frontend Atelier**: PWA mobile-first (offline-capable)
- **Frontend Public**: Pages QR ultra-légères

## Fonctionnalités

- ✅ Gestion des produits et recettes
- ✅ Traçabilité des lots de production
- ✅ Contrôles qualité (pH, température, etc.)
- ✅ Génération de QR codes et liens courts
- ✅ Pages publiques pour consommateurs
- ✅ Système de rappels
- ✅ Analytics des scans
- ✅ Export PDF des dossiers de lots
- ✅ Authentification JWT + 2FA
- ✅ Observabilité (logs, métriques, traces)

## Installation rapide

### Prérequis

- Rust 1.75+
- Docker & Docker Compose
- PostgreSQL 15+ (si pas Docker)
- Redis 7+ (si pas Docker)

### Démarrage avec Docker

```bash
# Cloner et configurer
git clone https://github.com/gotoo77/code_terroir.git
cd code_terroir
cp .env.example .env

# Démarrer l'infrastructure
docker-compose up -d postgres redis

# Installer sqlx CLI pour les migrations
cargo install sqlx-cli --no-default-features --features postgres

# Appliquer les migrations
sqlx migrate run

# Démarrer l'API
cargo run
```

### URLs de développement

- API: http://localhost:3030
- Health: http://localhost:3030/health
- API Docs: http://localhost:3030/api/v1 (TODO: OpenAPI)
- QR public: http://localhost:3030/t/{slug}

## Structure du projet

```
code-terroir/
├── src/
│   ├── api/           # Routes HTTP
│   ├── auth/          # Authentification JWT
│   ├── config/        # Configuration
│   ├── database/      # Pool PostgreSQL
│   ├── models/        # Structures de données
│   ├── services/      # Logique métier
│   ├── qr/           # Génération QR codes
│   ├── pdf/          # Export PDF
│   └── utils/        # Utilitaires
├── migrations/       # Scripts SQL
├── tests/           # Tests
└── ops/             # Docker, K8s, etc.
```

## API

Base: `/api/v1`

### Authentification
- `POST /auth/login` - Connexion
- `POST /auth/logout` - Déconnexion
- `POST /auth/refresh` - Refresh token

### Produits
- `GET /products` - Liste des produits
- `POST /products` - Créer un produit
- `GET /products/{id}` - Détail d'un produit
- `PUT /products/{id}` - Modifier un produit

### Lots
- `GET /batches` - Liste des lots
- `POST /batches` - Créer un lot
- `GET /batches/{id}` - Détail d'un lot
- `POST /batches/{id}/qa` - Ajouter contrôle QC
- `POST /batches/{id}/qr` - Générer QR code
- `POST /batches/{id}/publish` - Publier le lot
- `POST /batches/{id}/recall` - Rappeler le lot

### QR & Public
- `GET /t/{slug}` - Page publique QR (HTML)
- `GET /api/v1/qr/{slug}` - Données QR (JSON)

## Variables d'environnement

Voir `.env.example` pour la liste complète.

Principales variables :
- `DATABASE_URL` - Connexion PostgreSQL
- `REDIS_URL` - Connexion Redis
- `JWT_SECRET` - Clé secrète JWT (IMPORTANT en prod)
- `BASE_URL` - URL publique de l'API

## Tests

```bash
# Tests unitaires
cargo test

# Tests d'intégration avec base de données
cargo test --features integration-tests

# Tests E2E
cargo test --features e2e-tests
```

## Production

```bash
# Build optimisé
cargo build --release

# Docker
docker build -t code-terroir .
docker run -d --name code-terroir -p 3030:3030 code-terroir

# Avec docker-compose
docker-compose -f docker-compose.prod.yml up -d
```

## Licence

MIT - Voir [LICENSE](LICENSE)

## Contribuer

1. Fork le projet
2. Créer une branche feature (`git checkout -b feature/nouvelle-fonctionnalite`)
3. Commit (`git commit -am 'Ajouter nouvelle fonctionnalité'`)
4. Push (`git push origin feature/nouvelle-fonctionnalite`)
5. Créer une Pull Request

## Support

- 📧 Email: contact@code-terroir.com
- 📖 Documentation: https://docs.code-terroir.com
- 🐛 Issues: https://github.com/code-terroir/code-terroir/issues
