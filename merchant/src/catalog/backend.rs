use std::collections::HashMap;

use async_trait::async_trait;

use sea_query::{Cond, Expr, Iden, Query as Qsql, SqliteQueryBuilder as QueryBuilder};
use serde::{Deserialize, Serialize};
use sqlx::types::chrono::NaiveDateTime;
use sqlx::{types::Json, FromRow, SqlitePool as Pool};

use super::super::utils::query::{Order, Query};
use super::models::{
    CatalogObject, CatalogObjectBulkDocument, CatalogObjectDocument, Item, ItemModification,
    ItemVariation,
};
use super::service::{
    BulkDocumentConverter, CatalogCmd, CatalogError, CatalogService, Commander,
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

impl BulkDocumentConverter for CatalogSQLService {
    type Id = Id;
    fn to_simple_catalog_object(
        id: Self::Id,
        catalog: &CatalogObject<String>,
    ) -> Result<CatalogObject<Self::Id>, CatalogError> {
        match catalog {
            CatalogObject::Item(item) => Ok(CatalogObject::Item(item.clone())),
            CatalogObject::Modification(modification) => {
                let ItemModification {
                    enabled,
                    images,
                    name,
                    price,
                    ..
                } = modification;

                Ok(CatalogObject::Modification(ItemModification {
                    enabled: *enabled,
                    images: images.clone(),
                    item_id: id,
                    name: name.clone(),
                    price: price.clone(),
                }))
            }
            CatalogObject::Variation(variation) => {
                let ItemVariation {
                    available_units,
                    enabled,
                    images,
                    measurement_units,
                    name,
                    price,
                    sku,
                    upc,
                    ..
                } = variation;

                Ok(CatalogObject::Variation(ItemVariation {
                    available_units: *available_units,
                    enabled: *enabled,
                    images: images.clone(),
                    item_id: id,
                    measurement_units: measurement_units.clone(),
                    name: name.clone(),
                    price: price.clone(),
                    sku: sku.clone(),
                    upc: upc.clone(),
                }))
            }
        }
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

    fn get_sql_to_create(&self, field_data_name: CatalogSchema) -> String {
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
                Expr::value("$2"),
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
            //.and_where(Expr::col(CatalogSchema::Account).eq("-1"))
            .build(QueryBuilder);
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
}

#[async_trait]
impl CatalogService for CatalogSQLService {
    type Id = Id;
    type Query = SqlCatalogQueryOptions;

    async fn create(
        &self,
        account: Account,
        catalog_entry: &CatalogObject<Id>,
    ) -> Result<SqlCatalogObjectDocument, CatalogError> {
        let (data, sql, type_entry) = match catalog_entry {
            CatalogObject::Item(entry) => {
                let data = serde_json::to_value(entry).map_err(|_| CatalogError::MappingError)?;
                let sql = self.get_sql_to_create(CatalogSchema::ItemData);
                (data, sql, "Item")
            }
            CatalogObject::Variation(entry) => {
                let data = serde_json::to_value(entry).map_err(|_| CatalogError::MappingError)?;
                let sql = self.get_sql_to_create(CatalogSchema::ItemVariationData);
                if !self.exists(account.to_string(), entry.item_id).await? {
                    return Err(CatalogError::CatalogBadRequest);
                }
                (data, sql, "Variation")
            }
            CatalogObject::Modification(entry) => {
                let data = serde_json::to_value(entry).map_err(|_| CatalogError::MappingError)?;
                let sql = self.get_sql_to_create(CatalogSchema::ItemModificationData);
                if !self.exists(account.to_string(), entry.item_id).await? {
                    return Err(CatalogError::CatalogBadRequest);
                }
                (data, sql, "Modification")
            }
        };

        let mut pool = self
            .pool
            .acquire()
            .await
            .map_err(|_| CatalogError::DatabaseError)?;

        let result: CatalogObjectRow = sqlx::query_as(sql.as_str())
            .bind(rand::random::<Id>())
            .bind(type_entry)
            .bind(account)
            .bind(Json(data))
            .fetch_one(&mut pool)
            .await
            .map_err(|_| CatalogError::MappingError)?;

        Ok(result.to_catalog_entry_document()?)
    }

    async fn bulk_create(
        &self,
        account: Account,
        catalog: Vec<CatalogObjectBulkDocument<String>>,
    ) -> Result<Vec<SqlCatalogObjectDocument>, CatalogError> {
        let mut objects_created: HashMap<String, SqlCatalogObjectDocument> = HashMap::new();
        let mut objects_dependency: HashMap<String, u32> = HashMap::new();
        let mut objects_dependency_document: HashMap<String, &CatalogObject<String>> =
            HashMap::new();

        // we count all references to the dependency

        for (index, item) in catalog.iter().enumerate() {
            let key_id = match &item.id {
                Some(id) => make_id_by_alias(id),
                None => make_id_by_index(index.try_into().unwrap()),
            };

            objects_dependency.entry(key_id.clone()).or_insert(0);
            objects_dependency_document
                .entry(key_id)
                .or_insert(&item.catalog_object);

            match &item.catalog_object {
                CatalogObject::Variation(entry) => {
                    let key = make_id_by_alias(&entry.item_id);
                    objects_dependency.entry(key).and_modify(|e| *e += 1);
                }
                CatalogObject::Modification(entry) => {
                    let key = make_id_by_alias(&entry.item_id);
                    objects_dependency.entry(key).and_modify(|e| *e += 1);
                }
                _ => {}
            }
        }

        let mut items_sorted_to_insert: Vec<(&String, &u32)> = objects_dependency.iter().collect();
        items_sorted_to_insert.sort_by(|a, b| b.1.cmp(a.1));
        // we start creating the dependencies from the less dependant
        for (alias_id, _) in items_sorted_to_insert.clone() {
            let item = *objects_dependency_document.get(alias_id).unwrap();
            match item {
                CatalogObject::Item(item) => {
                    let catalog_object: CatalogObject<self::Id> =
                        CatalogObject::<u32>::Item(item.clone());
                    let document = self.create(account.clone(), &catalog_object).await?;
                    objects_created.entry(alias_id.clone()).or_insert(document);
                }
                CatalogObject::Variation(variation) => {
                    if let Some(catalog_object) = objects_created.get(&variation.item_id.clone()) {
                        let catalog_object =
                            <Self as BulkDocumentConverter>::to_simple_catalog_object(
                                catalog_object.id,
                                &CatalogObject::Variation(variation.clone()),
                            )
                            .unwrap();
                        let document = self.create(account.clone(), &catalog_object).await?;
                        objects_created.entry(alias_id.clone()).or_insert(document);
                    } else {
                        return Err(CatalogError::BulkReferenceNotExist(
                            variation.item_id.clone(),
                        ));
                    }
                }
                CatalogObject::Modification(modification) => {
                    if let Some(catalog_object  ) = objects_created.get(&modification.item_id.clone())
                    {
                        let catalog_object =
                            <Self as BulkDocumentConverter>::to_simple_catalog_object(
                                catalog_object.id,
                                &CatalogObject::Modification(modification.clone()),
                            )
                            .unwrap();
                        let document = self.create(account.clone(), &catalog_object).await?;
                        objects_created.entry(alias_id.clone()).or_insert(document);
                    } else {
                        return Err(CatalogError::BulkReferenceNotExist(
                            modification.item_id.clone(),
                        ));
                    }
                }
            }
        }

        let objects_created: Vec<SqlCatalogObjectDocument> = items_sorted_to_insert
            .iter()
            .map(|(id, _)| objects_created.remove(&(*id).clone()).unwrap())
            .collect();

        Ok(objects_created)
    }

    async fn exists(&self, _account: Account, id: Id) -> Result<bool, CatalogError> {
        let mut pool = self
            .pool
            .acquire()
            .await
            .map_err(|_| CatalogError::DatabaseError)?;

        let catalog_row: Count = sqlx::query_as(self.get_sql_to_exists().as_str())
            .bind(id)
            //.bind(account)
            .fetch_one(&mut pool)
            .await
            .map_err(|err| match err {
                sqlx::Error::RowNotFound => CatalogError::CatalogEntryNotFound(id.to_string()),
                _ => CatalogError::DatabaseError,
            })?;

        Ok(catalog_row.count != 0)
    }

    async fn read(
        &self,
        _account: Account,
        id: Id,
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
        account: Account,
        id: Id,
        catalog_entry: &CatalogObject<Id>,
    ) -> Result<SqlCatalogObjectDocument, CatalogError> {
        let (data, sql) = match catalog_entry {
            CatalogObject::Item(entry) => {
                let data = serde_json::to_value(entry).map_err(|_| CatalogError::MappingError)?;
                let sql = self.get_sql_to_update(CatalogSchema::ItemData, "Item");
                (data, sql)
            }
            CatalogObject::Variation(entry) => {
                let data = serde_json::to_value(entry).map_err(|_| CatalogError::MappingError)?;
                let sql = self.get_sql_to_update(CatalogSchema::ItemVariationData, "Variation");
                if !self.exists(account.to_string(), entry.item_id).await? {
                    return Err(CatalogError::CatalogBadRequest);
                }
                (data, sql)
            }
            CatalogObject::Modification(entry) => {
                let data = serde_json::to_value(entry).map_err(|_| CatalogError::MappingError)?;
                let sql =
                    self.get_sql_to_update(CatalogSchema::ItemModificationData, "Modification");
                if !self.exists(account.to_string(), entry.item_id).await? {
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
        account: Account,
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
            .map_err(|_| CatalogError::DatabaseError)?;

        let result: Vec<CatalogObjectRow> = bind_query_as(sqlx::query_as(&sql), &values)
            .fetch_all(&mut pool)
            .await
            .map_err(|_| CatalogError::DatabaseError)?;

        Ok(result
            .into_iter()
            .map(|x| x.to_catalog_entry_document().unwrap())
            .collect())
    }
}

async fn increase_item_variation_units(
    pool: &Pool,
    account: Account,
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

fn make_id_by_alias(id: &str) -> String {
    return format!("{}", id);
}

fn make_id_by_index(id: Id) -> String {
    return format!("#{}-index", id);
}

#[async_trait]
impl Commander for CatalogSQLService {
    type Cmd = CatalogCmd<Id>;
    type Account = Account;

    async fn cmd(&self, account: Self::Account, cmd: Self::Cmd) -> Result<(), CatalogError> {
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
            _ => return Err(CatalogError::CatalogBadRequest),
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
                Self::TypeEntry => "type_entry",
                Self::_Version => "version",
                Self::CreatedAt => "created_at",
            }
        )
        .unwrap();
    }
}
