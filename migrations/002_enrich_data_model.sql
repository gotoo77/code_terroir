-- Migration pour enrichir le modèle de données selon les spécifications détaillées
-- Version 2 - 2025-09-18

-- Modifier la table products pour ajouter les champs manquants
ALTER TABLE products
ADD COLUMN IF NOT EXISTS ingredients JSONB, -- Liste détaillée des ingrédients avec %
ADD COLUMN IF NOT EXISTS visuels TEXT[], -- Photos produit, logos (remplace image_urls)
ADD COLUMN IF NOT EXISTS conseils_utilisation TEXT, -- Suggestions service, accords
ADD COLUMN IF NOT EXISTS infos_legales TEXT; -- Mentions obligatoires

-- Renommer usage_advice en conseils_utilisation si nécessaire
-- (On garde les deux pour compatibilité)

-- Modifier la table batches pour ajouter les contrôles qualité et docs
ALTER TABLE batches
ADD COLUMN IF NOT EXISTS resultats_controle JSONB, -- Liste des contrôles qualité
ADD COLUMN IF NOT EXISTS docs_associes JSONB; -- Documents associés (HACCP, photos, etc.)

-- Créer table pour les boîtes individuelles (optionnel)
CREATE TABLE IF NOT EXISTS individual_boxes (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    batch_id UUID NOT NULL REFERENCES batches(id) ON DELETE CASCADE,
    serial_number VARCHAR(50) NOT NULL, -- Ex: "0001/0240"
    qr_code_slug VARCHAR(20) UNIQUE, -- Slug unique pour QR
    numero_sequence INTEGER NOT NULL, -- Position dans le lot (1, 2, 3...)
    total_sequence INTEGER NOT NULL, -- Nombre total de boîtes dans le lot
    date_numerotage TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    operateur_id UUID NOT NULL REFERENCES users(id),
    statut VARCHAR(20) NOT NULL DEFAULT 'active' CHECK (statut IN ('active', 'vendue', 'perdue', 'rappel')),
    commentaire TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(batch_id, numero_sequence) -- Pas de doublons dans un lot
);

-- Créer table pour l'historique des scans de boîtes individuelles
CREATE TABLE IF NOT EXISTS box_scan_logs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    box_id UUID NOT NULL REFERENCES individual_boxes(id) ON DELETE CASCADE,
    scan_date TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ip_address INET,
    user_agent TEXT,
    geolocation VARCHAR(100), -- Pays/ville si détectable
    scan_type VARCHAR(20) NOT NULL DEFAULT 'qr_scan' CHECK (scan_type IN ('qr_scan', 'manual_lookup', 'api_call')),
    additional_data JSONB
);

-- Ajouter des index pour les performances
CREATE INDEX IF NOT EXISTS idx_products_ingredients ON products USING gin(ingredients);
CREATE INDEX IF NOT EXISTS idx_products_visuels ON products USING gin(visuels);
CREATE INDEX IF NOT EXISTS idx_batches_resultats_controle ON batches USING gin(resultats_controle);
CREATE INDEX IF NOT EXISTS idx_batches_docs_associes ON batches USING gin(docs_associes);

-- Index pour les nouvelles tables
CREATE INDEX IF NOT EXISTS idx_individual_boxes_batch_id ON individual_boxes(batch_id);
CREATE INDEX IF NOT EXISTS idx_individual_boxes_qr_code_slug ON individual_boxes(qr_code_slug);
CREATE INDEX IF NOT EXISTS idx_individual_boxes_statut ON individual_boxes(statut);
CREATE INDEX IF NOT EXISTS idx_box_scan_logs_box_id ON box_scan_logs(box_id);
CREATE INDEX IF NOT EXISTS idx_box_scan_logs_scan_date ON box_scan_logs(scan_date);
CREATE INDEX IF NOT EXISTS idx_box_scan_logs_scan_type ON box_scan_logs(scan_type);

