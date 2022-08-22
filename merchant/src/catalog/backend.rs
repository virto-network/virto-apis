use std::borrow::Borrow;
use std::collections::HashMap;

use async_trait::async_trait;

use sea_query::{Cond, Expr, Iden, Query as Qsql, SqliteQueryBuilder as QueryBuilder};
use serde::{Deserialize, Serialize};
use sqlx::types::chrono::NaiveDateTime;
use sqlx::{types::Json, FromRow, SqlitePool as Pool};

use super::super::utils::query::{Order, Query};
use super::models::{
    CatalogObject, CatalogObjectBulkDocument, CatalogObjectDocument, Control, Item, ItemControl,
    ItemDelivery, ItemModification, ItemVariation, MatrixControl,
};
use super::service::{
    BulkDocumentReferencesResolver, CatalogCmd, CatalogError, CatalogId, CatalogService, Commander,
    ListCatalogQueryOptions,
};
use crate::catalog::service::{CatalogColumnOrder, IncreaseItemVariationUnitsPayload};
use sea_query::Order as OrderSql;

sea_query::sea_query_driver_sqlite!();
use sea_query_driver_sqlite::{bind_query, bind_query_as};

pub type Id = u32;
pub type Account = String;
pub type SQlCatalogCmd = CatalogCmd<Id>;
pub type SqlCatalogObject = CatalogObject<Id>;
pub type SqlCatalogObjectDocument = CatalogObjectDocument<Id, Account>;
#[allow(dead_code)]
pub type SqlCatalogItemVariation = ItemVariation<Id>;
#[allow(dead_code)]
pub type SqlCatalogObjectBulkDocument = CatalogObjectBulkDocument<Id>;
pub type SqlCatalogQueryOptions = Query<ListCatalogQueryOptions, CatalogColumnOrder>;

impl From<Order> for OrderSql {
    fn from(order_service: Order) -> Self {
        match order_service {
            Order::Asc => OrderSql::Asc,
            Order::Desc => OrderSql::Desc,
        }
    }
}

impl BulkDocumentReferencesResolver for CatalogSQLService {
    type Id = Id;
    fn resolve(
        id_map: &HashMap<&str, Self::Id>,
        catalog: &CatalogObject<String>,
    ) -> Result<CatalogObject<Self::Id>, CatalogError> {
        let item = match &catalog {
            CatalogObject::Item(item) => CatalogObject::Item(item.clone()),
            CatalogObject::Modification(ItemModification {
                processing_time,
                warranty_time,
                enabled,
                images,
                name,
                price,
                item_id,
            }) => CatalogObject::Modification(ItemModification {
                processing_time: processing_time.to_owned(),
                warranty_time: warranty_time.to_owned(),
                enabled: enabled.to_owned(),
                images: images.to_owned(),
                item_id: id_map
                    .get(item_id.as_str())
                    .ok_or(CatalogError::BulkReferenceNotExist(item_id.to_string()))?
                    .clone(),
                name: name.to_owned(),
                price: price.to_owned(),
            }),
            CatalogObject::Variation(ItemVariation {
                available_units,
                enabled,
                processing_time,
                extra_attributes,
                images,
                measurement_units,
                name,
                price,
                sku,
                upc,
                item_id,
            }) => CatalogObject::Variation(ItemVariation {
                available_units: available_units.to_owned(),
                enabled: enabled.to_owned(),
                images: images.to_owned(),
                item_id: id_map
                    .get(item_id.as_str())
                    .ok_or(CatalogError::BulkReferenceNotExist(item_id.to_string()))?
                    .clone(),
                processing_time: processing_time.to_owned(),
                extra_attributes: extra_attributes.to_owned(),
                measurement_units: measurement_units.to_owned(),
                name: name.to_owned(),
                price: price.to_owned(),
                sku: sku.to_owned(),
                upc: upc.to_owned(),
            }),
            CatalogObject::Control(ItemControl { control, item_id }) => {
                let control = match control {
                    Control::Matrix(item) => {
                        let mut combinations: HashMap<String, Self::Id> = HashMap::new();

                        for (template_id, id_ref) in item.combinations.iter() {
                            let id = id_map
                                .get(id_ref.as_str())
                                .clone()
                                .ok_or(CatalogError::BulkReferenceNotExist(id_ref.to_string()))?;

                            combinations
                                .entry(template_id.to_string())
                                .or_insert(id.to_owned());
                        }

                        Control::Matrix(MatrixControl {
                            combinations,
                            props: item.props.to_owned(),
                            key_template: item.key_template.to_owned(),
                        })
                    }
                    Control::Form(item) => Control::Form(item.to_vec()),
                };
                CatalogObject::Control(ItemControl {
                    control,
                    item_id: id_map
                        .get(item_id.as_str())
                        .ok_or(CatalogError::BulkReferenceNotExist(item_id.to_string()))?
                        .clone(),
                })
            }
            CatalogObject::Delivery(ItemDelivery { delivery, item_id }) => {
                CatalogObject::Delivery(ItemDelivery {
                    delivery: delivery.to_owned(),
                    item_id: id_map
                        .get(item_id.as_str())
                        .ok_or(CatalogError::BulkReferenceNotExist(item_id.to_string()))?
                        .clone(),
                })
            }
        };
        Ok(item)
    }
}
#[derive(Clone)]
pub struct CatalogSQLService {
    pool: Pool,
}

