CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS Catalogs
(
    uuid uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    account varchar(30) NOT NUll,
    type_entry varchar(20) NOT NULL,
    version timestamp DEFAULT now() NOT NULL,
    item_data JSONB DEFAULT NULL,
    item_variation_data JSONB DEFAULT NULL,
    item_modification_data JSONB DEFAULT NULL,
    created_at timestamp DEFAULT now()
);

CREATE INDEX IF NOT EXISTS item_index ON Catalogs USING gin (item_data);
CREATE INDEX IF NOT EXISTS item_variation_index ON Catalogs USING gin (item_variation_data);
CREATE INDEX IF NOT EXISTS item_modification_index ON Catalogs USING gin (item_modification_data);