mod utils;
mod fixtures;

use utils::{ get_conn, AnyHow, restore_db, check_if_error_is };
use utils::InstanceOf;

use fixtures::catalog:: { fake_item, fake_item_variation };
use async_std::task::sleep;
use std::time::Duration;

use merchant::catalog;
use merchant::utils::query::{ QueryOrderBy, Order };
use merchant::catalog::service::CatalogError;
use catalog::service::{ CatalogService, CatalogCmd, IncreaseItemVariationUnitsPayload, Commander, CatalogColumnOrder };
use catalog::backend::postgres::{ CatalogSQLService, SqlCatalogObjectDocument, SqlCatalogItemVariation, SqlCatalogObject, SqlCatalogQueryOptions };
use catalog::models::{Item, ItemCategory, CatalogObject, Image, ItemMeasurmentUnits};
use sqlx::types::{ Uuid };

const CATALOG_ACCOUNT: &str = "account";

pub fn check_catalog_object_document(catalog: &SqlCatalogObjectDocument) {
  assert!(catalog.version.instance_of::<chrono::NaiveDateTime>(), "it should be an instance of NaiveDateTime");
  assert!(catalog.uuid.instance_of::<Uuid>(), "it should be a instance of sqlx::types::uuid");
  assert!(catalog.account.instance_of::<String>(), "the accoutn property should be an str");
  assert!(catalog.created_at.instance_of::<chrono::NaiveDateTime>());
}

pub fn check_item_document(catalog: &SqlCatalogObjectDocument, item_object: Item) {
  check_catalog_object_document(catalog);
  assert!(matches!(catalog.catalog_object, CatalogObject::Item(_)), "the catalog object should be an item");
  match &catalog.catalog_object {
    CatalogObject::Item(item) => {
      assert!(item.tags.instance_of::<Vec<String>>(), "tags should be a instance of vector");
      assert!(item.name.instance_of::<String>(), "name should be a string");
      assert!(item.description.instance_of::<String>(), "description should be an string");
      assert!(item.category.instance_of::<ItemCategory>(), "description should be an string");
      // item tags
      assert_eq!(item.tags, item_object.tags);
      assert_eq!(item.name, item_object.name);
      assert_eq!(item.description, item_object.description);
      assert!(item.category == item_object.category, "category are distinct");
    },
    _ => panic!("catalog_object should be an item")
  }
}

pub fn check_variation_document(catalog: &SqlCatalogObjectDocument, variation: SqlCatalogItemVariation) {
  check_catalog_object_document(catalog);
  assert!(matches!(catalog.catalog_object, CatalogObject::Variation(_)), "the catalog object should be an Variation");
  match &catalog.catalog_object {
    CatalogObject::Variation(v) => {
      assert!(v.images.instance_of::<Vec<Image>>(), "it should be a vector of images");
      assert!(v.item_uuid.instance_of::<Uuid>(), "it should be an uuid");
      assert!(v.measurement_units.instance_of::<ItemMeasurmentUnits>(), "it should be an uuid");
      assert_eq!(v.images, variation.images);
      assert_eq!(v.item_uuid, variation.item_uuid);
      assert_eq!(v.measurement_units, variation.measurement_units);
      assert_eq!(v.name, variation.name);
      assert_eq!(v.price, variation.price);
      assert_eq!(v.sku, variation.sku);
      assert_eq!(v.available_units, variation.available_units);
      assert_eq!(v.upc, variation.upc);
    },
    _ => panic!("catalog_object should be an item")
  }
}


pub async fn make_item(catalog_service: &CatalogSQLService, item: Box<Item>)-> Result<SqlCatalogObjectDocument, AnyHow> {
  let entry = SqlCatalogObject::Item(*item);
  let catalog_entry_document = catalog_service.create(CATALOG_ACCOUNT.to_string(), &entry).await?;
  Ok(catalog_entry_document)
}


pub async fn make_variation(catalog_service: &CatalogSQLService, variation: Box<SqlCatalogItemVariation>)-> Result<SqlCatalogObjectDocument, CatalogError> {
  let entry = SqlCatalogObject::Variation(*variation);
  let catalog_entry_document = catalog_service.create(CATALOG_ACCOUNT.to_string(), &entry).await?;
  Ok(catalog_entry_document)
}