impl CatalogSQLService {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    fn get_sql_to_create(
        &self,
        field_data_name: CatalogSchema,
        object_type: &CatalogObject<Id>,
    ) -> String {
        let (sql, _) = Qsql::insert()
            .into_table(CatalogSchema::Table)
            .columns(vec![
                CatalogSchema::Id,
                CatalogSchema::TypeEntry,
                CatalogSchema::Account,
                field_data_name,
            ])
            .exprs_panic(vec![
                Expr::value("$1"),
                Expr::cust(format!("'{}'", object_type).as_ref()),
                Expr::value("$3"),
                Expr::value("$4"),
            ])
            .returning(Qsql::select().expr(Expr::asterisk()).take())
            .build(QueryBuilder);
        sql
    }

    fn get_sql_to_read(&self) -> String {
        let (sql, _) = Qsql::select()
            .expr(Expr::asterisk())
            .from(CatalogSchema::Table)
            .and_where(Expr::col(CatalogSchema::Id).eq("-1"))
            .build(QueryBuilder);

        sql
    }

    fn get_sql_to_exists(&self) -> String {
        let (sql, _) = Qsql::select()
            .expr(Expr::cust("COUNT(1) as count"))
            .from(CatalogSchema::Table)
            .and_where(Expr::col(CatalogSchema::Id).eq("-1"))
            .and_where(Expr::col(CatalogSchema::Account).eq("-1"))
            .build(QueryBuilder);

        println!("sql {:?}", sql);
        sql
    }

    fn get_sql_to_update(&self, field: CatalogSchema, type_entry: &str) -> String {
        let (sql, _) = Qsql::update()
            .table(CatalogSchema::Table)
            //.value_expr(CatalogSchema::Version, Expr::cust("now()"))
            .value(field, "-1".into())
            .and_where(Expr::col(CatalogSchema::Account).eq("1"))
            .and_where(Expr::cust(
                format!(
                    "{} = '{}'",
                    CatalogSchema::TypeEntry.to_string(),
                    type_entry
                )
                .as_ref(),
            ))
            .and_where(Expr::cust_with_values("id = ?", vec!["-1"]))
            .returning(Qsql::select().expr(Expr::asterisk()).take())
            .build(QueryBuilder);

        sql
    }

