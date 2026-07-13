# Versionnement et releases

Code Terroir utilise Semantic Versioning. Tant que le produit n'est pas déclaré stable, les
ruptures fonctionnelles restent possibles dans les versions `0.x` et chaque jalon porte un
suffixe de préversion.

## Jalons

- `0.1.0` : première version fonctionnelle figée.
- `0.2.0-alpha.N` : sécurité, cloisonnement, CI et organisation interne.
- `0.3.0-beta.N` : nouvelle expérience métier, responsive et accessible.
- `0.9.0-rc.N` : préproduction et pilote utilisateur.
- `1.0.0` : version de production après audit et restauration testée.

Le numéro n'est pas modifié pour chaque commit. Il évolue pour une livraison cohérente et testée.
Les corrections d'une version stable incrémentent `PATCH`; les ajouts compatibles incrémentent
`MINOR`; les ruptures après `1.0.0` incrémentent `MAJOR`.

## Préparer une version

1. Créer une branche ou une pull request de release.
2. Mettre à jour la version dans `Cargo.toml`, puis exécuter `cargo check` pour actualiser
   `Cargo.lock`.
3. Déplacer les changements concernés de `Unreleased` vers une section
   `## [VERSION] - AAAA-MM-JJ` dans `CHANGELOG.md`.
4. Vérifier localement formatage, tests, Clippy, HTML et image Docker.
5. Fusionner la pull request dans `main` et attendre une CI verte.
6. Depuis un clone propre et synchronisé sur `main`, exécuter `./scripts/release.sh VERSION`.

Le script crée un tag Git annoté `vVERSION`, le pousse puis crée la GitHub Release correspondante.
Les versions contenant un suffixe (`alpha`, `beta`, `rc`) sont publiées comme préversions.

Un tag ne doit jamais être déplacé ou recréé. Une correction nécessite un nouveau numéro.
