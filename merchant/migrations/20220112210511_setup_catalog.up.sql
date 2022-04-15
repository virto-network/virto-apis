CREATE TABLE IF NOT EXISTS Catalogs
(
    id INT PRIMARY KEY NOT NULL,
    account VARCHAR(30) NOT NUll,
    type_entry VARCHAR(20) NOT NULL,
    version SMALLINT DEFAULT CURRENT_TIMESTAMP NOT NULL,
    item_data JSONB DEFAULT NULL,
    item_variation_data JSONB DEFAULT NULL,
    item_modification_data JSONB DEFAULT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS item_index ON Catalogs (item_data);
CREATE INDEX IF NOT EXISTS item_variation_index ON Catalogs (item_variation_data);
CREATE INDEX IF NOT EXISTS item_modification_index ON Catalogs (item_modification_data);