    fn to_catalog_schema(item: &CatalogObject<Id>) -> CatalogSchema {
        match item {
            CatalogObject::Control(_) => CatalogSchema::ItemControlData,
            CatalogObject::Variation(_) => CatalogSchema::ItemVariationData,
            CatalogObject::Item(_) => CatalogSchema::ItemData,
            CatalogObject::Delivery(_) => CatalogSchema::ItemDeliveryData,
            CatalogObject::Modification(_) => CatalogSchema::ItemModificationData,
        }
    }
}

#[async_trait]
impl CatalogService for CatalogSQLService {
    type Id = Id;
    type Query = SqlCatalogQueryOptions;

    async fn create(
        &self,
        account: &Account,
        catalog_entry: &CatalogObject<Id>,
    ) -> Result<SqlCatalogObjectDocument, CatalogError> {
        let (data, sql) = match catalog_entry {
            item @ CatalogObject::Item(entry) => {
                let data = serde_json::to_value(entry).map_err(|_| CatalogError::MappingError)?;
                let sql = self.get_sql_to_create(CatalogSchema::ItemData, item);

                (data, sql)
            }
            variation @ CatalogObject::Variation(ItemVariation { item_id, .. })
            | variation @ CatalogObject::Modification(ItemModification { item_id, .. })
            | variation @ CatalogObject::Control(ItemControl { item_id, .. })
            | variation @ CatalogObject::Delivery(ItemDelivery { item_id, .. }) => {
                // Here we get {type, data} json value
                let data =
                    serde_json::to_value(variation).map_err(|_| CatalogError::MappingError)?;
                // we extract the data
                let data = data
                    .get("data")
                    .ok_or(CatalogError::MappingError)?
                    .to_owned();

                let sql = self
                    .get_sql_to_create(CatalogSQLService::to_catalog_schema(variation), variation);

                if !self.exists(account, item_id).await? {
                    println!(
                        "the catalog id doesnt not exist, {:?}, A:{}, Id:{}",
                        variation, account, item_id
                    );

                    return Err(CatalogError::CatalogBadRequest);
                }

                (data, sql)
            }
        };

        println!("SQL_TO_CREATE{:?}", sql);

        let mut pool = self
            .pool
            .acquire()
            .await
            .map_err(|_| CatalogError::DatabaseError)?;

        let result: CatalogObjectRow = sqlx::query_as(sql.as_str())
            .bind(rand::random::<Id>())
            .bind(account)
            .bind(Json(data))
            .fetch_one(&mut pool)
            .await
            .map_err(|_| CatalogError::MappingError)?;

        Ok(result.to_catalog_entry_document()?)
    }

