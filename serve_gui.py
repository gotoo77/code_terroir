#!/usr/bin/env python3
"""Serve the local GUI used to exercise the Code Terroir API."""

import http.server
import os
import socketserver
import sys
from pathlib import Path

from jinja2 import Environment, FileSystemLoader, select_autoescape


BASE_DIR = Path(__file__).parent
WEB_DIR = BASE_DIR / "web" / "gui"
HOST = "localhost"
PORT = 8081

os.chdir(WEB_DIR)

env = Environment(
    loader=FileSystemLoader(WEB_DIR),
    autoescape=select_autoescape(enabled_extensions=("html", "xml")),
)


def get_template_context():
    """Build the template context injected into the GUI."""
    return {
        "title": "Code Terroir API - Interface de Test",
        "header_title": "🥩 Code Terroir API",
        "header_subtitle": "Apothicaire Culinaire · interface de test pour la tracabilite artisanale",
        "api_base": f"http://{HOST}:3030",
        "tabs": {
            "health": "🏥 Health",
            "auth": "🔐 Auth",
            "producers": "🧑‍🌾 Producteurs",
            "products": "📦 Produits",
            "batches": "🏷️ Lots",
            "qa": "🧪 QA",
            "suppliers": "🏭 Fournisseurs",
            "ingredients": "🥘 Ingredients",
            "recipes": "📚 Recettes",
            "qr": "📱 QR Codes",
            "public": "🌍 Public",
        },
        "health": {
            "title": "API de sante",
            "description": "Verifiez que l'API Code Terroir fonctionne correctement",
        },
        "auth": {
            "title": "Authentification",
            "description": "Tester les endpoints auth actuellement exposes",
            "login_desc": "Tester le endpoint login placeholder",
            "register_desc": "Tester le endpoint register placeholder",
            "refresh_desc": "Tester le endpoint refresh placeholder",
            "form_labels": {
                "producer_id": "ID producteur:",
                "email": "Email:",
                "password": "Mot de passe:",
                "first_name": "Prenom:",
                "last_name": "Nom:",
                "role": "Role:",
                "refresh_token": "Refresh token:",
            },
            "form_placeholders": {
                "producer_id": "ex: 550e8400-e29b-41d4-a716-446655440001",
                "email": "ex: atelier@code-terroir.fr",
                "password": "motdepasse-test",
                "first_name": "Camille",
                "last_name": "Martin",
                "refresh_token": "token-test",
            },
            "roles": ["admin", "quality", "atelier", "logistics", "readonly"],
        },
        "producers": {
            "title": "Producteurs",
            "description": "Gerer les producteurs et leur fiche visible",
            "list_desc": "Recuperer la liste de tous les producteurs",
            "get_desc": "Recuperer un producteur specifique par son ID",
            "create_desc": "Creer un nouveau producteur",
            "update_desc": "Mettre a jour un producteur",
            "form_labels": {
                "producer_id": "ID producteur (UUID):",
                "raison_sociale": "Raison sociale*:",
                "agrement": "Agrement sanitaire:",
                "siret": "SIRET:",
                "adresse": "Adresse*:",
                "code_postal": "Code postal*:",
                "ville": "Ville*:",
                "pays": "Pays*:",
                "email": "Email*:",
                "telephone": "Telephone:",
                "site_web": "Site web:",
                "logo_url": "Logo URL:",
                "photo_url": "Photo URL:",
                "categorie_principale": "Categorie principale:",
            },
            "form_placeholders": {
                "producer_id": "ex: 550e8400-e29b-41d4-a716-446655440001",
                "raison_sociale": "ex: Apothicaire Culinaire",
                "agrement": "FR-47-000-EC",
                "siret": "12345678901234",
                "adresse": "12 rue des producteurs",
                "code_postal": "47000",
                "ville": "Agen",
                "pays": "France",
                "email": "contact@apothicaire.example.com",
                "telephone": "+33 5 12 34 56 78",
                "site_web": "https://apothicaire.example.com",
                "logo_url": "https://example.com/logo.png",
                "photo_url": "https://example.com/photo-atelier.jpg",
                "categorie_principale": "legumes",
            },
            "categories": [
                "legumes",
                "fruits",
                "assaisonnement",
                "epices",
                "sel",
                "poivres",
                "paprika",
                "viandes",
                "poissons",
                "laitages",
                "autre",
            ],
        },
        "products": {
            "title": "Produits",
            "description": "Gerer le catalogue des produits artisanaux",
            "list_desc": "Recuperer la liste de tous les produits",
            "get_desc": "Recuperer un produit specifique par son ID",
            "create_desc": "Creer un nouveau produit",
            "update_desc": "Mettre a jour un produit existant",
            "examples": {
                "demo_product_id": "550e8400-e29b-41d4-a716-446655440100",
                "demo_product_name": "Veloute des Maraichers",
            },
            "form_labels": {
                "id": "ID du produit*:",
                "producer_id": "ID du producteur:",
                "name": "Nom du produit*:",
                "category": "Categorie*:",
                "description": "Description marketing:",
                "image_url": "Image principale (URL):",
                "nutriscore": "Nutriscore:",
                "macros": "Valeurs nutritionnelles pour 100 g:",
                "allergens": "Allergenes UE:",
                "product_ingredients": "Composition produit (JSON):",
                "labels": "Labels et certifications (separes par des virgules):",
                "usage": "Conseils d'utilisation:",
                "legal": "Informations legales:",
            },
            "form_placeholders": {
                "producer_id": "ex: 550e8400-e29b-41d4-a716-446655440001",
                "name": "ex: Rillettes de canard aux figues",
                "description": "Description commerciale du produit...",
                "image_url": "https://example.com/visuels/produit.jpg",
                "product_ingredients": '[{\"ingredient_id\":\"550e8400-e29b-41d4-a716-446655440020\",\"producer_id\":\"550e8400-e29b-41d4-a716-446655440001\",\"quantity\":62.5,\"unit\":\"%\",\"ingredient_category\":\"legumes\"}]',
                "labels": "ex: Bio, IGP Sud-Ouest, Fermier",
                "usage": "Comment utiliser/servir le produit...",
                "legal": "Mentions legales, allergenes, etc...",
            },
            "categories": [
                "conserve",
                "confiture",
                "legume_appertise",
                "sauce",
                "condiment",
                "charcuterie",
                "fromage",
                "boisson",
                "boulangerie",
                "patisserie",
                "autre",
            ],
            "quick_fill": "🚀 Test rapide:",
            "nutriscores": ["A", "B", "C", "D", "E"],
            "allergen_options": [
                {"value": "gluten", "label": "Cereales contenant du gluten"},
                {"value": "crustaces", "label": "Crustaces"},
                {"value": "oeufs", "label": "Oeufs"},
                {"value": "poissons", "label": "Poissons"},
                {"value": "arachides", "label": "Arachides"},
                {"value": "soja", "label": "Soja"},
                {"value": "lait", "label": "Lait"},
                {"value": "fruits-a-coque", "label": "Fruits a coque"},
                {"value": "celeri", "label": "Celeri"},
                {"value": "moutarde", "label": "Moutarde"},
                {"value": "sesame", "label": "Graines de sesame"},
                {"value": "sulfites", "label": "Sulfites"},
                {"value": "lupin", "label": "Lupin"},
                {"value": "mollusques", "label": "Mollusques"},
            ],
            "example_product": {
                "producer_id": "550e8400-e29b-41d4-a716-446655440001",
                "name": "Veloute des Maraichers",
                "category": "sauce",
                "description": "Une recette douce et vive, cuisinee en petite serie avec les legumes de saison.",
                "image_url": "https://images.unsplash.com/photo-1547592180-85f173990554?auto=format&fit=crop&w=900&q=80",
                "nutriscore": "B",
                "energie_kcal_100g": 118,
                "glucides_100g": 12,
                "lipides_100g": 6,
                "proteines_100g": 4.8,
                "labels": "Artisanal, Lot-et-Garonne",
                "usage": "Servir chaud avec quelques graines torrefiees.",
                "legal": "Valeurs nutritionnelles pour 100 g. Conserver au frais apres ouverture.",
                "product_ingredients": '[{\"ingredient_id\":\"550e8400-e29b-41d4-a716-446655440020\",\"producer_id\":\"550e8400-e29b-41d4-a716-446655440001\",\"quantity\":70,\"unit\":\"%\",\"ingredient_category\":\"legumes\"},{\"ingredient_id\":\"550e8400-e29b-41d4-a716-446655440022\",\"producer_id\":\"550e8400-e29b-41d4-a716-446655440001\",\"quantity\":20,\"unit\":\"%\",\"ingredient_category\":\"laitages\"},{\"ingredient_id\":\"550e8400-e29b-41d4-a716-446655440021\",\"producer_id\":\"550e8400-e29b-41d4-a716-446655440002\",\"quantity\":10,\"unit\":\"%\",\"ingredient_category\":\"assaisonnement\"}]',
            },
        },
        "batches": {
            "title": "Lots de production",
            "description": "Creation et gestion des lots pour tracabilite alimentaire",
            "list_desc": "Recuperer la liste de tous les lots de production",
            "get_desc": "Recuperer un lot specifique par son ID",
            "create_desc": "Creer un nouveau lot de production",
            "update_desc": "Mettre a jour un lot existant",
            "recall_desc": "Rappeler un lot",
            "form_labels": {
                "batch_id": "ID du lot*:",
                "product_id": "ID du produit*:",
                "lot_code": "Code de lot (optionnel - auto-genere):",
                "dluo": "Date limite d'utilisation*:",
                "quantity": "Quantite produite*:",
                "site": "Site de production*:",
                "operators": "Operateurs (separes par virgules):",
                "notes": "Notes de production:",
                "recall_id": "ID du lot a rappeler*:",
                "recall_reason": "Raison du rappel*:",
            },
            "form_placeholders": {
                "product_id": "ex: 550e8400-e29b-41d4-a716-446655440004",
                "lot_code": "ex: LOT-2025-09-18-A",
                "quantity": "ex: 240",
                "site": "ex: Atelier Sud, Zone A",
                "operators": "ex: Jean Dupont, Marie Martin",
                "notes": "Notes speciales, observations...",
                "recall_reason": "Decrire la raison du rappel...",
            },
            "example_batch": {
                "product_id": "550e8400-e29b-41d4-a716-446655440100",
                "quantity": "240",
                "site": "Atelier Principal - Zone A",
                "operators": "Jean Dupont, Marie Martin",
                "notes": "Lot de production automne 2025.",
            },
        },
        "qa": {
            "title": "Controles qualite",
            "description": "Saisir et consulter les controles qualite des lots",
            "list_desc": "Recuperer tous les controles qualite",
            "get_desc": "Recuperer un controle qualite par son ID",
            "batch_list_desc": "Recuperer les controles qualite d'un lot",
            "create_desc": "Ajouter un controle qualite a un lot",
            "summary_desc": "Recuperer le resume qualite d'un lot",
            "form_labels": {
                "qa_id": "ID controle QA:",
                "batch_id": "ID lot*:",
                "operator_id": "ID operateur*:",
                "check_type": "Type de controle*:",
                "value": "Valeur mesuree:",
                "min_threshold": "Seuil min:",
                "max_threshold": "Seuil max:",
                "unit": "Unite:",
                "notes": "Notes:",
                "attachments": "Pieces jointes (URLs separees par des virgules):",
            },
            "form_placeholders": {
                "qa_id": "ex: 550e8400-e29b-41d4-a716-446655440040",
                "batch_id": "ex: 550e8400-e29b-41d4-a716-446655440002",
                "operator_id": "ex: 550e8400-e29b-41d4-a716-446655440100",
                "value": "ex: 4.2",
                "min_threshold": "ex: 3.8",
                "max_threshold": "ex: 4.6",
                "unit": "ex: pH",
                "notes": "Controle conforme apres cuisson",
                "attachments": "https://example.com/preuve.jpg",
            },
            "types": ["ph", "temperature", "sealed", "visual", "microbiological", "weight"],
        },
        "suppliers": {
            "title": "Fournisseurs",
            "description": "Gerer les fournisseurs d'ingredients",
            "list_desc": "Recuperer la liste de tous les fournisseurs",
            "get_desc": "Recuperer un fournisseur specifique par son ID",
            "create_desc": "Creer un nouveau fournisseur",
            "update_desc": "Mettre a jour un fournisseur",
            "update_desc": "Mettre a jour un fournisseur",
            "form_labels": {
                "supplier_id": "ID du fournisseur (UUID):",
                "producer_id": "ID du producteur*:",
                "name": "Nom du fournisseur*:",
                "email": "Email*:",
                "phone": "Telephone*:",
                "address": "Adresse*:",
            },
            "form_placeholders": {
                "supplier_id": "ex: 550e8400-e29b-41d4-a716-446655440010",
                "producer_id": "ex: 550e8400-e29b-41d4-a716-446655440001",
                "name": "ex: Ferme du Soleil Levant",
                "email": "ex: contact@ferme-soleil.fr",
                "phone": "ex: +33 5 12 34 56 78",
                "address": "ex: 123 Rue de la Campagne, 47000 Agen",
            },
            "example_supplier": {
                "producer_id": "550e8400-e29b-41d4-a716-446655440001",
                "name": "Ferme du Soleil Levant",
                "email": "contact@ferme-soleil.fr",
                "phone": "+33 5 12 34 56 78",
                "address": "123 Rue de la Campagne, 47000 Agen",
            },
        },
        "ingredients": {
            "title": "Ingredients",
            "description": "Gerer l'inventaire des ingredients",
            "list_desc": "Recuperer la liste de tous les ingredients",
            "get_desc": "Recuperer un ingredient specifique par son ID",
            "create_desc": "Creer un nouvel ingredient",
            "update_desc": "Mettre a jour un ingredient",
            "form_labels": {
                "ingredient_id": "ID de l'ingredient (UUID):",
                "producer_id": "ID du producteur*:",
                "supplier_id": "ID du fournisseur (optionnel):",
                "name": "Nom de l'ingredient*:",
                "category": "Categorie ingredient:",
                "allergens": "Allergenes UE:",
                "nutrition": "Infos nutritionnelles (JSON):",
                "documents": "Documents (URLs separees par des virgules):",
            },
            "form_placeholders": {
                "ingredient_id": "ex: 550e8400-e29b-41d4-a716-446655440020",
                "producer_id": "ex: 550e8400-e29b-41d4-a716-446655440001",
                "supplier_id": "ex: 550e8400-e29b-41d4-a716-446655440010",
                "name": "ex: Fraises bio",
                "category": "ex: legumes",
                "nutrition": 'ex: {"energie_kcal": 35, "per_100g": true}',
                "documents": "ex: https://.../fiche-technique.pdf",
            },
            "categories": [
                "legumes",
                "fruits",
                "assaisonnement",
                "epices",
                "sel",
                "poivres",
                "paprika",
                "viandes",
                "poissons",
                "laitages",
                "autre",
            ],
            "allergen_options": [
                {"value": "gluten", "label": "Cereales contenant du gluten"},
                {"value": "crustaces", "label": "Crustaces"},
                {"value": "oeufs", "label": "Oeufs"},
                {"value": "poissons", "label": "Poissons"},
                {"value": "arachides", "label": "Arachides"},
                {"value": "soja", "label": "Soja"},
                {"value": "lait", "label": "Lait"},
                {"value": "fruits-a-coque", "label": "Fruits a coque"},
                {"value": "celeri", "label": "Celeri"},
                {"value": "moutarde", "label": "Moutarde"},
                {"value": "sesame", "label": "Graines de sesame"},
                {"value": "sulfites", "label": "Sulfites"},
                {"value": "lupin", "label": "Lupin"},
                {"value": "mollusques", "label": "Mollusques"},
            ],
            "example_ingredient": {
                "producer_id": "550e8400-e29b-41d4-a716-446655440001",
                "supplier_id": "550e8400-e29b-41d4-a716-446655440010",
                "name": "Carottes des Landes",
                "category": "legumes",
                "nutrition": '{"energie_kcal": 41, "glucides": 9.6, "lipides": 0.2, "proteines": 0.9, "per_100g": true}',
                "documents": "https://example.com/docs/carottes.pdf",
            },
        },
        "recipes": {
            "title": "Recettes",
            "description": "Gerer les recettes et leurs versions",
            "list_desc": "Recuperer la liste des recettes",
            "get_desc": "Recuperer une recette par son ID",
            "create_desc": "Creer une nouvelle recette",
            "update_desc": "Mettre a jour une recette ou creer une nouvelle version",
            "form_labels": {
                "recipe_id": "ID recette (UUID):",
                "producer_id": "ID producteur*:",
                "name": "Nom de recette*:",
                "ingredients": "Ingredients (JSON)*:",
                "steps": "Etapes (JSON)*:",
                "notes": "Notes:",
                "new_version": "Creer une nouvelle version",
            },
            "form_placeholders": {
                "recipe_id": "ex: 550e8400-e29b-41d4-a716-446655440030",
                "producer_id": "ex: 550e8400-e29b-41d4-a716-446655440001",
                "name": "ex: Cassoulet de Rico",
                "ingredients": '[{"ingredient_id":"550e8400-e29b-41d4-a716-446655440020","nom":"Haricots","quantite":1.2,"unite":"kg","fournisseur_id":null,"fournisseur_nom":null,"ordre":1}]',
                "steps": '[{"etape":1,"nom":"Trempage","description":"Laisser tremper une nuit","duree_min":720,"temperature_c":null,"equipement":null,"controles":["visuel"]}]',
                "notes": "Version atelier printemps",
            },
        },
        "qr": {
            "title": "Codes QR",
            "description": "Generation et suivi des QR codes pour tracabilite",
            "list_desc": "Recuperer la liste de tous les QR codes generes",
            "get_desc": "Recuperer un QR code specifique par son ID",
            "create_desc": "Creer un nouveau QR code pour un lot de production",
            "scan_desc": "Simuler le scan d'un QR code par son slug",
            "analytics_desc": "Recuperer les statistiques de scan d'un QR code",
            "form_labels": {
                "qr_id": "ID du QR code (UUID):",
                "batch_id": "ID du lot de production*:",
                "format": "Format du QR code:",
                "slug": "Slug du QR code*:",
                "ip": "Adresse IP (optionnel):",
                "user_agent": "User Agent (optionnel):",
                "analytics_id": "ID du QR code:",
            },
            "form_placeholders": {
                "qr_id": "ex: 550e8400-e29b-41d4-a716-446655440005",
                "batch_id": "ex: 550e8400-e29b-41d4-a716-446655440002",
                "slug": "ex: x7k3m9",
                "ip": "ex: 192.168.1.100",
                "user_agent": "ex: Mozilla/5.0...",
                "analytics_id": "ex: 550e8400-e29b-41d4-a716-446655440005",
            },
            "formats": ["Both", "SVG", "PNG"],
            "example_batch_id": "550e8400-e29b-41d4-a716-446655440200",
        },
        "public": {
            "title": "Pages publiques",
            "description": "API publique pour les consommateurs",
            "trace_desc": "Recuperer les informations publiques d'un QR slug",
            "visit_desc": "Enregistrer une visite publique",
            "html_desc": "Ouvrir la page HTML publique liee au slug",
            "form_labels": {
                "slug": "Slug QR*:",
                "ip": "Adresse IP:",
                "user_agent": "User Agent:",
                "referer": "Referer:",
            },
            "form_placeholders": {
                "slug": "ex: x7k3m9",
                "ip": "ex: 192.168.1.25",
                "user_agent": "Mozilla/5.0",
                "referer": "https://apothicaire.example.com",
            },
        },
        "common": {
            "required_field": "*",
            "optional": "(optionnel)",
            "quick_fill": "🚀 Exemple:",
            "buttons": {
                "test": "Tester",
                "list": "Lister",
                "get": "Recuperer",
                "get_batch": "Recuperer le lot",
                "get_qr": "Recuperer le QR code",
                "get_qa": "Recuperer le controle",
                "get_public": "Recuperer la fiche",
                "create": "Creer",
                "update": "Mettre a jour",
                "login": "Tester login",
                "register": "Tester register",
                "refresh": "Tester refresh",
                "open_page": "Ouvrir la page",
                "create_batch": "Creer le lot",
                "create_qr": "Creer le QR code",
                "create_qa": "Ajouter le controle",
                "recall": "Rappeler le lot",
                "scan": "Enregistrer le scan",
                "analytics": "Voir les statistiques",
                "fill_example": "Remplir avec exemple",
                "use_example": "Utiliser ID de lot exemple",
                "use_id": "Utiliser",
            },
            "responses": {
                "header": "Reponse",
                "loading": "Chargement...",
                "creating": "Creation en cours...",
                "scanning": "Enregistrement du scan...",
                "recalling": "Rappel du lot en cours...",
                "fetching_stats": "Chargement des statistiques...",
            },
            "errors": {
                "connect_error": "Erreur de connexion",
                "product_id_required": "Veuillez saisir un ID de produit",
                "batch_id_required": "Veuillez saisir un ID de lot",
                "qr_id_required": "Veuillez saisir un ID de QR code",
                "slug_required": "Le slug du QR code est obligatoire",
                "product_name_required": "Le nom et la categorie du produit sont obligatoires",
                "batch_fields_required": "L'ID du produit, la DLUO, la quantite et le site sont obligatoires",
                "batch_id_reason_required": "L'ID du lot et la raison du rappel sont obligatoires",
                "qr_batch_required": "L'ID du lot de production est obligatoire",
            },
        },
    }