#[cfg(test)]
pub mod item_test {
  use super::*;

  #[async_std::test]
  async fn create_item() -> Result<(), AnyHow> {
    restore_db().await?;
    let pool = get_conn().await.unwrap();
    let catalog_service = CatalogSQLService::new(Box::new(pool));
    let item = Box::new(fake_item());
    let entry = SqlCatalogObject::Item(*Box::clone(&item));
    let catalog_entry_document = catalog_service.create(CATALOG_ACCOUNT.to_string(), &entry).await?;
    check_item_document(&catalog_entry_document, *item);
    Ok(())
  }


  #[async_std::test]
  async fn update_item() -> Result<(), AnyHow> {
    restore_db().await?;
    let pool = get_conn().await.unwrap();
    let catalog_service = CatalogSQLService::new(Box::new(pool));
    let item_old = Box::new(fake_item());
    let item_new = Box::new(fake_item());
    let catalog_item_document = make_item(&catalog_service, Box::clone(&item_old)).await?;
    check_item_document(&catalog_item_document, *Box::clone(&item_old));
    let updated_catalog_item = catalog_service.update(CATALOG_ACCOUNT.to_string(), catalog_item_document.uuid, &SqlCatalogObject::Item(*Box::clone(&item_new))).await?;
    check_item_document(&updated_catalog_item, *Box::clone(&item_new));
    let item_created = as_value!(updated_catalog_item.catalog_object, SqlCatalogObject::Item).unwrap();
    assert_ne!(item_created.name, item_old.name);
    assert_eq!(item_created.name, item_new.name);
    Ok(())
  }

  #[async_std::test]
  async fn read_item() -> Result<(), AnyHow> {
    restore_db().await?;
    let pool = get_conn().await.unwrap();
    let catalog_service = CatalogSQLService::new(Box::new(pool));
    let item = Box::new(fake_item());

    let catalog_item_document = make_item(&catalog_service, Box::clone(&item)).await?;
    check_item_document(&catalog_item_document, *Box::clone(&item));
    let read_catalog_item = catalog_service.read(CATALOG_ACCOUNT.to_string(), catalog_item_document.uuid).await?;
    check_item_document(&read_catalog_item, *Box::clone(&item));
    Ok(())
  }
}


#[async_std::test]
async fn check_if_exists() -> Result<(), AnyHow> {
  restore_db().await?;
  let pool = get_conn().await.unwrap();
  let catalog_service = CatalogSQLService::new(Box::new(pool));
  let item = Box::new(fake_item());
  let catalog_item_document = make_item(&catalog_service, Box::clone(&item)).await?;
  assert!(catalog_service.exists(CATALOG_ACCOUNT.to_string(), catalog_item_document.uuid).await?, "it should exists");
  assert!(!catalog_service.exists(CATALOG_ACCOUNT.to_string(), sqlx::types::uuid::Uuid::new_v4()).await?, "it should not exists");
  Ok(())
}


#[async_std::test]
async fn read_item_fails_if_uuid_doesnt_exists() -> Result<(), AnyHow> {
  restore_db().await?;
  let pool = get_conn().await.unwrap();
  let catalog_service = CatalogSQLService::new(Box::new(pool));
  let uuid = Uuid::new_v4();
  let read_catalog_item = catalog_service.read(CATALOG_ACCOUNT.to_string(), uuid).await;
  check_if_error_is(read_catalog_item.unwrap_err(), CatalogError::CatalogEntryNotFound(uuid.to_string()));
  Ok(())
}

#[cfg(test)]
pub mod item_variation_test {
  use super::*;

  #[async_std::test]
  async fn create_item_variation() -> Result<(), AnyHow> {
    restore_db().await?;
    let pool = get_conn().await.unwrap();
    let catalog_service = CatalogSQLService::new(Box::new(pool));
    let item = Box::new(fake_item());
    let catalog_item_document = make_item(&catalog_service, item).await?;
    let variation = Box::new(fake_item_variation(catalog_item_document.uuid));
    let catalog_variation_document = make_variation(&catalog_service, Box::clone(&variation)).await?;
    check_variation_document(&catalog_variation_document, *variation);
    Ok(())
  }

