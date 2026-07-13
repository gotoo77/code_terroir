-- Jeu de données de démo Code Terroir / Apothicaire Culinaire
-- Mot de passe utilisateur démo: demo12345

BEGIN;

-- Nettoyage ciblé
DELETE FROM qr_scans WHERE batch_id IN (
    '550e8400-e29b-41d4-a716-446655440200'
);
DELETE FROM qa_checks WHERE batch_id IN (
    '550e8400-e29b-41d4-a716-446655440200'
);
DELETE FROM qr_tags WHERE id IN (
    '550e8400-e29b-41d4-a716-446655440300'
);
DELETE FROM batches WHERE id IN (
    '550e8400-e29b-41d4-a716-446655440200'
);
DELETE FROM product_ingredients WHERE product_id IN (
    '550e8400-e29b-41d4-a716-446655440100'
);
DELETE FROM products WHERE id IN (
    '550e8400-e29b-41d4-a716-446655440100'
);
DELETE FROM recipes WHERE id IN (
    '550e8400-e29b-41d4-a716-446655440090'
);
DELETE FROM ingredients WHERE id IN (
    '550e8400-e29b-41d4-a716-446655440020',
    '550e8400-e29b-41d4-a716-446655440021',
    '550e8400-e29b-41d4-a716-446655440022'
);
DELETE FROM suppliers WHERE id IN (
    '550e8400-e29b-41d4-a716-446655440010'
);
DELETE FROM users WHERE id IN (
    '550e8400-e29b-41d4-a716-446655440050'
);
DELETE FROM producers WHERE id IN (
    '550e8400-e29b-41d4-a716-446655440001',
    '550e8400-e29b-41d4-a716-446655440002'
);

INSERT INTO producers (
    id, raison_sociale, agrement_sanitaire, siret, adresse, code_postal, ville, pays,
    email, telephone, site_web, logo_url, photo_url, categorie_principale
) VALUES
(
    '550e8400-e29b-41d4-a716-446655440001',
    'Apothicaire Culinaire',
    'FR-47-000-EC',
    '12345678901234',
    '12 rue des Producteurs',
    '47000',
    'Agen',
    'France',
    'contact@apothicaire.example.com',
    '+33 5 12 34 56 78',
    'https://apothicaire.example.com',
    'https://images.unsplash.com/photo-1514933651103-005eec06c04b?auto=format&fit=crop&w=800&q=80',
    'https://images.unsplash.com/photo-1556910103-1c02745aae4d?auto=format&fit=crop&w=800&q=80',
    'legumes'
),
(
    '550e8400-e29b-41d4-a716-446655440002',
    'Ferme du Soleil Levant',
    NULL,
    '98765432109876',
    '7 route des Maraîchers',
    '47310',
    'Estillac',
    'France',
    'bonjour@soleil-levant.example.com',
    '+33 5 53 10 20 30',
    'https://soleil-levant.example.com',
    NULL,
    'https://images.unsplash.com/photo-1501004318641-b39e6451bec6?auto=format&fit=crop&w=800&q=80',
    'assaisonnement'
);

INSERT INTO users (
    id, producer_id, email, password_hash, first_name, last_name, role, is_active,
    last_login, totp_secret, totp_enabled
) VALUES (
    '550e8400-e29b-41d4-a716-446655440050',
    '550e8400-e29b-41d4-a716-446655440001',
    'atelier@apothicaire.example.com',
    '$argon2id$v=19$m=19456,t=2,p=1$PhSb+W1pacbqSaNFyA59PA$wcdoVa9xcFyOmCgg/sLcmOEjMPAn03Mo9y/pg6rGt6c',
    'Camille',
    'Martin',
    'atelier',
    true,
    NOW(),
    NULL,
    false
);

INSERT INTO suppliers (
    id, producer_id, name, contact_email, contact_phone, address, certifications, documents
) VALUES (
    '550e8400-e29b-41d4-a716-446655440010',
    '550e8400-e29b-41d4-a716-446655440001',
    'Ferme du Soleil Levant',
    'bonjour@soleil-levant.example.com',
    '+33 5 53 10 20 30',
    '7 route des Maraîchers, 47310 Estillac',
    '{"bio": true, "origine": "Lot-et-Garonne"}'::jsonb,
    ARRAY['https://example.com/docs/soleil-levant.pdf']
);

