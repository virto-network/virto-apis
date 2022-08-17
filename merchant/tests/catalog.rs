mod fixtures;
mod utils;

use merchant::catalog::backend::Account;
use merchant::catalog::models::{
    CatalogObjectBulkDocument, CatalogObjectDocument, Control, Delivery, ItemControl, ItemDelivery,
    ItemModification, ItemVariation,
};
use sqlx::types::chrono::NaiveDateTime;
use utils::InstanceOf;
use utils::{check_if_error_is, restore_db, AnyHow};

use async_std::task::sleep;
use fixtures::catalog::{
    fake_item, fake_item_control, fake_item_modification, fake_item_variation,
};
use std::time::Duration;

use catalog::backend::{
    CatalogSQLService, SqlCatalogItemVariation, SqlCatalogObject, SqlCatalogObjectDocument,
    SqlCatalogQueryOptions,
};
use catalog::models::{CatalogObject, Image, Item, ItemCategory, ItemMeasurmentUnits};
use catalog::service::{
    CatalogCmd, CatalogColumnOrder, CatalogService, Commander, IncreaseItemVariationUnitsPayload,
};
use merchant::catalog;
use merchant::catalog::service::CatalogError;
use merchant::utils::query::{Order, OrderBy};
//use sqlx::types::;
type Id = u32;

const CATALOG_ACCOUNT: &str = "account";

pub fn check_catalog_object_document<T: 'static>(catalog: &CatalogObjectDocument<T, Account>) {
    assert!(
        catalog.version.instance_of::<NaiveDateTime>(),
        "it should be an instance of NaiveDateTime"
    );
    assert!(
        catalog.id.instance_of::<T>(),
        "it should be a instance of Id"
    );
    assert!(
        catalog.account.instance_of::<String>(),
        "the accoutn property should be an str"
    );
    assert!(catalog.created_at.instance_of::<NaiveDateTime>());
}

pub fn check_item_document(catalog: &SqlCatalogObjectDocument, item_object: &Item) {
    check_catalog_object_document(catalog);
    assert!(
        matches!(catalog.catalog_object, CatalogObject::Item(_)),
        "the catalog object should be an item"
    );
    match &catalog.catalog_object {
        CatalogObject::Item(item) => {
            assert!(
                item.tags.instance_of::<Vec<String>>(),
                "tags should be a instance of vector"
            );
            assert!(item.name.instance_of::<String>(), "name should be a string");
            assert!(
                item.description.instance_of::<String>(),
                "description should be an string"
            );
            assert!(
                item.category.instance_of::<ItemCategory>(),
                "description should be an string"
            );
            // item tags
            assert_eq!(item.tags, item_object.tags);
            assert_eq!(item.name, item_object.name);
            assert_eq!(item.description, item_object.description);
            assert!(
                item.category == item_object.category,
                "category are distinct"
            );
        }
        _ => panic!("catalog_object should be an item"),
    }
}

pub fn check_variation_document<T: 'static, Y: 'static>(
    catalog: &CatalogObjectDocument<T, Account>,
    variation: &ItemVariation<Y>,
) {
    check_catalog_object_document(catalog);
    assert!(
        matches!(catalog.catalog_object, CatalogObject::Variation(_)),
        "the catalog object should be an Variation"
    );
    match &catalog.catalog_object {
        CatalogObject::Variation(v) => {
            assert!(
                v.images.instance_of::<Vec<Image>>(),
                "it should be a vector of images"
            );
            assert!(v.item_id.instance_of::<T>(), "it should be an id");
            assert!(
                v.measurement_units.instance_of::<ItemMeasurmentUnits>(),
                "it should be an id"
            );
            assert_eq!(v.images, variation.images);
            // assert_eq!(v.item_id, variation.item_id);
            assert_eq!(v.measurement_units, variation.measurement_units);
            assert_eq!(v.name, variation.name);
            assert_eq!(v.price, variation.price);
            assert_eq!(v.sku, variation.sku);
            assert_eq!(v.available_units, variation.available_units);
            assert_eq!(v.upc, variation.upc);
        }
        _ => panic!("catalog_object should be an item"),
    }
}

