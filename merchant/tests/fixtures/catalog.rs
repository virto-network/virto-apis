use fake::faker::company::en::Buzzword;
use merchant::catalog::backend::{Id, SqlCatalogItemVariation};
use merchant::catalog::models::{Item, ItemCategory, ItemMeasurmentUnits, ItemVariation, Price, ItemModification};

use fake::faker::lorem::en::*;
use fake::faker::name::raw::*;
use fake::locales::*;
use fake::Fake;

pub fn fake_item() -> Item {
    let tags: Vec<String> = Words(3..5).fake();
    Item {
        category: ItemCategory::Shop,
        tags,
        enabled: true,
        name: Name(EN).fake(),
        description: "world".to_string(),
    }
}

pub fn fake_item_variation<T>(id: T) -> ItemVariation<T> {
    ItemVariation {
        item_id: id,
        images: vec![],
        enabled: true,
        measurement_units: ItemMeasurmentUnits::Area,
        name: Name(EN).fake(),
        price: Price::Fixed {
            amount: (100.0f32..1000.0f32).fake::<f32>(),
            currency: "USD".to_string(),
        },
        sku: Buzzword().fake(),
        available_units: 10,
        upc: None,
    }
}

pub fn fake_item_modification<T>(id: T) -> ItemModification<T> {
    ItemModification {
        item_id: id,
        images: vec![],
        enabled: true,
        name: Name(EN).fake(),
        price: Price::Fixed {
            amount: (100.0f32..1000.0f32).fake::<f32>(),
            currency: "USD".to_string(),
        },
    }
}