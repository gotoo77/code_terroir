# Code Terroir GUI - Interface Web

## Structure

La GUI est organisée en 3 fichiers séparés pour une meilleure maintenabilité :

### `index.html`
Structure HTML de l'interface de test.
- Contient la structure DOM des onglets et endpoints
- Injecte la variable `API_BASE` via Jinja2 depuis le serveur Python
- Charge le CSS et le JavaScript externes

### `style.css`
Feuille de styles complète.
- Mise en page responsive
- Thème colors (bleu/gris)
- Styles pour les cartes endpoint, formulaires, réponses

### `app.js`
Logique applicative JavaScript.
- Navigation entre onglets
- Requêtes AJAX vers l'API
- Gestion des formulaires
- Affichage des réponses

## Notes de développement

- Les fichiers CSS et JS ne subissent **pas** de templating Jinja2
- Seul `index.html` est traité par Jinja2 pour injecter `{{ api_base }}`
- `API_BASE` est défini en JavaScript inline dans le HTML avant de charger `app.js`
- Chemins relatifs : CSS et JS se chargent depuis le même répertoire

## Maintenance

Pour modifier l'interface :
1. Ajouter des styles → `style.css`
2. Ajouter des fonctions JS → `app.js`
3. Ajouter de la structure HTML → `index.html`

Le serveur Python (`serve_gui.py`) charge automatiquement et rend les templates.
