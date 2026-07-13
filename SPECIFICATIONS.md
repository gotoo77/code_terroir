# 📦 Spécification — Projet Code Terroir (V1 enrichie)

## 1. 🧱 Modèle de données

### 🔹 Entité `Produit`
Champs :
- id
- nom
- description
- image_url (nouveau)
- nutriscore (A → E)
- energie_kcal_100g
- glucides_100g
- lipides_100g
- proteines_100g
- allergenes
- date_creation

---

### 🔹 Entité `Producteur`
Champs :
- id
- nom
- photo_url
- telephone
- email
- categorie_principale

---

### 🔹 Entité `Ingredient`
Champs :
- id
- nom
- categorie

---

### 🔹 Relation Produit ↔ Producteur

Produit
→ Produit_Ingredient
→ Ingredient
→ Producteur

Table Produit_Ingredient :
- produit_id
- ingredient_id
- producteur_id
- quantite

---

## 2. 🖥️ Interface Back-office

### Saisie d’un lot

- Sélection catégorie (viande, légumes, assaisonnement)
- Filtrage dynamique ingrédients + producteurs
- Sélection ingrédient
- Sélection producteur
- Bouton auto-remplissage intelligent

---

## 3. 🌐 Interface publique

### Accès sécurisé
- QR code unique
- Token sécurisé
- Accès limité au produit scanné

---

### Fiche produit

#### Infos générales
- Nom
- Image
- Description

#### Producteurs impliqués
Tableau :
Ingrédient | Catégorie | Producteur | Contact

#### Nutrition

Composition nutritionnelle

Valeurs pour 100g :
- Énergie
- Glucides
- Lipides
- Protéines

Répartition :
- Glucides : 50%
- Lipides : 30%
- Protéines : 20%

#### Nutriscore
A → E

#### Allergènes
Liste visible

---

## 4. 🎨 Identité visuelle

Apothicaire Culinaire

Slogan :
"La transparence du champ à l’assiette"

---

## 5. 🔐 Sécurité

QR code :
- produit_id
- token sécurisé

Backend :
- vérification token
- accès restreint

---

## 6. 🧠 Architecture

Backend :
- API REST
- JSON pour allergènes

Frontend :
- HTML + JS dynamique

---

## 7. 🚀 Évolutions

- Upload images
- Historique lots
- Analytics QR
- Notation produits
- API publique