pub fn check_control_document<T: 'static, Y: 'static>(
    catalog_a: &CatalogObjectDocument<T, Account>,
    control_b: &ItemControl<Y>,
) {
    check_catalog_object_document(catalog_a);
    assert!(
        matches!(catalog_a.catalog_object, CatalogObject::Control(_)),
        "the catalog object should be a Control"
    );
    assert!(control_b.item_id.instance_of::<Y>(), "it should be an id");
    if let CatalogObject::Control(item_control_a) = &catalog_a.catalog_object {
        match &item_control_a.control {
            Control::Matrix(matrix_a) => {
                if let Control::Matrix(matrix_b) = &control_b.control {
                    let combination_a = matrix_a.to_owned();
                    for (key_a, _) in combination_a.combinations.iter() {
                        assert!(
                            matrix_b.combinations.contains_key(key_a),
                            "combinations_b dont have the requested key"
                        );
                    }

                    assert_eq!(matrix_a.key_template, matrix_b.key_template);

                    for prop in matrix_b.props.iter() {
                        assert!(
                            matches!(matrix_a.props.iter().find(|x| x.name == prop.name), Some(_)),
                            "the catalog object should be an Variation"
                        );
                    }
                } else {
                    panic!("control_b is not a matrix control");
                }
            }
            _ => panic!("not supported other controls than Matrix"),
        }
    } else {
        panic!("catalog_object should be an item");
    }
}

pub fn check_delivery_document<T: 'static + PartialEq, Y: 'static + PartialEq>(
    delivery_a_doc: &CatalogObjectDocument<T, Account>,
    delivery_b: &ItemDelivery<Y>,
) where
    ItemDelivery<T>: PartialEq<ItemDelivery<Y>>,
{
    check_catalog_object_document(delivery_a_doc);
    assert!(
        matches!(delivery_a_doc.catalog_object, CatalogObject::Variation(_)),
        "the catalog object should be an Variation"
    );
    assert!(delivery_b.item_id.instance_of::<Y>(), "it should be an id");

    if let CatalogObject::Delivery(ref delivery_a) = delivery_a_doc.catalog_object {
        if delivery_a != delivery_b {
            panic!("delivery are not the same")
        }
    }
}

pub fn check_modification_document<T: 'static, Y: 'static>(
    catalog: &CatalogObjectDocument<T, Account>,
    modification: &ItemModification<Y>,
) {
    check_catalog_object_document(catalog);
    assert!(
        matches!(catalog.catalog_object, CatalogObject::Modification(_)),
        "the catalog object should be an Modification"
    );
    match &catalog.catalog_object {
        CatalogObject::Modification(m) => {
            assert!(
                m.images.instance_of::<Vec<Image>>(),
                "it should be a vector of images"
            );
            assert!(m.item_id.instance_of::<T>(), "it should be a valid id");
            assert_eq!(m.images, modification.images);
            // assert_eq!(m.item_id, modification.item_id);
            assert_eq!(m.name, modification.name);
            assert_eq!(m.price, modification.price);
        }
        _ => panic!("catalog_object should be an item"),
    }
}

pub async fn make_item(
    catalog_service: &CatalogSQLService,
    item: Item,
) -> Result<SqlCatalogObjectDocument, AnyHow> {
    let entry = SqlCatalogObject::Item(item);
    let catalog_entry_document = catalog_service
        .create(&CATALOG_ACCOUNT.to_string(), &entry)
        .await?;
    Ok(catalog_entry_document)
}

pub async fn make_variation(
    catalog_service: &CatalogSQLService,
    variation: SqlCatalogItemVariation,
) -> Result<SqlCatalogObjectDocument, CatalogError> {
    let entry = SqlCatalogObject::Variation(variation);
    let catalog_entry_document = catalog_service
        .create(&CATALOG_ACCOUNT.to_string(), &entry)
        .await?;
    Ok(catalog_entry_document)
}

// pub async fn make_delivery<T>(
//     catalog_service: &CatalogSQLService,
//     delivery: ItemDelivery<T>,
// ) -> Result<SqlCatalogObjectDocument, CatalogError> {
//     let entry = CatalogObject::Delivery(delivery);
//     let catalog_entry_document = catalog_service
//         .create(&CATALOG_ACCOUNT.to_string(), &entry)
//         .await?;
//     Ok(catalog_entry_document)
// }

#[cfg(test)]
pub mod item_test {
    use super::*;

    #[async_std::test]
    async fn create_item() -> Result<(), AnyHow> {
        let pool = restore_db().await?;
        let catalog_service = CatalogSQLService::new(pool);
        let entry = SqlCatalogObject::Item(fake_item());
        let catalog_entry_document = catalog_service
            .create(&CATALOG_ACCOUNT.to_string(), &entry)
            .await?;
        check_item_document(&catalog_entry_document, entry.item().unwrap());
        Ok(())
    }

