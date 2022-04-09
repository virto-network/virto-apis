use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_with::with_prefix;
use sqlx::types::chrono::NaiveDateTime;

with_prefix!(price_prefix "price_");
with_prefix!(warranty_prefix "warranty_time_");
with_prefix!(processing_prefix "processing_time_");
with_prefix!(delivery_prefix "delivery_");

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


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, PartialOrd, Eq)]
#[serde(tag = "type")]
pub enum Time {
    Fixed { seconds: u32 },
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
    #[serde(flatten, with = "warranty_prefix")]
    pub warranty_time: Option<Time>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ItemVariation<Id> {
    pub item_id: Id,
    pub name: String,
    #[serde(flatten, with = "processing_prefix")]
    pub processing_time: Option<Time>,
    pub sku: String,
    pub images: Vec<Image>,
    pub upc: Option<String>,
    pub enabled: bool,
    pub measurement_units: ItemMeasurmentUnits,
    pub available_units: i32,
    #[serde(flatten, with = "price_prefix")]
    pub price: Price,
    // #[serde(flatten)]
    pub extra_attributes: Option<HashMap<String, String>>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ItemModification<Id> {
    pub item_id: Id,
    pub name: String,
    #[serde(flatten, with = "processing_prefix")]
    pub processing_time: Option<Time>,
    #[serde(flatten, with = "warranty_prefix")]
    pub warranty_time: Option<Time>,
    pub images: Vec<Image>,
    #[serde(flatten, with = "price_prefix")]
    pub price: Price,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, PartialOrd)]
#[serde(tag = "type")]
pub enum Delivery {
  Shipping { width_mm: i32, length_mm: i32, height_mm: i32, weight_grams: i32 },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ItemDelivery<Id> {
    pub item_id: Id,
    #[serde(flatten, with = "delivery_prefix")]
    pub delivery: Delivery
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ControlOption {
  value: String,
  name: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Prop {
  name: String,
  options: Vec<ControlOption>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MatrixControl<Id> {
  matrix_map: HashMap<String, Id>,
  key_template: String,
  props: Vec<Prop>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommonFormItem {
  name: String
}

#[derive(Serialize, Deserialize, Debug)]
pub enum FormItem {
  Text(CommonFormItem),
  Email(CommonFormItem),
  Password(CommonFormItem)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FormControl<Id> {
  item_id: Id,
  form: Vec<FormItem>
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Control<Id> {
  Matrix(MatrixControl<Id>),
  Form(FormControl<Id>)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ItemControl<Id> {
    pub item_id: Id,
    #[serde(flatten, with = "delivery_prefix")]
    pub control: Control<Id>
}


#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "data")]
pub enum CatalogObject<Id> {
    Item(Item),
    Variation(ItemVariation<Id>),
    Modification(ItemModification<Id>),
    Delivery(ItemDelivery<Id>),
    Control(ItemControl<Id>)
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

#[derive(Serialize, Deserialize, Debug)]
pub struct CatalogObjectBulkDocument<Id> {
    pub id: Option<Id>,
    #[serde(flatten)]
    pub catalog_object: CatalogObject<Id>,
}