class ReusableTCPServer(socketserver.TCPServer):
    allow_reuse_address = True


class CORSHTTPRequestHandler(http.server.SimpleHTTPRequestHandler):
    """HTTP handler with CORS support."""

    def end_headers(self):
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type, Authorization")
        super().end_headers()

    def do_GET(self):
        if self.path in ["/", "/index.html"]:
            try:
                template = env.get_template("index.html")
                content = template.render(**get_template_context())
                self.send_response(200)
                self.send_header("Content-Type", "text/html; charset=utf-8")
                self.end_headers()
                self.wfile.write(content.encode("utf-8"))
                return
            except Exception as exc:
                self.send_error(500, f"Error: {exc}")
                return

        super().do_GET()

    def do_OPTIONS(self):
        self.send_response(200)
        self.end_headers()


if __name__ == "__main__":
    try:
        with ReusableTCPServer(("", PORT), CORSHTTPRequestHandler) as httpd:
            print("Code Terroir API Tester GUI")
            print(f"http://{HOST}:{PORT}")
            print(f"Serving from: {WEB_DIR}")
            print(f"API Base: http://{HOST}:3030")
            httpd.serve_forever()
    except KeyboardInterrupt:
        print("\nServer stopped")
        sys.exit(0)
    except Exception as exc:
        print(f"Error: {exc}")
        sys.exit(1)
