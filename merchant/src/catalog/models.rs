use std::{
    collections::HashMap,
    fmt::{self, Display}, default,
};

use serde::{Deserialize, Serialize};
use serde_with::with_prefix;
use sqlx::types::chrono::NaiveDateTime;

with_prefix!(price_prefix "price_");
with_prefix!(warranty_prefix "warranty_time_");
with_prefix!(processing_prefix "processing_time_");
with_prefix!(delivery_prefix "delivery_");
with_prefix!(control_prefix "control_");

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
    Fixed { 
        amount: f32,
        asset_name: String,
        asset_scale: i8
    },
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

impl Default for Image {
    fn default() -> Self {
        Image {
            url: "".to_owned()
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub category: ItemCategory,
    pub tags: Vec<String>,
    pub name: String,
    #[serde(default = "default_images")]
    pub images: Vec<Image>,
    pub description: String,
    pub enabled: bool,
    #[serde(flatten, with = "warranty_prefix")]
    pub warranty_time: Option<Time>,
}

fn default_images() -> Vec<Image> {
    vec![]
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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
    pub extra_attributes: Option<HashMap<String, String>>,
}



#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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
    Shipping {
        width_mm: i32,
        length_mm: i32,
        height_mm: i32,
        weight_grams: i32,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ItemDelivery<Id> {
    pub item_id: Id,
    #[serde(flatten, with = "delivery_prefix")]
    pub delivery: Delivery,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MatrixProp {
    pub name: String,
    pub options: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MatrixControl<Id> {
    pub combinations: HashMap<String, Id>,
    pub key_template: String,
    pub props: Vec<MatrixProp>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum FormItem {
    Text(HashMap<String, String>),
    Email(HashMap<String, String>),
    Password(HashMap<String, String>),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum Control<Id> {
    Matrix(MatrixControl<Id>),
    Form(Vec<FormItem>),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ItemControl<Id> {
    pub item_id: Id,
    #[serde(flatten, with = "control_prefix")]
    pub control: Control<Id>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum CatalogObject<Id> {
    Item(Item),
    Variation(ItemVariation<Id>),
    Modification(ItemModification<Id>),
    Delivery(ItemDelivery<Id>),
    Control(ItemControl<Id>),
}

impl<Id> Display for CatalogObject<Id> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Item(_) => write!(f, "Item"),
            Self::Variation(_) => write!(f, "Variation"),
            Self::Modification(_) => write!(f, "Modification"),
            Self::Delivery(_) => write!(f, "Delivery"),
            Self::Control(_) => write!(f, "Control"),
        }
    }
}

// TODO: see if we  need to remove
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
