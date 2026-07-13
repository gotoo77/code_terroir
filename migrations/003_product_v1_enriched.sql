-- Migration V3 - réalignement métier selon TODO.md et SPECIFICATIONS.md

ALTER TABLE producers
ADD COLUMN IF NOT EXISTS photo_url TEXT,
ADD COLUMN IF NOT EXISTS categorie_principale VARCHAR(100);

ALTER TABLE ingredients
ADD COLUMN IF NOT EXISTS category VARCHAR(100);

ALTER TABLE products
ADD COLUMN IF NOT EXISTS image_url TEXT,
ADD COLUMN IF NOT EXISTS nutriscore VARCHAR(1),
ADD COLUMN IF NOT EXISTS energie_kcal_100g REAL,
ADD COLUMN IF NOT EXISTS glucides_100g REAL,
ADD COLUMN IF NOT EXISTS lipides_100g REAL,
ADD COLUMN IF NOT EXISTS proteines_100g REAL,
ADD COLUMN IF NOT EXISTS allergenes TEXT[] NOT NULL DEFAULT '{}';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM information_schema.table_constraints
        WHERE constraint_name = 'products_nutriscore_valid'
          AND table_name = 'products'
    ) THEN
        ALTER TABLE products
            ADD CONSTRAINT products_nutriscore_valid
            CHECK (nutriscore IS NULL OR nutriscore IN ('A', 'B', 'C', 'D', 'E'));
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS product_ingredients (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    ingredient_id UUID NOT NULL REFERENCES ingredients(id) ON DELETE RESTRICT,
    producer_id UUID NOT NULL REFERENCES producers(id) ON DELETE RESTRICT,
    quantity REAL,
    unit VARCHAR(30),
    ingredient_category VARCHAR(100),
    notes TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ingredients_category ON ingredients(category);
CREATE INDEX IF NOT EXISTS idx_producers_categorie_principale ON producers(categorie_principale);
CREATE INDEX IF NOT EXISTS idx_products_nutriscore ON products(nutriscore);
CREATE INDEX IF NOT EXISTS idx_products_allergenes ON products USING gin(allergenes);
CREATE INDEX IF NOT EXISTS idx_product_ingredients_product_id ON product_ingredients(product_id);
CREATE INDEX IF NOT EXISTS idx_product_ingredients_ingredient_id ON product_ingredients(ingredient_id);
CREATE INDEX IF NOT EXISTS idx_product_ingredients_producer_id ON product_ingredients(producer_id);

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM information_schema.triggers
        WHERE trigger_name = 'update_product_ingredients_updated_at'
          AND event_object_table = 'product_ingredients'
    ) THEN
        CREATE TRIGGER update_product_ingredients_updated_at
            BEFORE UPDATE ON product_ingredients
            FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
    END IF;
END $$;
