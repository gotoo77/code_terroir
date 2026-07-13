# Plan de mise en production

## Ambition

Code Terroir doit devenir un outil métier utilisable sans connaissance des API, y compris sur
mobile et avec des technologies d'assistance. L'interface ne doit exposer ni verbes HTTP, ni
routes, ni UUID, ni JSON. Elle doit guider l'utilisateur dans ses tâches : gérer son catalogue,
préparer un lot, effectuer les contrôles, publier une fiche et retrouver l'historique.

Objectif d'accessibilité : WCAG 2.2 niveau AA et RGAA 4.1.2, avec validation HTML W3C. Une
conformité ne sera déclarée qu'après audit manuel ; les outils automatisés servent de garde-fous,
pas de preuve suffisante.

Références :

- <https://www.w3.org/TR/WCAG22/>
- <https://accessibilite.numerique.gouv.fr/>
- <https://validator.w3.org/nu/>
- <https://www.w3.org/WAI/ARIA/apg/>

## État initial — v0.1.0

### Risques critiques

1. Les routes de gestion ne vérifient pas les JWT et n'appliquent aucune autorisation par rôle ou
   par producteur. Un client peut lire ou modifier les données sans être connecté.
2. L'inscription est publique et accepte le rôle demandé, y compris `admin` : élévation de
   privilèges possible.
3. Le CORS autorise toutes les origines et toutes les routes métier sont directement exposées.
4. Plusieurs réponses renvoient les erreurs SQL internes au client.
5. Le JavaScript injecte des réponses et des données métier avec `innerHTML`, ce qui ouvre la voie
   à des XSS. Les pages publiques assemblent également du HTML depuis les données en base.
6. Le HTML rendu échoue au validateur W3C avec 174 messages. Certaines clés Jinja (`get`,
   `update`) sont résolues comme méthodes de dictionnaire et produisent des boutons cassés.

### UX et accessibilité

- Navigation organisée par routes API, pas par objectifs métier.
- Une page unique d'environ 60 ko avec 61 actions inline et 111 champs.
- Aucun élément `main`, `nav`, `form`, `fieldset`, `legend`, `h2` ou `h3` dans le rendu actuel.
- Aucun attribut ARIA, aucune région de statut et aucun champ marqué `required`.
- Le mot de passe utilise `type="text"`.
- Les champs techniques demandent des UUID et du JSON à l'utilisateur.
- Les erreurs passent par `alert()` ou par des blocs non annoncés aux lecteurs d'écran.
- Pas de thème sombre, pas de préférence système, pas de mémorisation du thème.
- Responsive limité à une seule rupture à 768 px ; aucun test documenté à 320 px ou zoom 400 %.

### Backend et données

- Authentification présente mais non branchée aux routes et non utilisée par le GUI.
- Pas de séparation multi-tenant fiable par `producer_id`.
- Refresh tokens sans rotation ni révocation ; pas de logout effectif.
- Slugs QR générés sans vérification préalable d'unicité ni stratégie de retry.
- Plusieurs parcours métier ne sont pas modélisés comme une machine à états contrôlée.
- Les données d'analytics (IP, user-agent, referer) peuvent être fournies par le client.
- Couverture limitée à 22 tests unitaires ; pas de tests d'intégration, E2E ou accessibilité.

### Exploitation

- Pas de CI GitHub.
- Images `nightly`, `latest` ou non épinglées ; pas de chaîne reproductible à long terme.
- PostgreSQL, Redis, Grafana et Prometheus publient leurs ports dans le Compose général.
- Pas de Compose de production malgré la documentation.
- Pas de procédure testée de sauvegarde/restauration ni de rollback.
- Des dépendances d'observabilité sont déclarées, mais il n'existe pas de dispositif complet
  métriques/traces/alertes.
- La documentation annonce React/PWA, 2FA, PDF, métriques et tests E2E qui ne correspondent pas à
  l'implémentation actuelle.

## Principes de conception

1. Sécurité et intégrité avant cosmétique.
2. Vocabulaire métier partout ; détails techniques réservés à un panneau diagnostic administrateur.
3. HTML natif et sémantique avant ARIA.
4. Une action principale claire par écran, avec brouillons et confirmations pour les actions
   irréversibles.
5. Mobile-first, clavier-first, lisible à 200 % et utilisable à 400 % de zoom.
6. Thèmes clair et sombre fondés sur des design tokens, avec contraste AA dans les deux thèmes.
7. Chaque lot possède des critères automatisés et une recette manuelle.

## Lots

### Lot 0 — Vérité du produit et garde-fous

- Corriger la documentation pour ne décrire que les fonctions réellement disponibles.
- Ajouter une CI : formatage, tests, Clippy sans avertissement, audit des dépendances,
  scan des secrets et construction Docker.
- Ajouter validation W3C et contrôles d'accessibilité automatisés sur le HTML rendu.
- Documenter les environnements développement, test et production.

**Sortie :** CI verte sur un clone neuf ; aucune promesse fictive dans le README ; dette connue
consignée.

### Lot 1 — Sécurité et cloisonnement