    #[async_std::test]
    async fn update_item() -> Result<(), AnyHow> {
        let pool = restore_db().await?;
        let catalog_service = CatalogSQLService::new(pool);
        let item_old = fake_item();
        let item_new = fake_item();
        let item_doc = make_item(&catalog_service, item_old.clone()).await?;
        check_item_document(&item_doc, &item_old);
        let updated_catalog_item = catalog_service
            .update(
                &CATALOG_ACCOUNT.to_string(),
                &item_doc.id,
                &SqlCatalogObject::Item(item_new.clone()),
            )
            .await?;
        check_item_document(&updated_catalog_item, &item_new);
        let item_created =
            as_value!(updated_catalog_item.catalog_object, SqlCatalogObject::Item).unwrap();
        assert_ne!(item_created.name, item_old.name);
        assert_eq!(item_created.name, item_new.name);
        Ok(())
    }

    #[async_std::test]
    async fn read_item() -> Result<(), AnyHow> {
        let pool = restore_db().await?;
        let catalog_service = CatalogSQLService::new(pool);

        let item_doc = make_item(&catalog_service, fake_item()).await?;
        let item = item_doc.catalog_object.item().unwrap();
        check_item_document(&item_doc, &item);
        let read_catalog_item = catalog_service
            .read(&CATALOG_ACCOUNT.to_string(), &item_doc.id)
            .await?;
        check_item_document(&read_catalog_item, &item);
        Ok(())
    }
}

#[async_std::test]
async fn check_if_exists() -> Result<(), AnyHow> {
    let pool = restore_db().await?;
    let catalog_service = CatalogSQLService::new(pool);
    let item = fake_item();
    let catalog_item_document = make_item(&catalog_service, item).await?;
    assert!(
        catalog_service
            .exists(&CATALOG_ACCOUNT.to_string(), &catalog_item_document.id)
            .await?,
        "it should exists"
    );
    assert!(
        !catalog_service
            .exists(&CATALOG_ACCOUNT.to_string(), &Id::default())
            .await?,
        "it should not exists"
    );
    Ok(())
}

#[async_std::test]
async fn read_item_fails_if_id_doesnt_exists() -> Result<(), AnyHow> {
    let pool = restore_db().await?;
    let catalog_service = CatalogSQLService::new(pool);
    let id = Id::default();
    let read_catalog_item = catalog_service
        .read(&CATALOG_ACCOUNT.to_string(), &id)
        .await;
    check_if_error_is(
        read_catalog_item.unwrap_err(),
        CatalogError::CatalogEntryNotFound(id.to_string()),
    );
    Ok(())
}

#[cfg(test)]
pub mod item_variation_test {
    use super::*;

    #[async_std::test]
    async fn create_item_variation() -> Result<(), AnyHow> {
        let pool = restore_db().await?;
        let catalog_service = CatalogSQLService::new(pool);
        let item_doc = make_item(&catalog_service, fake_item()).await?;
        let variation = fake_item_variation(item_doc.id);
        let variation_doc = make_variation(&catalog_service, variation.clone()).await?;

        check_variation_document(&variation_doc, &variation);
        Ok(())
    }

    #[async_std::test]
    async fn create_item_variation_fails_if_not_exists_item_id() -> Result<(), AnyHow> {
        let pool = restore_db().await?;
        let catalog_service = CatalogSQLService::new(pool);
        let variation = fake_item_variation(Id::default());
        let result = make_variation(&catalog_service, variation).await;
        check_if_error_is(result.unwrap_err(), CatalogError::CatalogBadRequest);
        Ok(())
    }
    #[async_std::test]
    async fn update_variation() -> Result<(), AnyHow> {
        let pool = restore_db().await?;
        let catalog_service = CatalogSQLService::new(pool);
        let item = fake_item();
        let item_doc = make_item(&catalog_service, item).await?;
        let variation = fake_item_variation(item_doc.id);
        let variation_new = fake_item_variation(item_doc.id);
        let catalog_variation_document =
            make_variation(&catalog_service, variation.clone()).await?;
        check_variation_document(&catalog_variation_document, &variation);
        let updated_catalog_variation = catalog_service
            .update(
                &CATALOG_ACCOUNT.to_string(),
                &catalog_variation_document.id,
                &SqlCatalogObject::Variation(variation_new.clone()),
            )
            .await?;
        check_variation_document(&updated_catalog_variation, &variation_new);
        let variation_updated = as_value!(
            updated_catalog_variation.catalog_object,
            SqlCatalogObject::Variation
        )
        .unwrap();
        assert_ne!(variation_updated.name, variation.name);
        assert_eq!(variation_updated.name, variation_new.name);
        Ok(())
    }

