use fake::faker::company::en::Buzzword;
use merchant::catalog::{Id, Item, ItemCategory, ItemMeasurmentUnits, ItemVariation, Price};

use fake::faker::lorem::en::*;
use fake::faker::name::raw::*;
use fake::locales::*;
use fake::Fake;

pub fn fake_item() -> Item {
    Item {
        category: ItemCategory::Shop,
        tags: Words(3..5).fake(),
        enabled: true,
        name: Name(EN).fake(),
        description: "world".to_string(),
    }
}

pub fn fake_item_variation(item_id: Id) -> ItemVariation {
    ItemVariation {
        item_id,
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
