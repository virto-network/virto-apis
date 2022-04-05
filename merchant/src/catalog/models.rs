use serde::{Deserialize, Serialize};
use serde_with::with_prefix;
use sqlx::types::chrono::NaiveDateTime;

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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
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
    PaperWork,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, PartialOrd)]
#[serde(tag = "type")]
pub enum Price {
    Fixed { amount: f32, currency: String },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub url: String,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub category: ItemCategory,
    pub tags: Vec<String>,
    pub name: String,
    pub description: String,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ItemVariation<Id> {
    pub item_id: Id,
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
pub struct ItemModification<Id> {
    pub item_id: Id,
    pub name: String,
    pub images: Vec<Image>,
    #[serde(flatten, with = "price_prefix")]
    pub price: Price,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "data")]
pub enum CatalogObject<Id> {
    Item(Item),
    Variation(ItemVariation<Id>),
    Modification(ItemModification<Id>),
}

#[allow(dead_code)]
impl<Id> CatalogObject<Id> {
    pub fn item(&self) -> Option<&Item> {
        match self {
            Self::Item(it) => Some(it),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CatalogObjectDocument<Id, Account> {
    pub id: Id,
    pub account: Account,
    pub version: NaiveDateTime,
    pub created_at: NaiveDateTime,
    #[serde(flatten)]
    pub catalog_object: CatalogObject<Id>,
}
