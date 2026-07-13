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

## Autorisation et cloisonnement

Les permissions d'écriture sont définies dans une matrice unique dans
`src/api/authorization.rs`. Les routes de lecture restent accessibles à tout compte actif, mais
chaque requête filtre systématiquement les données avec le `producer_id` issu du jeton.

| Action | Admin | Qualité | Atelier | Logistique | Lecture seule |
|---|---:|---:|---:|---:|---:|
| Gérer le catalogue | Oui | Non | Oui | Non | Non |
| Gérer les lots | Oui | Non | Oui | Non | Non |
| Rappeler un lot | Oui | Oui | Non | Non | Non |
| Créer un contrôle qualité | Oui | Oui | Oui | Non | Non |
| Modifier un contrôle qualité | Oui | Oui | Non | Non | Non |
| Gérer les QR codes | Oui | Non | Oui | Non | Non |
| Modifier le producteur | Oui | Non | Non | Non | Non |

Le cloisonnement est répété dans les requêtes SQL finales, y compris lorsqu'une vérification
d'appartenance a déjà été effectuée auparavant. Cette redondance volontaire évite qu'une écriture
inter-producteurs devienne possible lors d'une évolution ultérieure du code.
