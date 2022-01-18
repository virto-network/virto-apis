-- Add down migration script here
DROP TABLE IF EXISTS catalogs;
DROP INDEX IF EXISTS item_index;
DROP INDEX IF EXISTS item_variation_index;
DROP INDEX IF EXISTS item_modification_index;