-- Trigger pour individual_boxes (seulement si n'existe pas)
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.triggers
                   WHERE trigger_name = 'update_individual_boxes_updated_at'
                   AND event_object_table = 'individual_boxes') THEN
        CREATE TRIGGER update_individual_boxes_updated_at
            BEFORE UPDATE ON individual_boxes
            FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
    END IF;
END $$;

-- Améliorer la table recipes (elle était basique)
ALTER TABLE recipes
ADD COLUMN IF NOT EXISTS ingredients_detail JSONB, -- Ingrédients avec quantités, fournisseurs
ADD COLUMN IF NOT EXISTS etapes_detail JSONB; -- Étapes détaillées avec températures, durées

-- Ajouter des contraintes sur les recettes (si elles n'existent pas déjà)
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints
                   WHERE constraint_name = 'recipes_version_positive'
                   AND table_name = 'recipes') THEN
        ALTER TABLE recipes ADD CONSTRAINT recipes_version_positive CHECK (version > 0);
    END IF;
END $$;

-- Index pour les recettes
CREATE INDEX IF NOT EXISTS idx_recipes_producer_name_version ON recipes(producer_id, name, version);
CREATE INDEX IF NOT EXISTS idx_recipes_active ON recipes(is_active);

-- Vue pour obtenir la version active d'une recette
CREATE OR REPLACE VIEW active_recipes AS
SELECT DISTINCT ON (producer_id, name) *
FROM recipes
WHERE is_active = true
ORDER BY producer_id, name, version DESC;

-- Fonctions utilitaires

-- Fonction pour générer le prochain numéro de version d'une recette
CREATE OR REPLACE FUNCTION get_next_recipe_version(p_producer_id UUID, p_name TEXT)
RETURNS INTEGER AS $$
BEGIN
    RETURN COALESCE(
        (SELECT MAX(version) + 1
         FROM recipes
         WHERE producer_id = p_producer_id AND name = p_name),
        1
    );
END;
$$ LANGUAGE plpgsql;

-- Fonction pour générer une série de boîtes numérotées
CREATE OR REPLACE FUNCTION create_box_series(
    p_batch_id UUID,
    p_quantity INTEGER,
    p_operator_id UUID,
    p_format TEXT DEFAULT '{numero:04}/{total:04}'
) RETURNS VOID AS $$
DECLARE
    i INTEGER;
    serial_num TEXT;
BEGIN
    FOR i IN 1..p_quantity LOOP
        -- Générer le numéro de série formaté
        serial_num := REPLACE(REPLACE(p_format, '{numero:04}', LPAD(i::TEXT, 4, '0')), '{total:04}', LPAD(p_quantity::TEXT, 4, '0'));
        serial_num := REPLACE(REPLACE(serial_num, '{numero}', i::TEXT), '{total}', p_quantity::TEXT);

        INSERT INTO individual_boxes (
            batch_id,
            serial_number,
            numero_sequence,
            total_sequence,
            operateur_id
        ) VALUES (
            p_batch_id,
            serial_num,
            i,
            p_quantity,
            p_operator_id
        );
    END LOOP;
END;
$$ LANGUAGE plpgsql;

-- Ajouter des commentaires pour documenter les tables
COMMENT ON TABLE individual_boxes IS 'Boîtes individuelles pour séries limitées et storytelling poussé';
COMMENT ON TABLE box_scan_logs IS 'Historique des scans QR pour traçabilité fine';
COMMENT ON COLUMN products.ingredients IS 'Liste détaillée des ingrédients avec pourcentages et allergènes';
COMMENT ON COLUMN products.conseils_utilisation IS 'Suggestions de service, accords mets-vins';
COMMENT ON COLUMN batches.resultats_controle IS 'Résultats des contrôles qualité (pH, température, etc.)';
COMMENT ON COLUMN batches.docs_associes IS 'Documents associés (fiches HACCP, photos cuves, analyses labo)';

-- Mettre à jour les stats de la base
ANALYZE;