INSERT INTO ingredients (
    id, producer_id, name, category, allergens, nutritional_info, supplier_id, documents
) VALUES
(
    '550e8400-e29b-41d4-a716-446655440020',
    '550e8400-e29b-41d4-a716-446655440001',
    'Carottes des Landes',
    'legumes',
    ARRAY[]::text[],
    '{"energie_kcal": 41, "glucides": 9.6, "lipides": 0.2, "proteines": 0.9, "per_100g": true}'::jsonb,
    '550e8400-e29b-41d4-a716-446655440010',
    ARRAY['https://example.com/docs/carottes.pdf']
),
(
    '550e8400-e29b-41d4-a716-446655440021',
    '550e8400-e29b-41d4-a716-446655440002',
    'Poivre fumé',
    'assaisonnement',
    ARRAY['moutarde']::text[],
    '{"energie_kcal": 251, "glucides": 64.0, "lipides": 3.3, "proteines": 10.4, "per_100g": true}'::jsonb,
    '550e8400-e29b-41d4-a716-446655440010',
    ARRAY['https://example.com/docs/poivre.pdf']
),
(
    '550e8400-e29b-41d4-a716-446655440022',
    '550e8400-e29b-41d4-a716-446655440001',
    'Crème légère',
    'laitages',
    ARRAY['lait']::text[],
    '{"energie_kcal": 193, "glucides": 4.0, "lipides": 18.0, "proteines": 2.5, "per_100g": true}'::jsonb,
    NULL,
    ARRAY['https://example.com/docs/creme.pdf']
);

INSERT INTO recipes (
    id, producer_id, name, version, ingredients, steps, notes, is_active, ingredients_detail, etapes_detail
) VALUES (
    '550e8400-e29b-41d4-a716-446655440090',
    '550e8400-e29b-41d4-a716-446655440001',
    'Veloute des Maraichers',
    1,
    '[{"ingredient_id":"550e8400-e29b-41d4-a716-446655440020","quantity":70,"unit":"%"},{"ingredient_id":"550e8400-e29b-41d4-a716-446655440022","quantity":20,"unit":"%"},{"ingredient_id":"550e8400-e29b-41d4-a716-446655440021","quantity":10,"unit":"%"}]'::jsonb,
    '[{"step":1,"description":"Laver et couper les legumes","duration_min":20},{"step":2,"description":"Cuire puis mixer avec la creme","duration_min":45}]'::jsonb,
    'Recette de démonstration GUI',
    true,
    '[{"ingredient_id":"550e8400-e29b-41d4-a716-446655440020","nom":"Carottes des Landes","quantite":70,"unite":"%"},{"ingredient_id":"550e8400-e29b-41d4-a716-446655440022","nom":"Crème légère","quantite":20,"unite":"%"},{"ingredient_id":"550e8400-e29b-41d4-a716-446655440021","nom":"Poivre fumé","quantite":10,"unite":"%"}]'::jsonb,
    '[{"etape":1,"nom":"Préparation","description":"Préparer les légumes","duree_min":20},{"etape":2,"nom":"Cuisson","description":"Cuire puis mixer","duree_min":45}]'::jsonb
);