  #[async_std::test]
  async fn create_item_variation_fails_if_not_exists_item_uuid() -> Result<(), AnyHow> {
    restore_db().await?;
    let pool = get_conn().await.unwrap();
    let catalog_service = CatalogSQLService::new(Box::new(pool));
    let variation = Box::new(fake_item_variation(sqlx::types::uuid::Uuid::new_v4()));
    let result = make_variation(&catalog_service, Box::clone(&variation)).await;
    check_if_error_is(result.unwrap_err(), CatalogError::CatalogBadRequest);
    Ok(())
  }
  #[async_std::test]
  async fn update_variation() -> Result<(), AnyHow> {
    restore_db().await?;
    let pool = get_conn().await.unwrap();
    let catalog_service = CatalogSQLService::new(Box::new(pool));
    let item = Box::new(fake_item());
    let catalog_item_document = make_item(&catalog_service, item).await?;
    let variation = Box::new(fake_item_variation(catalog_item_document.uuid));
    let variation_new = Box::new(fake_item_variation(catalog_item_document.uuid));
    let catalog_variation_document = make_variation(&catalog_service, Box::clone(&variation)).await?;
    check_variation_document(&catalog_variation_document, *Box::clone(&variation));
    let updated_catalog_variation = catalog_service.update(CATALOG_ACCOUNT.to_string(), catalog_variation_document.uuid, &SqlCatalogObject::Variation(*Box::clone(&variation_new))).await?;
    check_variation_document(&updated_catalog_variation, *Box::clone(&variation_new));
    let variation_updated = as_value!(updated_catalog_variation.catalog_object, SqlCatalogObject::Variation).unwrap();
    assert_ne!(variation_updated.name, variation.name);
    assert_eq!(variation_updated.name, variation_new.name);
    Ok(())
  }

  #[async_std::test]
  async fn update_variation_fails_if_not_exists_item_uuid() -> Result<(), AnyHow> {
    restore_db().await?;
    let pool = get_conn().await.unwrap();
    let catalog_service = CatalogSQLService::new(Box::new(pool));
    let item = Box::new(fake_item());
    let catalog_item_document = make_item(&catalog_service, item).await?;
    let variation = Box::new(fake_item_variation(catalog_item_document.uuid));
    let variation_new = Box::new(fake_item_variation(Uuid::new_v4()));
    let catalog_variation_document = make_variation(&catalog_service, Box::clone(&variation)).await?;
    check_variation_document(&catalog_variation_document, *Box::clone(&variation));
    let result = catalog_service.update(CATALOG_ACCOUNT.to_string(), catalog_variation_document.uuid, &SqlCatalogObject::Variation(*Box::clone(&variation_new))).await;
    check_if_error_is(result.unwrap_err(), CatalogError::CatalogBadRequest);
    Ok(())
  }

  #[async_std::test]
  async fn read_item() -> Result<(), AnyHow> {
    restore_db().await?;
    let pool = get_conn().await.unwrap();
    let catalog_service = CatalogSQLService::new(Box::new(pool));
    let item = Box::new(fake_item());
    let catalog_item_document = make_item(&catalog_service, item).await?;
    let variation = Box::new(fake_item_variation(catalog_item_document.uuid));
    let catalog_variation_document = make_variation(&catalog_service, Box::clone(&variation)).await?;
    check_variation_document(&catalog_variation_document, *Box::clone(&variation));
    let read_catalog_variation = catalog_service.read(CATALOG_ACCOUNT.to_string(), catalog_variation_document.uuid).await?;
    check_variation_document(&read_catalog_variation, *Box::clone(&variation));
    Ok(())
  }
}


#[cfg(test)]
pub mod item_find_test {
  use merchant::catalog::{service::{ListCatalogQueryOptions}, models::Price};
  use super::*;

