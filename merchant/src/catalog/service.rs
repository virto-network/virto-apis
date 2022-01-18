use std::fmt::Display;

use super::super::utils::query::{ Query };
use super::models::{ CatalogObject, CatalogObjectDocument};
sea_query::sea_query_driver_postgres!();

use async_trait::async_trait;
use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize, Debug)]
pub struct IncreaseItemVariationUnitsPayload<TUuid> {
  pub uuid: TUuid,
  pub units: i32
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum CatalogCmd<TUuid> {
  IncreaseItemVariationUnits(IncreaseItemVariationUnitsPayload<TUuid>),
}

#[async_trait]
pub trait Commander<TAccount> {
  type Cmd;
  async fn cmd(&self, account: TAccount, cmd: Self::Cmd) -> Result<(), CatalogError>;
}

#[derive(Serialize, Deserialize, Debug)]
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

#[async_trait]
pub trait CatalogService<TUuid, TAccount>: Commander<TAccount> {
  async fn create(&self, account: TAccount, catalog: &CatalogObject<TUuid>) -> Result<CatalogObjectDocument<TUuid, TAccount>, CatalogError>;
  async fn exists(&self, account: TAccount, uuid: TUuid) -> Result<bool, CatalogError>;
  async fn read(&self, account: TAccount, uuid: TUuid) -> Result<CatalogObjectDocument<TUuid, TAccount>, CatalogError>;
  async fn update(&self, account: TAccount, uuid: TUuid, catalog_document: &CatalogObject<TUuid>) -> Result<CatalogObjectDocument<TUuid, TAccount>, CatalogError>;
  async fn list(&self, account: TAccount, query: &Query<ListCatalogQueryOptions, CatalogColumnOrder>) -> Result<Vec<CatalogObjectDocument<TUuid, TAccount>>, CatalogError>;
}

impl std::error::Error for CatalogError {}

#[derive(Debug, PartialEq, Eq)]
pub enum CatalogError {
  DatabaseError,
  CatalogEntryNotFound(String),
  CatalogBadRequest,
  MappingError
}

impl Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}


