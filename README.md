# Code Terroir

Plateforme de traçabilité pour producteurs artisanaux, écrite en Rust avec PostgreSQL et Redis.

> [!WARNING]
> La version actuelle est une preuve de concept fonctionnelle destinée au développement. Les
> routes métier exigent désormais un JWT, mais le cloisonnement par producteur, le RBAC complet et
> la gestion des sessions ne sont pas encore finalisés. Ne déployez pas cette version sur Internet
> et n'y stockez pas de données réelles.

La première version fonctionnelle est figée par le tag `v0.1.0`. Le travail de sécurisation et de
refonte de l'expérience utilisateur est suivi dans le
[plan de mise en production](docs/PRODUCTION_READINESS_PLAN.md).

## Fonctions réellement disponibles

- gestion des producteurs, fournisseurs, ingrédients, recettes et produits ;
- création et suivi de lots de production ;
- contrôles qualité et rappels de lots ;
- génération de QR codes et fiche publique de traçabilité ;
- enregistrement et consultation de statistiques de scans ;
- authentification JWT appliquée aux routes métier ;
- interface HTML locale de test pour appeler l'API.

## Limites connues

- l'interface actuelle est un testeur d'API pour développeurs, pas encore l'interface métier cible ;
- filtrage des données par producteur et permissions par rôle encore incomplets ;
- jetons de rafraîchissement sans rotation ni révocation ;
- pas encore de 2FA, PWA, export PDF opérationnel ou mode hors ligne ;
- pas encore de tests d'intégration ou E2E ;
- le Compose fourni est réservé au développement local ;
- observabilité, sauvegardes et procédure de déploiement restent à finaliser.

## Architecture actuelle

- API : Rust 1.90, Warp, SQLx ;
- données : PostgreSQL 15 et Redis 7 ;
- interface de test : HTML, CSS, JavaScript et serveur local Python/Jinja2 ;
- pages publiques : HTML généré par l'API Rust ;
- environnement local : Docker Compose.

## Démarrage local

### Prérequis

- Rust 1.90 ;
- Python 3.11 ou supérieur ;
- Docker avec la commande `docker compose`.

### Installation

```bash
git clone https://github.com/gotoo77/code_terroir.git
cd code_terroir
cp .env.example .env
```

Remplacez toutes les valeurs `change-me` dans `.env`. Ce fichier est ignoré par Git et ne doit
jamais être commité.

### API et dépendances

```bash
docker compose up -d postgres redis
cargo run
```

L'API applique les migrations SQLx au démarrage.

### Interface de test

Dans un second terminal :

```bash
python3 -m venv .venv
source .venv/bin/activate
python -m pip install -r requirements.txt
python serve_gui.py
```

Adresses locales :

- interface de test : <http://localhost:8081> ;
- API : <http://localhost:3030> ;
- état de santé : <http://localhost:3030/health> ;
- fiche publique : `http://localhost:3030/t/{slug}`.

Le script `./admin.sh start` peut également lancer l'API et l'interface après le démarrage de
PostgreSQL et Redis.

## Vérifications

```bash
cargo fmt --all -- --check
cargo test --locked
./scripts/check-clippy-baseline.sh
./scripts/check-html.sh
docker compose config --quiet
docker build -t code-terroir .
```

La dette Clippy actuelle est bornée par le script de contrôle : aucune nouvelle alerte n'est
acceptée, et le plafond doit diminuer au fil des corrections.

## Configuration

Les variables sont documentées dans [.env.example](.env.example). Les plus importantes sont :

- `DATABASE_URL` et `DOCKER_DATABASE_URL` ;
- `REDIS_URL` ;
- `JWT_SECRET` ;
- `BOOTSTRAP_TOKEN`, jeton aléatoire d'au moins 32 caractères utilisé une seule fois pour créer le
  premier administrateur ;
- `CORS_ALLOWED_ORIGINS`, liste d'origines autorisées séparées par des virgules ;
- `QR_BASE_URL` et `BASE_URL` ;
- `POSTGRES_PASSWORD` et `GRAFANA_ADMIN_PASSWORD`.

Les secrets doivent être fournis par l'environnement en production, jamais intégrés à une image
ou au dépôt. L'inscription publique est désactivée après le premier compte et le rôle de ce compte
est imposé côté serveur. À ce stade, un producteur doit déjà exister en base avant cette
initialisation ; ce parcours sera remplacé par l'assistant de configuration métier.

## État du projet

- `main` : première version fonctionnelle ;
- `v0.1.0` : jalon immuable de cette version ;
- `agent/ui-ux-production-readiness` : sécurisation et refonte en cours.

Les changements sont regroupés dans la
[PR de mise en production](https://github.com/gotoo77/code_terroir/pull/1).

## Licence

MIT — voir [LICENSE](LICENSE).
