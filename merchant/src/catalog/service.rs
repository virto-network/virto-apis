use std::{collections::HashMap, fmt::Display};

use super::models::{CatalogObject, CatalogObjectBulkDocument, CatalogObjectDocument};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct IncreaseItemVariationUnitsPayload<Id> {
    pub id: Id,
    pub units: i32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum CatalogCmd<Id> {
    IncreaseItemVariationUnits(IncreaseItemVariationUnitsPayload<Id>),
}

#[async_trait]
pub trait Commander {
    type Account;
    type Cmd;
    async fn cmd(&self, account: &Self::Account, cmd: Self::Cmd) -> Result<(), CatalogError>;
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ListCatalogQueryOptions {
    pub name: Option<String>,
    pub tags: Option<Vec<String>>,
    pub max_price: Option<f32>,
    pub min_price: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum CatalogColumnOrder {
    CreatedAt,
    Price,
}

pub trait BulkDocumentReferencesResolver {
    type Id;
    fn resolve(
        idMap: &HashMap<&str, Self::Id>,
        catalog: &CatalogObject<String>,
    ) -> Result<CatalogObject<Self::Id>, CatalogError>;
}

pub type CatalogId<Trait> = <Trait as CatalogService>::Id;
#[async_trait]
pub trait CatalogService: BulkDocumentReferencesResolver<Id = CatalogId<Self>> + Commander {
    type Id;
    type Query: Send;

    async fn create(
        &self,
        account: &Self::Account,
        catalog: &CatalogObject<CatalogId<Self>>,
    ) -> Result<CatalogObjectDocument<CatalogId<Self>, Self::Account>, CatalogError>;

    async fn bulk_create(
        &self,
        account: &Self::Account,
        catalog: &[CatalogObjectBulkDocument<String>], // we use string type for Id to enable referecing
    ) -> Result<Vec<CatalogObjectDocument<CatalogId<Self>, Self::Account>>, CatalogError>;

    async fn exists(
        &self,
        account: &Self::Account,
        id: &CatalogId<Self>,
    ) -> Result<bool, CatalogError>;

    async fn read(
        &self,
        account: &Self::Account,
        id: &CatalogId<Self>,
    ) -> Result<CatalogObjectDocument<CatalogId<Self>, Self::Account>, CatalogError>;

    async fn update(
        &self,
        account: &Self::Account,
        id: &CatalogId<Self>,
        catalog_document: &CatalogObject<CatalogId<Self>>,
    ) -> Result<CatalogObjectDocument<CatalogId<Self>, Self::Account>, CatalogError>;

    async fn list(
        &self,
        account: &Self::Account,
        query: &Self::Query,
    ) -> Result<Vec<CatalogObjectDocument<CatalogId<Self>, Self::Account>>, CatalogError>;
}

impl std::error::Error for CatalogError {}

#[derive(Debug, PartialEq, Eq)]
pub enum CatalogError {
    DatabaseError,
    CatalogEntryNotFound(String),
    CatalogBadRequest,
    MappingError,
    BulkReferenceNotExist(String),
}

impl Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}