    #[async_std::test]
    async fn update_variation_fails_if_not_exists_item_id() -> Result<(), AnyHow> {
        let pool = restore_db().await?;
        let catalog_service = CatalogSQLService::new(pool);
        let catalog_item_document = make_item(&catalog_service, fake_item()).await?;
        let variation = fake_item_variation(catalog_item_document.id);
        let variation_new = fake_item_variation(Id::default());
        let catalog_variation_document =
            make_variation(&catalog_service, variation.clone()).await?;
        check_variation_document(&catalog_variation_document, &variation);
        let result = catalog_service
            .update(
                &CATALOG_ACCOUNT.to_string(),
                &catalog_variation_document.id,
                &SqlCatalogObject::Variation(variation_new),
            )
            .await;
        check_if_error_is(result.unwrap_err(), CatalogError::CatalogBadRequest);
        Ok(())
    }

    #[async_std::test]
    async fn read_item() -> Result<(), AnyHow> {
        let pool = restore_db().await?;
        let catalog_service = CatalogSQLService::new(pool);
        let item_doc = make_item(&catalog_service, fake_item()).await?;
        let variation = fake_item_variation(item_doc.id);
        let variation_doc = make_variation(&catalog_service, variation.clone()).await?;
        check_variation_document(&variation_doc, &variation);
        let read_catalog_variation = catalog_service
            .read(&CATALOG_ACCOUNT.to_string(), &variation_doc.id)
            .await?;
        check_variation_document(&read_catalog_variation, &variation);
        Ok(())
    }
}

#[cfg(test)]
pub mod item_find_test {
    use super::*;
    use merchant::catalog::{models::Price, service::ListCatalogQueryOptions};

    #[async_std::test]
    async fn list_item_by_name() -> Result<(), AnyHow> {
        let pool = restore_db().await?;
        let catalog_service = CatalogSQLService::new(pool);
        let doc = make_item(&catalog_service, fake_item()).await?;
        let item = doc.catalog_object.item().unwrap();

        let query_name_not_exists = SqlCatalogQueryOptions {
            limit: None,
            order_by: None,
            options: ListCatalogQueryOptions {
                max_price: None,
                min_price: None,
                name: Some("None".to_string()),
                tags: None,
            },
        };

        let query_name_exists = SqlCatalogQueryOptions {
            limit: None,
            order_by: None,
            options: ListCatalogQueryOptions {
                max_price: None,
                min_price: None,
                name: Some(item.name.clone()),
                tags: None,
            },
        };

        let items_empty = catalog_service
            .list(&CATALOG_ACCOUNT.to_string(), &query_name_not_exists)
            .await?;
        assert_eq!(items_empty.len(), 0);
        let items_found = catalog_service
            .list(&CATALOG_ACCOUNT.to_string(), &query_name_exists)
            .await?;
        assert_eq!(items_found.len(), 1);
        let item_found = &items_found[0];
        check_item_document(item_found, &item);
        Ok(())
    }

