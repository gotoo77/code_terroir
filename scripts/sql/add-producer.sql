INSERT INTO producers (
    id,
    raison_sociale,
    adresse,
    code_postal,
    ville,
    pays,
    email,
    telephone,
    site_web
) VALUES (
    gen_random_uuid(),
    'Ferme du Capybara',
    '12 Chemin des Marais',
    '34000',
    'Montpellier',
    'France',
    'contact@capybara.bio',
    '+33 6 12 34 56 78',
    'https://ferme-capybara.bio'
);