  #[async_std::test]
  async fn list_item_by_name() -> Result<(), AnyHow> {
    restore_db().await?;
    let pool = get_conn().await.unwrap();
    let catalog_service = CatalogSQLService::new(Box::new(pool));
    let item = Box::new(fake_item());
    make_item(&catalog_service, Box::clone(&item)).await?;

    let query_name_not_exists = SqlCatalogQueryOptions {
      limit: None,
      order_by: None,
      options: ListCatalogQueryOptions {
        max_price: None,
        min_price: None,
        name: Some("None".to_string()),
        tags: None,
      }
    };

    let query_name_exists = SqlCatalogQueryOptions {
      limit: None,
      order_by: None,
      options: ListCatalogQueryOptions {
        max_price: None,
        min_price: None,
        name: Some(Box::clone(&item).name),
        tags: None,
      }
    };

    let items_empty = catalog_service.list(CATALOG_ACCOUNT.to_string(), &query_name_not_exists).await?;
    assert_eq!(items_empty.len(), 0);
    let items_found = catalog_service.list(CATALOG_ACCOUNT.to_string(), &query_name_exists).await?;
    assert_eq!(items_found.len(), 1);
    let item_found = &items_found[0];
    check_item_document(item_found, *Box::clone(&item));
    Ok(())
  }

  #[async_std::test]
  async fn list_item_by_min_and_max_amount() -> Result<(), AnyHow> {
    restore_db().await?;
    let pool = get_conn().await.unwrap();
    let catalog_service = CatalogSQLService::new(Box::new(pool));
    let item = Box::new(fake_item());
    let item_document = make_item(&catalog_service, Box::clone(&item)).await?;
    let mut variation = Box::new(fake_item_variation(item_document.uuid));    
    variation.price = Price::Fixed {
      amount: 2000.0f32,
      currency: "USD".to_string(),
    };

    let variation_document = make_variation(&catalog_service, Box::clone(&variation)).await?;
    let mut variation_two = Box::new(fake_item_variation(item_document.uuid));
    check_variation_document(&variation_document, *Box::clone(&variation));

    variation_two.price = Price::Fixed {
      amount: 5000.0f32,
      currency: "USD".to_string(),
    };

    let variation_document_two = make_variation(&catalog_service, Box::clone(&variation_two)).await?;
    check_variation_document(&variation_document_two, *Box::clone(&variation_two));

    let min_just_appear_variation_two_query = SqlCatalogQueryOptions {
      limit: None,
      order_by: None,
      options: ListCatalogQueryOptions {
        max_price: None,
        min_price: Some(5000.0f32),
        name: None,
        tags: None,
      }
    };

    let min_appear_variation_two_and_one = SqlCatalogQueryOptions {
      limit: None,
      order_by: Some(QueryOrderBy {
        column: CatalogColumnOrder::CreatedAt,
        direction: Order::Asc,
      }),
      options: ListCatalogQueryOptions {
        max_price: None,
        min_price: Some(2000.0f32),
        name: None,
        tags: None,
      }
    };

    let items_found_variation_two = catalog_service.list(CATALOG_ACCOUNT.to_string(), &min_just_appear_variation_two_query).await?;

    assert_eq!(items_found_variation_two.len(), 1);
    let document_variation = &items_found_variation_two[0];
    check_variation_document(document_variation, *Box::clone(&variation_two));

    let items_found_variation_one_and_two = catalog_service.list(CATALOG_ACCOUNT.to_string(), &min_appear_variation_two_and_one).await?;

    assert_eq!(items_found_variation_one_and_two.len(), 2);

    let item_one = &items_found_variation_one_and_two[0];
    let item_two = &items_found_variation_one_and_two[1];

    check_variation_document(item_one, *Box::clone(&variation));
    check_variation_document(item_two, *Box::clone(&variation_two));

    let max_just_appear_variation_one_query = SqlCatalogQueryOptions {
      limit: None,
      order_by: None,
      options: ListCatalogQueryOptions {
        max_price: Some(2000.0f32),
        min_price: None,
        name: None,
        tags: None,
      }
    };

    let max_appear_variation_two_and_one = SqlCatalogQueryOptions {
      limit: None,
      order_by: Some(QueryOrderBy {
        column: CatalogColumnOrder::CreatedAt,
        direction: Order::Asc,
      }),
      options: ListCatalogQueryOptions {
        max_price: Some(5000.0f32),
        min_price: None,
        name: None,
        tags: None,
      }
    };

    let items_found_variation_two = catalog_service.list(CATALOG_ACCOUNT.to_string(), &max_just_appear_variation_one_query).await?;
    assert_eq!(items_found_variation_two.len(), 1);

    let document_variation = &items_found_variation_two[0];
    check_variation_document(document_variation, *Box::clone(&variation));

    let items_found_variation_one_and_two = catalog_service.list(CATALOG_ACCOUNT.to_string(), &max_appear_variation_two_and_one).await?;
    assert_eq!(items_found_variation_one_and_two.len(), 2);

    let item_one = &items_found_variation_one_and_two[0];
    let item_two = &items_found_variation_one_and_two[1];

    check_variation_document(item_one, *Box::clone(&variation));
    check_variation_document(item_two, *Box::clone(&variation_two));
    
    Ok(())
  }


