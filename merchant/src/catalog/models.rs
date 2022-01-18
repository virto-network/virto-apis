use serde_with::with_prefix;
use serde::{Serialize, Deserialize};

with_prefix!(price_prefix "price_");

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ItemMeasurmentUnits {
  Time,
  Area,
  Custom,
  Generic,
  Units,
  Length,
  Volume,
  Weight,
}

#[derive(Serialize, Deserialize, Debug, Clone,  PartialEq, Eq)]
pub enum ItemCategory {
  Shop,
  Restaurant,
  Liquor,
  Beuty,
  FashionAndAccesories,
  Technology,
  Home,
  FarmacyAndHelth,
  VehiclesAndAccesories,
  Sports,
  Pets,
  ArtAndCrafts,
  ToolsAndGarden,
  BabysAndKids,
  Entertainment,
  ToysAndGames,
  BusinessesAndSupplies,
  SexShop,
  PaperWork
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, PartialOrd)]
#[serde(tag = "type")]
pub enum Price {
  Fixed {
    amount: f32,
    currency: String,
  }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Image {
  pub url: String,
}
#[derive(Serialize, Deserialize, Debug, Clone,  PartialEq, Eq)]
pub struct Item {
  pub category: ItemCategory,
  pub tags: Vec<String>,
  pub name: String,
  pub description: String,
  pub enabled: bool
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ItemVariation<TUuid> {
  pub item_uuid: TUuid,
  pub name: String,
  pub sku: String,
  pub images: Vec<Image>,
  pub upc: Option<String>,
  pub enabled: bool,
  pub measurement_units: ItemMeasurmentUnits,
  pub available_units: i32,
  #[serde(flatten, with = "price_prefix")]
  pub price: Price,
}


#[derive(Serialize, Deserialize, Debug)]
pub struct ItemModification<TUuid> {
  pub item_uuid: TUuid,
  pub name: String,
  pub images: Vec<Image>,
  #[serde(flatten, with = "price_prefix")]
  pub price: Price,
  pub enabled: bool
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "data")]
pub enum CatalogObject<TUuid> {
  Item(Item),
  Variation(ItemVariation<TUuid>),
  Modification(ItemModification<TUuid>),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CatalogObjectDocument<TUuid, TAccount> {
  pub uuid: TUuid,
  pub account: TAccount,
  pub version: chrono::NaiveDateTime,
  pub created_at: chrono::NaiveDateTime,
  #[serde(flatten)]
  pub catalog_object: CatalogObject<TUuid>,
}