    #[async_std::test]
    async fn list_item_by_min_and_max_amount() -> Result<(), AnyHow> {
        let pool = restore_db().await?;
        let catalog_service = CatalogSQLService::new(pool);
        let item_doc = make_item(&catalog_service, fake_item()).await?;
        //let item = doc.catalog_object.item().unwrap();
        let mut variation = fake_item_variation(item_doc.id);
        variation.price = Price::Fixed {
            amount: 2000.0f32,
            asset_name: "USD".to_string(),
            asset_scale: 2,
        };

        let variation_document = make_variation(&catalog_service, variation.clone()).await?;
        let mut variation_two = fake_item_variation(item_doc.id);
        check_variation_document(&variation_document, &variation);

        variation_two.price = Price::Fixed {
            amount: 5000.0f32,
            asset_name: "USD".to_string(),
            asset_scale: 2,
        };

        let variation_document_two =
            make_variation(&catalog_service, variation_two.clone()).await?;
        check_variation_document(&variation_document_two, &variation_two);

        let min_just_appear_variation_two_query = SqlCatalogQueryOptions {
            limit: None,
            order_by: None,
            options: ListCatalogQueryOptions {
                max_price: None,
                min_price: Some(5000.0f32),
                name: None,
                tags: None,
            },
        };

        let min_appear_variation_two_and_one = SqlCatalogQueryOptions {
            limit: None,
            order_by: Some(OrderBy {
                field: CatalogColumnOrder::CreatedAt,
                direction: Order::Asc,
            }),
            options: ListCatalogQueryOptions {
                max_price: None,
                min_price: Some(2000.0f32),
                name: None,
                tags: None,
            },
        };

        let items_found_variation_two = catalog_service
            .list(
                &CATALOG_ACCOUNT.to_string(),
                &min_just_appear_variation_two_query,
            )
            .await?;

        assert_eq!(items_found_variation_two.len(), 1);
        let document_variation = &items_found_variation_two[0];
        check_variation_document(document_variation, &variation_two);

        let items_found_variation_one_and_two = catalog_service
            .list(
                &CATALOG_ACCOUNT.to_string(),
                &min_appear_variation_two_and_one,
            )
            .await?;

        assert_eq!(items_found_variation_one_and_two.len(), 2);

        let item_one = &items_found_variation_one_and_two[0];
        let item_two = &items_found_variation_one_and_two[1];

        check_variation_document(item_one, &variation);
        check_variation_document(item_two, &variation_two);

        let max_just_appear_variation_one_query = SqlCatalogQueryOptions {
            limit: None,
            order_by: None,
            options: ListCatalogQueryOptions {
                max_price: Some(2000.0f32),
                min_price: None,
                name: None,
                tags: None,
            },
        };

        let max_appear_variation_two_and_one = SqlCatalogQueryOptions {
            limit: None,
            order_by: Some(OrderBy {
                field: CatalogColumnOrder::CreatedAt,
                direction: Order::Asc,
            }),
            options: ListCatalogQueryOptions {
                max_price: Some(5000.0f32),
                min_price: None,
                name: None,
                tags: None,
            },
        };

        let items_found_variation_two = catalog_service
            .list(
                &CATALOG_ACCOUNT.to_string(),
                &max_just_appear_variation_one_query,
            )
            .await?;
        assert_eq!(items_found_variation_two.len(), 1);

        let document_variation = &items_found_variation_two[0];
        check_variation_document(document_variation, &variation);

        let items_found_variation_one_and_two = catalog_service
            .list(
                &CATALOG_ACCOUNT.to_string(),
                &max_appear_variation_two_and_one,
            )
            .await?;
        assert_eq!(items_found_variation_one_and_two.len(), 2);

        let item_one = &items_found_variation_one_and_two[0];
        let item_two = &items_found_variation_one_and_two[1];

        check_variation_document(item_one, &variation);
        check_variation_document(item_two, &variation_two);

        Ok(())
    }

    #[async_std::test]
    async fn list_item_by_tags() -> Result<(), AnyHow> {
        let pool = restore_db().await?;
        let catalog_service = CatalogSQLService::new(pool);
        let doc = make_item(&catalog_service, fake_item()).await?;
        let item = doc.catalog_object.item().unwrap();

        let query_not_exists_tags = SqlCatalogQueryOptions {
            limit: None,
            order_by: None,
            options: ListCatalogQueryOptions {
                max_price: None,
                min_price: None,
                name: None,
                tags: Some(vec!["not-existing".to_string()]),
            },
        };

        let query_tags_exists = SqlCatalogQueryOptions {
            limit: None,
            order_by: None,
            options: ListCatalogQueryOptions {
                max_price: None,
                min_price: None,
                name: None,
                tags: Some(item.tags.clone()),
            },
        };

        let items_empty = catalog_service
            .list(&CATALOG_ACCOUNT.to_string(), &query_not_exists_tags)
            .await?;
        assert_eq!(items_empty.len(), 0);
        let items_found = catalog_service
            .list(&CATALOG_ACCOUNT.to_string(), &query_tags_exists)
            .await?;
        assert_eq!(items_found.len(), 1);
        let item_found = &items_found[0];
        check_item_document(item_found, item);
        Ok(())
    }
}

#[cfg(test)]
pub mod catalog_cmd {
    use super::*;