    async fn bulk_create(
        &self,
        account: &Account,
        catalog: &[CatalogObjectBulkDocument<String>],
    ) -> Result<Vec<SqlCatalogObjectDocument>, CatalogError> {
        let mut objects_dependency_count: HashMap<String, u32> = HashMap::new();
        let mut objects_created_document: HashMap<String, SqlCatalogObjectDocument> =
            HashMap::new();
        let mut objects_created_document_id: HashMap<&str, CatalogId<Self>> = HashMap::new();
        let mut objects_dependency_map: HashMap<String, &CatalogObject<String>> = HashMap::new();

        for (index, item) in catalog.iter().enumerate() {
            let key_id = match &item.id {
                Some(id) => id.clone(),
                None => make_id_by_index(index.try_into().unwrap()),
            };

            objects_dependency_map
                .entry(key_id.clone())
                .or_insert(&item.catalog_object);

            objects_dependency_count
                .entry(key_id)
                .or_insert(0);

            match &item.catalog_object {
                CatalogObject::Variation(ItemVariation { item_id, .. })
                | CatalogObject::Modification(ItemModification { item_id, .. }) => {
                    let key = item_id.clone();
                    objects_dependency_count.entry(key).and_modify(|e| *e += 1);
                }
                CatalogObject::Control(ItemControl { item_id, control }) => {
                    objects_dependency_count
                        .entry(item_id.clone())
                        .and_modify(|e| *e += 1);

                    if let Control::Matrix(MatrixControl { combinations, .. }) = &control {
                        for (key, id_ref) in combinations.iter() {
                            println!("combination key: {:?}  idRef: {:?}", key, id_ref);
                            objects_dependency_count
                                .entry(id_ref.clone())
                                .or_insert(0);

                            objects_dependency_count
                                .entry(id_ref.clone())
                                .and_modify(|e| *e += 1);
                        }
                    }
                }
                _ => {}
            }
        }

        let mut items_sorted_to_insert: Vec<(&String, &u32)> =
            objects_dependency_count.iter().collect();

        items_sorted_to_insert.sort_by(|a, b| b.1.cmp(a.1));

        println!("THE SORTED ITEMS {:?}", items_sorted_to_insert);

        // we start creating the dependencies from the less dependant
        for (alias_id, _) in &items_sorted_to_insert {
            let item = objects_dependency_map.get(*alias_id).unwrap();
            println!("item iter {:?}", item);
            println!(
                "objects_created_document_id {:?}",
                objects_created_document_id
            );
            match item {
                item @ CatalogObject::Item(_) => {
                    let catalog_object = <Self as BulkDocumentReferencesResolver>::resolve(
                        &objects_created_document_id,
                        item,
                    )?;
                    let document = self.create(account, &catalog_object).await?;
                    println!("creating element Item {}, {:?}", item, document);

                    objects_created_document_id
                        .entry(alias_id)
                        .or_insert(document.id);

                    objects_created_document
                        .entry(alias_id.to_string())
                        .or_insert(document);
                }
                v @ CatalogObject::Variation(ItemVariation { item_id, .. })
                | v @ CatalogObject::Modification(ItemModification { item_id, .. })
                | v @ CatalogObject::Control(ItemControl { item_id, .. })
                | v @ CatalogObject::Delivery(ItemDelivery { item_id, .. }) => {
                    println!(" item_id {:?}, variation {:?} ", item_id, v);
                    println!("Objects documents {:?}", objects_created_document);

                    if objects_created_document_id.get(item_id.as_str()).is_some() {
                        println!("!ID FOUND {:?}", item_id);

                        let catalog_object = <Self as BulkDocumentReferencesResolver>::resolve(
                            &objects_created_document_id,
                            v,
                        )?;

                        let document = self.create(account, &catalog_object).await?;

                        objects_created_document_id
                            .entry(alias_id)
                            .or_insert(document.id);

                        objects_created_document
                            .entry(alias_id.to_string())
                            .or_insert(document);
                    } else {
                        return Err(CatalogError::BulkReferenceNotExist(item_id.to_string()));
                    }
                }
            }
        }

        Ok(items_sorted_to_insert
            .iter()
            .map(|(id, _)| objects_created_document.remove(id.as_str()).unwrap())
            .collect())
    }

    async fn exists(&self, _account: &Account, id: &Id) -> Result<bool, CatalogError> {
        let mut pool = self
            .pool
            .acquire()
            .await
            .map_err(|_| CatalogError::DatabaseError)?;

        println!("Check if exists {:?} {:?}", _account, id);

        let catalog_row: Count = sqlx::query_as(self.get_sql_to_exists().as_str())
            .bind(id)
            .bind(_account)
            .fetch_one(&mut pool)
            .await
            .map_err(|err| match err {
                sqlx::Error::RowNotFound => CatalogError::CatalogEntryNotFound(id.to_string()),
                _ => CatalogError::DatabaseError,
            })?;

        println!("row count {:?}", catalog_row);

        Ok(catalog_row.count != 0)
    }