INSERT INTO products (
    id, producer_id, name, category, recipe_id, description_marketing, ingredients, nutritional_values,
    image_url, nutriscore, energie_kcal_100g, glucides_100g, lipides_100g, proteines_100g, allergenes,
    labels_certifications, visuels, conseils_utilisation, infos_legales
) VALUES (
    '550e8400-e29b-41d4-a716-446655440100',
    '550e8400-e29b-41d4-a716-446655440001',
    'Velouté des Maraîchers',
    'sauce',
    '550e8400-e29b-41d4-a716-446655440090',
    'Une recette douce et vive, cuisinée en petite série avec les légumes de saison.',
    '[{"nom":"Carottes des Landes","pourcentage":70,"allergenes":[]},{"nom":"Crème légère","pourcentage":20,"allergenes":["lait"]},{"nom":"Poivre fumé","pourcentage":10,"allergenes":["moutarde"]}]'::jsonb,
    '{"energie_kcal": 118, "glucides": 12.0, "lipides": 6.0, "proteines": 4.8, "per_100g": true}'::jsonb,
    'https://images.unsplash.com/photo-1547592180-85f173990554?auto=format&fit=crop&w=900&q=80',
    'B',
    118,
    12,
    6,
    4.8,
    ARRAY['lait','moutarde']::text[],
    ARRAY['Artisanal','Lot-et-Garonne'],
    ARRAY[
        'https://images.unsplash.com/photo-1547592180-85f173990554?auto=format&fit=crop&w=900&q=80',
        'https://images.unsplash.com/photo-1511690743698-d9d85f2fbf38?auto=format&fit=crop&w=900&q=80'
    ],
    'Servir chaud avec quelques graines torréfiées.',
    'Valeurs nutritionnelles pour 100 g. Conserver au frais après ouverture.'
);

INSERT INTO product_ingredients (
    id, product_id, ingredient_id, producer_id, quantity, unit, ingredient_category, notes, sort_order
) VALUES
(
    '550e8400-e29b-41d4-a716-446655440110',
    '550e8400-e29b-41d4-a716-446655440100',
    '550e8400-e29b-41d4-a716-446655440020',
    '550e8400-e29b-41d4-a716-446655440001',
    70,
    '%',
    'legumes',
    'Base de la recette',
    0
),
(
    '550e8400-e29b-41d4-a716-446655440111',
    '550e8400-e29b-41d4-a716-446655440100',
    '550e8400-e29b-41d4-a716-446655440022',
    '550e8400-e29b-41d4-a716-446655440001',
    20,
    '%',
    'laitages',
    'Onctuosité',
    1
),
(
    '550e8400-e29b-41d4-a716-446655440112',
    '550e8400-e29b-41d4-a716-446655440100',
    '550e8400-e29b-41d4-a716-446655440021',
    '550e8400-e29b-41d4-a716-446655440002',
    10,
    '%',
    'assaisonnement',
    'Final aromatique',
    2
);

INSERT INTO batches (
    id, product_id, producer_id, lot_code, production_date, dluo_ddm, quantity_produced,
    production_site, operators, production_parameters, resultats_controle, docs_associes, state, notes
) VALUES (
    '550e8400-e29b-41d4-a716-446655440200',
    '550e8400-e29b-41d4-a716-446655440100',
    '550e8400-e29b-41d4-a716-446655440001',
    'VELOUTE-2026-04-19-A',
    NOW(),
    NOW() + INTERVAL '180 days',
    240,
    'Atelier principal',
    ARRAY['Camille Martin'],
    '{"cuisson_temp_c": 92, "cuisson_duree_min": 45, "ligne_production":"cuve-01"}'::jsonb,
    NULL,
    NULL,
    'published',
    'Lot de démonstration pour GUI et fiche publique'
);

INSERT INTO qa_checks (
    id, batch_id, check_type, value, min_threshold, max_threshold, is_compliant, unit, operator_id, notes, attachments, checked_at
) VALUES (
    '550e8400-e29b-41d4-a716-446655440250',
    '550e8400-e29b-41d4-a716-446655440200',
    'temperature',
    92.0,
    85.0,
    95.0,
    true,
    '°C',
    '550e8400-e29b-41d4-a716-446655440050',
    'Température conforme en fin de cuisson',
    ARRAY['https://example.com/docs/controle-temperature.jpg'],
    NOW()
);

INSERT INTO qr_tags (
    id, batch_id, slug, short_url, qr_code_svg, qr_code_png, print_count, scan_count, last_scanned_at, is_active
) VALUES (
    '550e8400-e29b-41d4-a716-446655440300',
    '550e8400-e29b-41d4-a716-446655440200',
    'veldemo1',
    'http://localhost:3030/t/veldemo1',
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><rect width="100" height="100" fill="#f3f6f0"/><text x="18" y="55" font-size="11">veldemo1</text></svg>',
    NULL,
    1,
    0,
    NULL,
    true
);

COMMIT;
