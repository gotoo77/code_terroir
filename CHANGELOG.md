# Changelog

Les changements notables de Code Terroir sont consignés dans ce fichier. Le projet suit
[Semantic Versioning](https://semver.org/lang/fr/) et le format
[Keep a Changelog](https://keepachangelog.com/fr/1.1.0/).

## [Unreleased]

Version de développement ciblée : `0.2.0-alpha.2`.

### Added

- CI pour le formatage, les tests, Clippy, le HTML, les secrets et l'image Docker.
- Authentification JWT, sessions persistantes et rotation/révocation des jetons de rafraîchissement.
- Protection anti-bruteforce Redis avec réponse `429` et en-tête `Retry-After`.
- Limite de taille des requêtes, identifiants de corrélation et en-têtes HTTP de sécurité.
- Initialisation contrôlée du premier administrateur et de sa première exploitation.
- Référentiel configurable et partagé pour les allergènes UE, catégories et unités.
- Documentation de l'architecture et du plan de préparation à la production.

### Changed

- Cloisonnement systématique des données par producteur et matrice RBAC centralisée sur les routes.
- Rotation des secrets locaux d'initialisation et durcissement de la configuration CORS/Nginx.
- Réorganisation des scripts, documents, configurations et composants d'authentification.
- Interface d'initialisation clarifiée et HTML rendu validé par le validateur Nu du W3C.

### Removed

- Prototypes HTML, anciens écrans monolithiques, endpoint QR factice et modules Rust vides.
- Dépendance Rust `config` inutilisée.

### Security

- Filtrage du producteur jusque dans les mutations et agrégations SQL des contrôles qualité,
  recettes, QR codes et liaisons produit-ingrédient.
- Couverture automatisée de toutes les combinaisons de rôles et d'actions privées.
- Neutralisation des erreurs internes exposées aux clients.
- Vérification constante du jeton d'initialisation et simulation Argon2 pour les comptes inconnus.
- Refus explicite des comptes TOTP tant que leur validation n'est pas implémentée.
- Scan de l'historique Git contre les secrets dans la CI.

## [0.1.0] - 2026-07-13

### Added

- Première version fonctionnelle de l'API de traçabilité artisanale.
- Gestion initiale des producteurs, produits, ingrédients, fournisseurs, recettes, lots, contrôles
  qualité et QR codes.
- Interface locale de test de l'API.

[Unreleased]: https://github.com/gotoo77/code_terroir/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/gotoo77/code_terroir/releases/tag/v0.1.0