  #[async_std::test]
  async fn list_item_by_tags() -> Result<(), AnyHow> {
    restore_db().await?;
    let pool = get_conn().await.unwrap();
    let catalog_service = CatalogSQLService::new(Box::new(pool));
    let item = Box::new(fake_item());
    make_item(&catalog_service, Box::clone(&item)).await?;

    let query_not_exists_tags = SqlCatalogQueryOptions {
      limit: None,
      order_by: None,
      options: ListCatalogQueryOptions {
        max_price: None,
        min_price: None,
        name: None,
        tags: Some(vec!["not-exist-this-thing".to_string()]),
      }
    };

    let query_tags_exists = SqlCatalogQueryOptions {
      limit: None,
      order_by: None,
      options: ListCatalogQueryOptions {
        max_price: None,
        min_price: None,
        name: None,
        tags: Some(Box::clone(&item).tags),
      }
    };

    let items_empty = catalog_service.list(CATALOG_ACCOUNT.to_string(), &query_not_exists_tags).await?;
    assert_eq!(items_empty.len(), 0);
    let items_found = catalog_service.list(CATALOG_ACCOUNT.to_string(), &query_tags_exists).await?;
    assert_eq!(items_found.len(), 1);
    let item_found = &items_found[0];
    check_item_document(item_found, *Box::clone(&item));
    Ok(())
  }
}




#[cfg(test)]
pub mod catalog_cmd {
  use merchant::catalog::{service::{ListCatalogQueryOptions}, models::Price, backend::postgres::SqlUuid};
  use super::*;

  #[async_std::test]
  async fn increase_item_in_variations() -> Result<(), AnyHow> {
    restore_db().await?;
    let pool = get_conn().await.unwrap();
    let catalog_service = CatalogSQLService::new(Box::new(pool));
    let item = Box::new(fake_item());
    let item_document = make_item(&catalog_service, Box::clone(&item)).await?;
    let variation = Box::new(fake_item_variation(item_document.uuid));
    let mut variation_document = make_variation(&catalog_service, Box::clone(&variation)).await?;
    check_variation_document(&variation_document, *Box::clone(&variation));
    let command =  CatalogCmd::IncreaseItemVariationUnits(IncreaseItemVariationUnitsPayload {
      uuid: variation_document.uuid,
      units: 10,
    });

    catalog_service.cmd(
      CATALOG_ACCOUNT.to_string(),
      command
    ).await?;

    sleep(Duration::from_secs(2)).await;
    let read_catalog_variation = catalog_service.read(CATALOG_ACCOUNT.to_string(), variation_document.uuid).await?;
    let read_variation = as_value!(read_catalog_variation.catalog_object, CatalogObject::Variation).unwrap();
    assert_eq!(read_variation.available_units, variation.available_units + 10);

    let command =  CatalogCmd::IncreaseItemVariationUnits(IncreaseItemVariationUnitsPayload {
      uuid: variation_document.uuid,
      units: -10,
    });

    catalog_service.cmd(
      CATALOG_ACCOUNT.to_string(),
      command
    ).await?;

    sleep(Duration::from_secs(2)).await;
    let read_catalog_variation = catalog_service.read(CATALOG_ACCOUNT.to_string(), variation_document.uuid).await?;
    let read_variation = as_value!(read_catalog_variation.catalog_object, CatalogObject::Variation).unwrap();
    assert_eq!(read_variation.available_units, variation.available_units);
    Ok(())
  }
}