    async fn read(
        &self,
        _account: &Account,
        id: &Id,
    ) -> Result<SqlCatalogObjectDocument, CatalogError> {
        let mut pool = self
            .pool
            .acquire()
            .await
            .map_err(|_| CatalogError::DatabaseError)?;
        let catalog_row: CatalogObjectRow = sqlx::query_as(self.get_sql_to_read().as_str())
            .bind(id)
            .fetch_one(&mut pool)
            .await
            .map_err(|err| match err {
                sqlx::Error::RowNotFound => CatalogError::CatalogEntryNotFound(id.to_string()),
                _ => CatalogError::DatabaseError,
            })?;

        Ok(catalog_row.to_catalog_entry_document()?)
    }

    async fn update(
        &self,
        account: &Account,
        id: &Id,
        catalog_entry: &CatalogObject<Id>,
    ) -> Result<SqlCatalogObjectDocument, CatalogError> {
        let (data, sql) = match catalog_entry {
            CatalogObject::Item(entry) => {
                let data = serde_json::to_value(entry).map_err(|_| CatalogError::MappingError)?;
                let sql = self.get_sql_to_update(CatalogSchema::ItemData, "Item");
                (data, sql)
            }
            variation @ CatalogObject::Variation(ItemVariation { item_id, .. })
            | variation @ CatalogObject::Modification(ItemModification { item_id, .. })
            | variation @ CatalogObject::Control(ItemControl { item_id, .. })
            | variation @ CatalogObject::Delivery(ItemDelivery { item_id, .. }) => {
                // Here we get {type, data} json value
                let data =
                    serde_json::to_value(variation).map_err(|_| CatalogError::MappingError)?;
                // we extract the data
                let data = data
                    .get("data")
                    .ok_or(CatalogError::MappingError)?
                    .to_owned();

                println!("{:?}", data);
                let sql = self.get_sql_to_update(
                    CatalogSQLService::to_catalog_schema(variation),
                    variation.to_string().as_str(),
                );

                if !self.exists(account, item_id).await? {
                    println!("the catalog id doesnt not exist");
                    return Err(CatalogError::CatalogBadRequest);
                }
                (data, sql)
            }
        };

        let mut pool = self
            .pool
            .acquire()
            .await
            .map_err(|_| CatalogError::DatabaseError)?;

        println!("SQL: {}", sql);
        let result: CatalogObjectRow = sqlx::query_as(sql.as_str())
            .bind(Json(data))
            .bind(account.as_str())
            .bind(id)
            .fetch_one(&mut pool)
            .await
            .map_err(|err| match err {
                sqlx::Error::RowNotFound => CatalogError::CatalogEntryNotFound(id.to_string()),
                _ => CatalogError::DatabaseError,
            })?;

        Ok(result.to_catalog_entry_document()?)
    }

