mod fixtures;
mod utils;

use utils::{new_context, AnyHow};
use utils::{send, InstanceOf};

use async_std::task::sleep;
use fixtures::catalog::{fake_item, fake_item_variation};
use std::time::Duration;

use catalog::{
    CatalogObject, CatalogObjectDocument, Image, IncreaseItemVariationUnitsPayload, Item,
    ItemCategory, ItemMeasurmentUnits, ItemVariation, OrderField,
};
use merchant::catalog::CatalogError;
use merchant::{catalog, Context, Error, Msg};

type Id = u32;

pub fn check_item_document(catalog: &CatalogObjectDocument, item_object: &Item) {
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

pub fn check_variation_document(catalog: &CatalogObjectDocument, variation: &ItemVariation) {
    match &catalog.catalog_object {
        CatalogObject::Variation(v) => {
            assert!(
                v.images.instance_of::<Vec<Image>>(),
                "it should be a vector of images"
            );
            assert!(v.item_id.instance_of::<Id>(), "it should be an id");
            assert!(
                v.measurement_units.instance_of::<ItemMeasurmentUnits>(),
                "it should be an id"
            );
            assert_eq!(v.images, variation.images);
            assert_eq!(v.item_id, variation.item_id);
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

async fn make(
    cx: &mut Context,
    item: impl Into<CatalogObject>,
) -> Result<CatalogObjectDocument, Error> {
    Ok(send(cx, Msg::CatalogCreate(item.into())).await?)
}

#[cfg(test)]
pub mod item_test {
    use merchant::Error;

    use super::*;

    #[async_std::test]
    async fn create_item() -> Result<(), AnyHow> {
        let mut cx = new_context().await?;
        let item = fake_item();
        let doc: CatalogObjectDocument =
            send(&mut cx, Msg::CatalogCreate(item.clone().into())).await?;
        check_item_document(&doc, &item);
        Ok(())
    }

    #[async_std::test]
    async fn update_item() -> Result<(), AnyHow> {
        let mut cx = new_context().await?;
        let item_old = fake_item();
        let item_new = fake_item();

        let item_doc = make(&mut cx, item_old.clone()).await?;
        check_item_document(&item_doc, &item_old);

        let updated_item: CatalogObjectDocument = send(
            &mut cx,
            Msg::CatalogUpdate(item_doc.id, item_new.clone().into()),
        )
        .await?;
        check_item_document(&updated_item, &item_new);

        let item_created = as_value!(updated_item.catalog_object, CatalogObject::Item).unwrap();
        assert_ne!(item_created.name, item_old.name);
        assert_eq!(item_created.name, item_new.name);
        Ok(())
    }

    #[async_std::test]
    async fn query_item() -> Result<(), AnyHow> {
        let mut cx = new_context().await?;

        let item = fake_item();
        let item_doc = make(&mut cx, item.clone()).await?;
        check_item_document(&item_doc, &item);

        let read_catalog_item: CatalogObjectDocument = send(&mut cx, item_doc.id).await?;
        check_item_document(&read_catalog_item, &item);
        Ok(())
    }

    #[async_std::test]
    async fn query_nonexistent_item() -> Result<(), AnyHow> {
        let mut cx = new_context().await?;

        let id = Id::default();
        let read_catalog_err: Result<(), Error> = send(&mut cx, id).await;
        assert!(matches!(
            read_catalog_err.unwrap_err(),
            Error::Catalog(CatalogError::CatalogEntryNotFound(_)),
        ));
        Ok(())
    }
}

#[cfg(test)]
pub mod item_variation_test {
    use super::*;

    #[async_std::test]
    async fn create_item_variation() -> Result<(), AnyHow> {
        let mut cx = new_context().await?;
        let item_doc = make(&mut cx, fake_item()).await?;
        let variation = fake_item_variation(item_doc.id);
        let variation_doc = make(&mut cx, variation.clone()).await?;
        check_variation_document(&variation_doc, &variation);
        Ok(())
    }

    #[async_std::test]
    async fn create_item_variation_fails_if_not_exists_item_id() -> Result<(), AnyHow> {
        let mut cx = new_context().await?;
        let variation = fake_item_variation(Id::default());
        let result = make(&mut cx, variation).await;
        assert!(matches!(
            result.unwrap_err(),
            Error::Catalog(CatalogError::CatalogBadRequest)
        ));
        Ok(())
    }

    #[async_std::test]
    async fn update_variation() -> Result<(), AnyHow> {
        let mut cx = new_context().await?;
        let item = fake_item();
        let item_doc = make(&mut cx, item).await?;
        let variation = fake_item_variation(item_doc.id);
        let variation_new = fake_item_variation(item_doc.id);
        let variation_doc = make(&mut cx, variation.clone()).await?;
        check_variation_document(&variation_doc, &variation);

        let updated_variation: CatalogObjectDocument = send(
            &mut cx,
            Msg::CatalogUpdate(variation_doc.id, variation_new.clone().into()),
        )
        .await?;
        check_variation_document(&updated_variation, &variation_new);
        let variation_updated =
            as_value!(updated_variation.catalog_object, CatalogObject::Variation).unwrap();
        assert_ne!(variation_updated.name, variation.name);
        assert_eq!(variation_updated.name, variation_new.name);
        Ok(())
    }

    #[async_std::test]
    async fn update_variation_fails_if_not_exists_item_id() -> Result<(), AnyHow> {
        let mut cx = new_context().await?;
        let catalog_item_document = make(&mut cx, fake_item()).await?;
        let variation = fake_item_variation(catalog_item_document.id);
        let variation_new = fake_item_variation(Id::default());
        let variation_doc = make(&mut cx, variation.clone()).await?;
        check_variation_document(&variation_doc, &variation);

        let result: Result<(), Error> = send(
            &mut cx,
            Msg::CatalogUpdate(variation_doc.id, variation_new.into()),
        )
        .await;
        assert!(matches!(
            result.unwrap_err(),
            Error::Catalog(CatalogError::CatalogBadRequest)
        ));
        Ok(())
    }

    #[async_std::test]
    async fn read_item() -> Result<(), AnyHow> {
        let mut cx = new_context().await?;
        let item_doc = make(&mut cx, fake_item()).await?;
        let variation = fake_item_variation(item_doc.id);
        let variation_doc = make(&mut cx, variation.clone()).await?;
        check_variation_document(&variation_doc, &variation);

        let read_catalog_variation: CatalogObjectDocument = send(&mut cx, variation_doc.id).await?;
        check_variation_document(&read_catalog_variation, &variation);
        Ok(())
    }
}

#[cfg(test)]
pub mod item_find_test {
    use super::*;
    use common::query::{Order, OrderBy, Query};
    use merchant::catalog::{Price, QueryOptions};

    #[async_std::test]
    async fn list_item_by_name() -> Result<(), AnyHow> {
        let mut cx = new_context().await?;
        let doc = make(&mut cx, fake_item()).await?;
        let item = doc.catalog_object.item().unwrap();

        let query_name_not_exists = Query {
            limit: None,
            order_by: None,
            options: QueryOptions {
                max_price: None,
                min_price: None,
                name: Some("None".to_string()),
                tags: None,
            },
        };

        let query_name_exists = Query {
            limit: None,
            order_by: None,
            options: QueryOptions {
                max_price: None,
                min_price: None,
                name: Some(item.name.clone()),
                tags: None,
            },
        };

        let items_empty: Vec<CatalogObjectDocument> = send(&mut cx, query_name_not_exists).await?;
        assert_eq!(items_empty.len(), 0);
        let items_found: Vec<CatalogObjectDocument> = send(&mut cx, query_name_exists).await?;
        assert_eq!(items_found.len(), 1);
        let item_found = &items_found[0];
        check_item_document(item_found, &item);
        Ok(())
    }

    #[async_std::test]
    async fn list_item_by_min_and_max_amount() -> Result<(), AnyHow> {
        let mut cx = new_context().await?;
        let item_doc = make(&mut cx, fake_item()).await?;
        let mut variation = fake_item_variation(item_doc.id);
        variation.price = Price::Fixed {
            amount: 2000.0f32,
            currency: "USD".to_string(),
        };

        let variation_document = make(&mut cx, variation.clone()).await?;
        let mut variation_two = fake_item_variation(item_doc.id);
        check_variation_document(&variation_document, &variation);

        variation_two.price = Price::Fixed {
            amount: 5000.0f32,
            currency: "USD".to_string(),
        };

        let variation_document_two = make(&mut cx, variation_two.clone()).await?;
        check_variation_document(&variation_document_two, &variation_two);

        let min_just_appear_variation_two_query = Query {
            limit: None,
            order_by: None,
            options: QueryOptions {
                max_price: None,
                min_price: Some(5000.0f32),
                name: None,
                tags: None,
            },
        };

        let min_appear_variation_two_and_one = Query {
            limit: None,
            order_by: Some(OrderBy {
                field: OrderField::CreatedAt,
                direction: Order::Asc,
            }),
            options: QueryOptions {
                max_price: None,
                min_price: Some(2000.0f32),
                name: None,
                tags: None,
            },
        };

        let items_found_variation_two: Vec<CatalogObjectDocument> =
            send(&mut cx, min_just_appear_variation_two_query).await?;

        assert_eq!(items_found_variation_two.len(), 1);
        let document_variation = &items_found_variation_two[0];
        check_variation_document(document_variation, &variation_two);

        let items_found_variation_one_and_two: Vec<CatalogObjectDocument> =
            send(&mut cx, min_appear_variation_two_and_one).await?;
        assert_eq!(items_found_variation_one_and_two.len(), 2);

        let item_one = &items_found_variation_one_and_two[0];
        let item_two = &items_found_variation_one_and_two[1];

        check_variation_document(item_one, &variation);
        check_variation_document(item_two, &variation_two);

        let max_just_appear_variation_one_query = Query {
            limit: None,
            order_by: None,
            options: QueryOptions {
                max_price: Some(2000.0f32),
                min_price: None,
                name: None,
                tags: None,
            },
        };

        let max_appear_variation_two_and_one = Query {
            limit: None,
            order_by: Some(OrderBy {
                field: OrderField::CreatedAt,
                direction: Order::Asc,
            }),
            options: QueryOptions {
                max_price: Some(5000.0f32),
                min_price: None,
                name: None,
                tags: None,
            },
        };

        let items_found_variation_two: Vec<CatalogObjectDocument> =
            send(&mut cx, max_just_appear_variation_one_query).await?;
        assert_eq!(items_found_variation_two.len(), 1);

        let document_variation = &items_found_variation_two[0];
        check_variation_document(document_variation, &variation);

        let items_found_variation_one_and_two: Vec<CatalogObjectDocument> =
            send(&mut cx, max_appear_variation_two_and_one).await?;
        assert_eq!(items_found_variation_one_and_two.len(), 2);

        let item_one = &items_found_variation_one_and_two[0];
        let item_two = &items_found_variation_one_and_two[1];

        check_variation_document(item_one, &variation);
        check_variation_document(item_two, &variation_two);

        Ok(())
    }

    #[async_std::test]
    async fn list_item_by_tags() -> Result<(), AnyHow> {
        let mut cx = new_context().await?;
        let doc = make(&mut cx, fake_item()).await?;
        let item = doc.catalog_object.item().unwrap();

        let query_not_exists_tags = Query {
            limit: None,
            order_by: None,
            options: QueryOptions {
                max_price: None,
                min_price: None,
                name: None,
                tags: Some(vec!["not-existing".to_string()]),
            },
        };

        let query_tags_exists = Query {
            limit: None,
            order_by: None,
            options: QueryOptions {
                max_price: None,
                min_price: None,
                name: None,
                tags: Some(item.tags.clone()),
            },
        };

        let items_empty: Vec<CatalogObjectDocument> = send(&mut cx, query_not_exists_tags).await?;
        assert_eq!(items_empty.len(), 0);

        let items_found: Vec<CatalogObjectDocument> = send(&mut cx, query_tags_exists).await?;
        assert_eq!(items_found.len(), 1);
        let item_found = &items_found[0];
        check_item_document(item_found, &item);
        Ok(())
    }
}

#[cfg(test)]
pub mod catalog_cmd {
    use super::*;

    #[async_std::test]
    async fn increase_item_in_variations() -> Result<(), AnyHow> {
        let mut cx = new_context().await?;
        let doc = make(&mut cx, fake_item()).await?;
        let variation = fake_item_variation(doc.id);
        let variation_document = make(&mut cx, variation.clone()).await?;
        check_variation_document(&variation_document, &variation);

        let command = IncreaseItemVariationUnitsPayload {
            id: variation_document.id,
            units: 10,
        };

        send(&mut cx, command).await?;

        sleep(Duration::from_secs(2)).await;

        let read_catalog_variation: CatalogObjectDocument =
            send(&mut cx, variation_document.id).await?;
        let read_variation = as_value!(
            read_catalog_variation.catalog_object,
            CatalogObject::Variation
        )
        .unwrap();
        assert_eq!(
            read_variation.available_units,
            variation.available_units + 10
        );

        let command = IncreaseItemVariationUnitsPayload {
            id: variation_document.id,
            units: -10,
        };

        send(&mut cx, command).await?;

        sleep(Duration::from_secs(1)).await;
        let read_catalog_variation: CatalogObjectDocument =
            send(&mut cx, variation_document.id).await?;
        let read_variation = as_value!(
            read_catalog_variation.catalog_object,
            CatalogObject::Variation
        )
        .unwrap();
        assert_eq!(read_variation.available_units, variation.available_units);
        Ok(())
    }
}