- Middleware JWT pour toutes les routes privées.
- RBAC explicite et filtrage systématique par producteur.
- Inscription initiale contrôlée, invitations pour les utilisateurs suivants, aucun rôle choisi par
  le client.
- Rotation/révocation des refresh tokens et logout réel.
- CORS par liste blanche, limites de taille, rate limiting, timeouts et en-têtes de sécurité.
- Échapper toutes les données injectées dans le HTML ; supprimer les constructions XSS.
- Réponses d'erreur publiques neutres avec identifiant de corrélation, détails uniquement en logs.
- Tests d'autorisation et d'isolation multi-tenant.

**Sortie :** aucune route métier anonyme ; matrice de permissions testée ; aucune XSS connue ; les
erreurs internes ne quittent pas le serveur.

### Lot 2 — Architecture de l'information métier

Navigation cible :

- Tableau de bord
- Produits et recettes
- Production et lots
- Contrôles qualité
- Traçabilité et QR codes
- Fournisseurs et ingrédients
- Équipe et paramètres

Les listes mènent à des fiches et les fiches à des actions. Les relations sont choisies par nom,
recherche ou autocomplétion, jamais par UUID. La création d'un lot devient un assistant court :
produit, quantités/date, contrôles, récapitulatif.

**Sortie :** cinq scénarios métier principaux réalisables sans voir une route, un UUID ou du JSON.

### Lot 3 — Design system, RWD, thèmes et accessibilité

- Design tokens CSS pour couleurs, espacements, typographie, rayons, ombres et états.
- Mobile-first de 320 px aux grands écrans, sans défilement horizontal à 400 % de zoom.
- Structure sémantique (`header`, `nav`, `main`, titres ordonnés, vrais formulaires et fieldsets).
- Labels, aides, erreurs liées aux champs, résumé d'erreurs et régions `aria-live`.
- Focus visible et non masqué, cibles d'au moins 24 × 24 px, ordre clavier logique.
- Contrastes AA, prise en charge de `prefers-reduced-motion` et du contraste forcé.
- Thème `auto | clair | sombre`, préférence système par défaut et choix mémorisé localement.
- Suppression des gestionnaires inline et des styles inline.

**Sortie :** HTML W3C sans erreur ; axe sans violation critique/sérieuse ; parcours clavier complet ;
contrastes vérifiés dans les deux thèmes ; recette lecteur d'écran documentée.

### Lot 4 — Fiabilité des parcours métier

- Transactions pour chaque modification composée.
- Machine à états des lots avec transitions et permissions explicites.
- Validation métier partagée côté serveur et messages compréhensibles côté interface.
- Idempotence des créations sensibles et retry contrôlé des identifiants QR.
- Journal d'audit des modifications, rappels et publications.
- Gestion robuste des pièces jointes, types, tailles, antivirus et stockage.

**Sortie :** aucun état partiel après erreur ; historique attribuable ; scénarios de concurrence
testés.

### Lot 5 — Stratégie de tests

- Tests unitaires du domaine.
- Tests PostgreSQL/Redis des migrations, contraintes et autorisations.
- Tests contractuels API.
- E2E des parcours métier avec Playwright.
- Tests accessibilité axe + tests manuels RGAA/WCAG.
- Tests responsive et régression visuelle des thèmes.
- Test de sauvegarde/restauration et test de migration depuis la version précédente.

**Sortie :** matrice de tests publiée ; parcours critiques bloquants en CI ; restauration démontrée.

### Lot 6 — Exploitation production

- Rust stable épinglé et images versionnées par digest.
- Images minimales, health/readiness checks, arrêt gracieux et limites de ressources.
- Compose local séparé du déploiement de production ; aucun port de données exposé publiquement.
- TLS, politique CSP, proxy durci et gestion externe des secrets.
- Logs structurés sans données sensibles, métriques utiles, traces et alertes actionnables.
- Sauvegardes chiffrées, rétention, restauration et procédure d'incident.
- SBOM, scan de conteneur, audit RustSec et mise à jour automatisée des dépendances.

**Sortie :** déploiement reproductible en préproduction, supervision effective et runbook testé.

### Lot 7 — Validation avec les utilisateurs

- Tests avec le destinataire réel sur les cinq tâches principales.
- Mesurer réussite sans aide, erreurs, temps et points d'hésitation.
- Corriger le vocabulaire et les étapes avant de polir les détails visuels.
- Audit d'accessibilité manuel indépendant avant toute déclaration de conformité.

**Sortie :** tâches critiques accomplies sans aide par les utilisateurs pilotes ; écarts
d'accessibilité documentés et corrigés ou publiés honnêtement.

## Ordre de livraison recommandé

1. Lots 0 et 1 : prérequis absolus.
2. Lots 2 et 3 : nouvelle expérience utilisable et accessible.
3. Lots 4 et 5 : fiabilité démontrée.
4. Lots 6 et 7 : préproduction, pilote, puis version candidate.

Jalons proposés :

- `v0.2.0-alpha` : sécurité et socle CI.
- `v0.3.0-beta` : nouvelle UX complète et accessible.
- `v0.9.0-rc.1` : exploitation et pilote validés.
- `v1.0.0` : production après audit et restauration testée.