    async fn list(
        &self,
        account: &Account,
        query: &Self::Query,
    ) -> Result<Vec<SqlCatalogObjectDocument>, CatalogError> {
        let name_is_like_expr = |name: &str| {
            Cond::any()
                .add(Expr::cust_with_values(
                    format!(
                        "json_extract({}, '$.name') LIKE ?",
                        CatalogSchema::ItemData.to_string().as_str()
                    )
                    .as_str(),
                    vec![format!("%{}%", name)],
                ))
                .add(Expr::cust_with_values(
                    format!(
                        "json_extract({}, '$.name') LIKE ?",
                        CatalogSchema::ItemVariationData.to_string().as_str()
                    )
                    .as_str(),
                    vec![format!("%{}%", name)],
                ))
        };

        let (sql, values) = Qsql::select()
            .expr(Expr::asterisk())
            .from(CatalogSchema::Table)
            .and_where(Expr::col(CatalogSchema::Account).eq(account.to_string()))
            .conditions(
                query.options.name.is_some(),
                |q| {
                    let name = query.options.name.as_ref().unwrap();
                    q.cond_where(name_is_like_expr(name.as_str()));
                },
                |_| {},
            )
            .conditions(
                query.options.tags.is_some(),
                |q| {
                    let tags = query.options.tags.as_ref().unwrap();
                    let value_array = serde_json::to_value(tags)
                        .map_err(|_| CatalogError::MappingError)
                        .unwrap();
                    let str_json = serde_json::to_string(&value_array)
                        .map_err(|_| CatalogError::MappingError)
                        .unwrap();
                    q.cond_where(Expr::cust_with_values(
                        format!(
                            "json_extract({}, '$.tags') LIKE ?",
                            CatalogSchema::ItemData.to_string()
                        )
                        .as_str(),
                        vec![str_json],
                    ));
                },
                |_| {},
            )
            .conditions(
                query.options.max_price.is_some(),
                |q| {
                    let max_price = query.options.max_price.unwrap();
                    q.cond_where(Expr::cust_with_values(
                        format!(
                            "json_extract({}, '$.price_amount') <= ?",
                            CatalogSchema::ItemVariationData.to_string()
                        )
                        .as_str(),
                        vec![max_price],
                    ));
                },
                |_| {},
            )
            .conditions(
                query.options.min_price.is_some(),
                |q| {
                    let min_price = query.options.min_price.unwrap();
                    q.cond_where(Expr::cust_with_values(
                        format!(
                            "json_extract({}, '$.price_amount') >= ?",
                            CatalogSchema::ItemVariationData.to_string()
                        )
                        .as_str(),
                        vec![min_price],
                    ));
                },
                |_| {},
            )
            .conditions(
                query.order_by.is_some(),
                |q| {
                    let order_by = query.order_by.as_ref().unwrap();
                    match order_by.field {
                        CatalogColumnOrder::Price => {
                            q.order_by_expr(
                                Expr::cust(
                                    format!(
                                        "json_extract({}, '$.price_amount')",
                                        CatalogSchema::ItemVariationData.to_string()
                                    )
                                    .as_str(),
                                ),
                                OrderSql::from(order_by.direction),
                            );
                        }
                        CatalogColumnOrder::CreatedAt => {
                            q.order_by_expr(
                                Expr::cust(CatalogSchema::CreatedAt.to_string().as_str()),
                                OrderSql::from(order_by.direction),
                            );
                        }
                    };
                },
                |_| {},
            )
            .build(QueryBuilder);

        let mut pool = self
            .pool
            .acquire()
            .await
            .map_err(|err| {
                println!("{:?}", err);
                CatalogError::DatabaseError
            })?;

        let result: Vec<CatalogObjectRow> = bind_query_as(sqlx::query_as(&sql), &values)
            .fetch_all(&mut pool)
            .await
            .map_err(|err| {
                println!("{:?}", err);
                CatalogError::DatabaseError
            })?;

        Ok(result
            .into_iter()
            .map(|x| x.to_catalog_entry_document().unwrap())
            .collect())
    }
}

async fn increase_item_variation_units(
    pool: &Pool,
    account: &Account,
    options: &IncreaseItemVariationUnitsPayload<Id>,
) -> Result<(), CatalogError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|_| CatalogError::DatabaseError)?;

    let sql_increase_expr = format!(
        "json_set({col_name}, '$.available_units', (
          json_extract({col_name}, '$.available_units') + ?
        ))",
        col_name = CatalogSchema::ItemVariationData.to_string()
    );

    let (sql, values) = Qsql::update()
        .table(CatalogSchema::Table)
        .value_expr(
            CatalogSchema::ItemVariationData,
            Expr::cust_with_values(sql_increase_expr.as_str(), vec![options.units]),
        )
        .and_where(Expr::cust_with_values(
            "id = ?",
            vec![options.id.to_string()],
        ))
        .and_where(Expr::col(CatalogSchema::Account).eq(account.to_string()))
        .build(QueryBuilder);

    bind_query(sqlx::query(&sql), &values)
        .execute(&mut tx)
        .await
        .map_err(|_| CatalogError::DatabaseError)?;

    tx.commit().await.map_err(|_| CatalogError::DatabaseError)?;
    Ok(())
}

