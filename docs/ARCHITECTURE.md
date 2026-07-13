# Architecture du projet

## Répertoires

- `src/` : API Rust, modèles et accès aux infrastructures.
- `web/` : serveur local Python et interface HTML/CSS/JavaScript.
- `config/` : référentiels métier versionnés, sans secret.
- `migrations/` : migrations SQLx immuables après application.
- `seeds/` : données de démonstration.
- `scripts/` : commandes de développement, contrôles et utilitaires SQL.
- `ops/` : configuration Docker, proxy et observabilité.
- `docs/` : spécifications, plans et documents archivés.

## Types de configuration

Les secrets, URLs et paramètres propres à un environnement sont fournis par variables
d'environnement. Ils ne doivent jamais être enregistrés dans le dépôt.

Les listes métier partagées sont définies dans `config/reference-data.json`. Le fichier est
embarqué dans le binaire Rust et lu directement par l'interface Python. Un fichier externe peut
être choisi avec `REFERENCE_DATA_PATH`; il est validé au démarrage.

Les préférences propres à un producteur devront être stockées en base de données lorsqu'elles
deviendront administrables depuis l'interface.

## Principes d'évolution

- Ne jamais modifier une migration déjà appliquée; créer une nouvelle migration.
- Ne pas dupliquer un référentiel entre l'API et l'interface.
- Garder les gestionnaires HTTP minces et déplacer progressivement la logique vers des modules
  métier testables.
- Ajouter une fonctionnalité réelle plutôt qu'un module vide réservé à un usage futur.
