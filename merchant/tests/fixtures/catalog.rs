use std::collections::HashMap;

use fake::faker::company::en::Buzzword;
use merchant::catalog::backend::{Id, SqlCatalogItemVariation};
use merchant::catalog::models::{
    Control, Delivery, Item, ItemCategory, ItemControl, ItemDelivery, ItemMeasurmentUnits,
    ItemModification, ItemVariation, MatrixControl, MatrixProp, Price,
};

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
        images: vec![],
        name: Name(EN).fake(),
        description: "world".to_string(),
        warranty_time: None,
    }
}

pub fn fake_item_variation<T>(item_id: T) -> ItemVariation<T> {
    ItemVariation {
        extra_attributes: None,
        processing_time: None,
        item_id,
        images: vec![],
        enabled: true,
        measurement_units: ItemMeasurmentUnits::Area,
        name: Name(EN).fake(),
        price: Price::Fixed {
            amount: (100.0f32..1000.0f32).fake::<f32>(),
            asset_name: "USD".to_string(),
            asset_scale: 2,
        },
        sku: Buzzword().fake(),
        available_units: 10,
        upc: None,
    }
}

pub fn fake_item_control<T>(item_id: T) -> ItemControl<T> {
    ItemControl {
        control: Control::Matrix(MatrixControl {
            combinations: HashMap::new(),
            key_template: String::from(":color-:size"),
            props: vec![
                MatrixProp {
                    name: String::from("color"),
                    options: vec![
                        String::from("Blue"),
                        String::from("Red"),
                        String::from("Yellow"),
                    ],
                },
                MatrixProp {
                    name: String::from("size"),
                    options: vec![
                        String::from("S"),
                        String::from("M"),
                        String::from("X"),
                        String::from("L"),
                    ],
                },
            ],
        }),
        item_id,
    }
}

pub fn fake_item_delivery<T>(item_id: T) -> ItemDelivery<T> {
    ItemDelivery {
        delivery: Delivery::Shipping {
            width_mm: 1,
            length_mm: 1,
            height_mm: 1,
            weight_grams: 200,
        },
        item_id,
    }
}

pub fn fake_item_modification<T>(id: T) -> ItemModification<T> {
    ItemModification {
        processing_time: None,
        warranty_time: None,
        item_id: id,
        images: vec![],
        enabled: true,
        name: Name(EN).fake(),
        price: Price::Fixed {
            amount: (100.0f32..1000.0f32).fake::<f32>(),
            asset_name: "USD".to_string(),
            asset_scale: 2,
        },
    }
}