fn make_id_by_index(id: Id) -> String {
    return format!("#{}-index", id);
}

#[async_trait]
impl Commander for CatalogSQLService {
    type Cmd = CatalogCmd<Id>;
    type Account = Account;

    async fn cmd(&self, account: &Self::Account, cmd: Self::Cmd) -> Result<(), CatalogError> {
        match cmd {
            Self::Cmd::IncreaseItemVariationUnits(options) => {
                increase_item_variation_units(&self.pool, account, &options).await?;
                Ok(())
            }
        }
    }
}
#[derive(Serialize, Deserialize, Debug, FromRow)]
struct Count {
    count: i64,
}

#[derive(Serialize, Deserialize, Debug, FromRow)]
pub struct CatalogObjectRow {
    pub id: Id,
    pub account: String,
    pub version: NaiveDateTime,
    pub type_entry: String,
    pub item_data: Option<Json<Item>>,
    pub item_variation_data: Option<Json<ItemVariation<Id>>>,
    pub item_modification_data: Option<Json<ItemModification<Id>>>,
    pub item_control_data: Option<Json<ItemControl<Id>>>,
    pub item_delivery_data: Option<Json<ItemDelivery<Id>>>,
    pub created_at: NaiveDateTime,
}

impl CatalogObjectRow {
    pub fn to_catalog_entry(&self) -> Result<CatalogObject<Id>, CatalogError> {
        let mut value = serde_json::Map::new();
        let type_entry_str: &str = self.type_entry.as_str();

        value.insert("type".to_string(), type_entry_str.into());

        let data = match Some(type_entry_str) {
            Some("Item") => serde_json::to_value(self.item_data.as_ref().unwrap()),
            Some("Variation") => serde_json::to_value(self.item_variation_data.as_ref().unwrap()),
            Some("Modification") => {
                serde_json::to_value(self.item_modification_data.as_ref().unwrap())
            }
            Some("Control") => serde_json::to_value(self.item_control_data.as_ref().unwrap()),
            Some("Delivery") => serde_json::to_value(self.item_delivery_data.as_ref().unwrap()),
            _ => {
                println!("mapping not found");
                return Err(CatalogError::CatalogBadRequest);
            }
        }
        .map_err(|_| CatalogError::MappingError);

        value.insert("data".to_string(), data?);

        let value: CatalogObject<Id> = serde_json::from_value(serde_json::Value::Object(value))
            .map_err(|_| CatalogError::MappingError)?;

        Ok(value)
    }

    pub fn to_catalog_entry_document(self) -> Result<SqlCatalogObjectDocument, CatalogError> {
        let entry = self.to_catalog_entry()?;
        Ok(SqlCatalogObjectDocument {
            account: self.account,
            created_at: self.created_at,
            catalog_object: entry,
            id: self.id,
            version: self.version,
        })
    }
}

pub enum CatalogSchema {
    Table,
    Id,
    Account,
    TypeEntry,
    _Version,
    ItemData,
    ItemVariationData,
    ItemModificationData,
    ItemDeliveryData,
    ItemControlData,
    CreatedAt,
}

impl Iden for CatalogSchema {
    fn unquoted(&self, s: &mut dyn std::fmt::Write) {
        write!(
            s,
            "{}",
            match self {
                Self::Table => "catalogs",
                Self::Id => "id",
                Self::Account => "account",
                Self::ItemData => "item_data",
                Self::ItemVariationData => "item_variation_data",
                Self::ItemModificationData => "item_modification_data",
                Self::ItemDeliveryData => "item_delivery_data",
                Self::ItemControlData => "item_control_data",
                Self::TypeEntry => "type_entry",
                Self::_Version => "version",
                Self::CreatedAt => "created_at",
            }
        )
        .unwrap();
    }
}
