pub(crate) mod backend;
pub(crate) mod models;

pub use backend::{Account, Catalog, CatalogObject, CatalogObjectDocument, Id, ItemVariation};
pub use models::{Image, Item, ItemCategory, ItemMeasurmentUnits, Price};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct QueryOptions {
    pub name: Option<String>,
    pub tags: Option<Vec<String>>,
    pub max_price: Option<f32>,
    pub min_price: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum OrderField {
    CreatedAt,
    Price,
}

#[async_trait]
pub(crate) trait CatalogService {
    type Id;
    type Query;
    type Account: AsRef<str>;

    async fn create(
        &self,
        account: &Self::Account,
        catalog: &models::CatalogObject<Self::Id>,
    ) -> Result<models::CatalogObjectDocument<Self::Id, Self::Account>, CatalogError>;

    async fn exists(&self, id: &Self::Id) -> Result<bool, CatalogError>;

    async fn read(
        &self,
        id: Self::Id,
    ) -> Result<models::CatalogObjectDocument<Self::Id, Self::Account>, CatalogError>;

    async fn update(
        &self,
        id: Self::Id,
        catalog_document: &models::CatalogObject<Self::Id>,
    ) -> Result<models::CatalogObjectDocument<Self::Id, Self::Account>, CatalogError>;

    async fn list(
        &self,
        account: &Self::Account,
        query: &Self::Query,
    ) -> Result<Vec<models::CatalogObjectDocument<Self::Id, Self::Account>>, CatalogError>;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IncreaseItemVariationUnitsPayload<Id> {
    pub id: Id,
    pub units: i32,
}

impl std::error::Error for CatalogError {}

#[derive(Debug, PartialEq, Eq)]
pub enum CatalogError {
    StorageError,
    CatalogEntryNotFound(String),
    CatalogBadRequest,
    MappingError,
}

impl Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}