    #[async_std::test]
    async fn increase_item_in_variations() -> Result<(), AnyHow> {
        let pool = restore_db().await?;
        let catalog_service = CatalogSQLService::new(pool);
        let doc = make_item(&catalog_service, fake_item()).await?;
        let variation = fake_item_variation(doc.id);
        let variation_document = make_variation(&catalog_service, variation.clone()).await?;
        check_variation_document(&variation_document, &variation);

        let command = CatalogCmd::IncreaseItemVariationUnits(IncreaseItemVariationUnitsPayload {
            id: variation_document.id,
            units: 10,
        });

        catalog_service
            .cmd(&CATALOG_ACCOUNT.to_string(), command)
            .await?;

        sleep(Duration::from_secs(2)).await;
        let read_catalog_variation = catalog_service
            .read(&CATALOG_ACCOUNT.to_string(), &variation_document.id)
            .await?;
        let read_variation = as_value!(
            read_catalog_variation.catalog_object,
            CatalogObject::Variation
        )
        .unwrap();
        assert_eq!(
            read_variation.available_units,
            variation.available_units + 10
        );

        let command = CatalogCmd::IncreaseItemVariationUnits(IncreaseItemVariationUnitsPayload {
            id: variation_document.id,
            units: -10,
        });

        catalog_service
            .cmd(&CATALOG_ACCOUNT.to_string(), command)
            .await?;

        sleep(Duration::from_secs(2)).await;
        let read_catalog_variation = catalog_service
            .read(&CATALOG_ACCOUNT.to_string(), &variation_document.id)
            .await?;
        let read_variation = as_value!(
            read_catalog_variation.catalog_object,
            CatalogObject::Variation
        )
        .unwrap();
        assert_eq!(read_variation.available_units, variation.available_units);
        Ok(())
    }
}

// #[cfg(test)]
// pub mod item_delivery {
// use super::*;

// #[async_std::test]
// async fn create_item_variation() -> Result<(), AnyHow> {
//     let pool = restore_db().await?;
//     let catalog_service = CatalogSQLService::new(pool);
//     let item_doc = make_item(&catalog_service, fake_item()).await?;
//     let variation = fake_item_variation(item_doc.id);
//     let variation_doc = make_variation(&catalog_service, variation.clone()).await?;
//     check_variation_document(&variation_doc, &variation);
//     Ok(())
// }
// }

#[cfg(test)]
pub mod bulk {

    use super::*;

    #[async_std::test]
    async fn create_bulk() -> Result<(), AnyHow> {
        let pool = restore_db().await?;
        let catalog_service = CatalogSQLService::new(pool);
        let id_ref = String::from("#item-a");
        let item_a = fake_item();
        let variation_a_for_a = fake_item_variation(id_ref.clone());
        let modification_a_for_a = fake_item_modification(id_ref.clone());
        let control_a_for_a = fake_item_control(id_ref.clone());

        let ItemControl {
            mut control,
            item_id,
        } = control_a_for_a;

        if let Control::Matrix(ref mut control) = control {
            let key = control.key_template.clone().replace(":color", "Red");
            let key = key.replace(":size", "L");
            control.combinations.entry(key).or_insert(id_ref.clone());
        }

        let control_a_for_a = ItemControl { control, item_id };

        let items: Vec<CatalogObjectBulkDocument<String>> = vec![
            CatalogObjectBulkDocument {
                id: Some(id_ref.clone()),
                catalog_object: CatalogObject::Item(item_a.clone()),
            },
            CatalogObjectBulkDocument {
                id: None,
                catalog_object: CatalogObject::Variation(variation_a_for_a.clone()),
            },
            CatalogObjectBulkDocument {
                id: None,
                catalog_object: CatalogObject::Modification(modification_a_for_a.clone()),
            },
            CatalogObjectBulkDocument {
                id: None,
                catalog_object: CatalogObject::Control(control_a_for_a.clone()),
            },
        ];

        let items_docs = catalog_service
            .bulk_create(&CATALOG_ACCOUNT.to_string(), &items)
            .await?;

        for item_doc in items_docs.iter() {
            match &item_doc.catalog_object {
                CatalogObject::Item(_) => {
                    check_item_document(item_doc, &item_a);
                }
                CatalogObject::Modification(_) => {
                    check_modification_document(item_doc, &modification_a_for_a);
                }
                CatalogObject::Variation(_) => {
                    check_variation_document(item_doc, &variation_a_for_a);
                }
                CatalogObject::Control(_) => {
                    check_control_document(item_doc, &control_a_for_a);
                }
                _ => panic!("never should flow through here"),
            }
        }

        Ok(())
    }
}
