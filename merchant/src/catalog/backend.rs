use async_trait::async_trait;

use sea_query::{Cond, Expr, Iden, Query as Qsql, SqliteQueryBuilder as QueryBuilder};
use serde::{Deserialize, Serialize};
use sqlx::types::chrono::NaiveDateTime;
use sqlx::{types::Json, FromRow, SqlitePool as Pool};

use super::models;

use super::{CatalogError, CatalogService, OrderField};
use sea_query::Order as OrderSql;

pub type CatalogObject = models::CatalogObject<Id>;
pub type CatalogObjectDocument = models::CatalogObjectDocument<Id, Account>;
#[allow(dead_code)]
pub type ItemVariation = models::ItemVariation<Id>;
pub type ItemModification = models::ItemModification<Id>;

sea_query::sea_query_driver_sqlite!();
use sea_query_driver_sqlite::{bind_query, bind_query_as};

pub type Id = u32;
pub type Account = String;

#[derive(Clone)]
pub struct Catalog {
    pool: Pool,
}

impl Catalog {
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
            .build(QueryBuilder);
        sql
    }

    fn get_sql_to_update(&self, field: CatalogSchema, type_entry: &str) -> String {
        let (sql, _) = Qsql::update()
            .table(CatalogSchema::Table)
            .value(field, "-1".into())
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

    pub(crate) async fn increase_item_variation_units(
        &self,
        options: &super::IncreaseItemVariationUnitsPayload<Id>,
    ) -> Result<(), CatalogError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| CatalogError::StorageError)?;

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
            .build(QueryBuilder);

        bind_query(sqlx::query(&sql), &values)
            .execute(&mut tx)
            .await
            .map_err(|_| CatalogError::StorageError)?;

        tx.commit().await.map_err(|_| CatalogError::StorageError)?;
        Ok(())
    }
}

#[async_trait]
impl CatalogService for Catalog {
    type Id = Id;
    type Query = crate::Query;
    type Account = Account;

    async fn create(
        &self,
        account: &Self::Account,
        catalog_entry: &CatalogObject,
    ) -> Result<CatalogObjectDocument, CatalogError> {
        let (data, sql, type_entry) = match catalog_entry {
            CatalogObject::Item(entry) => {
                let data = serde_json::to_value(entry).map_err(|_| CatalogError::MappingError)?;
                let sql = self.get_sql_to_create(CatalogSchema::ItemData);
                (data, sql, "Item")
            }
            CatalogObject::Variation(entry) => {
                let data = serde_json::to_value(entry).map_err(|_| CatalogError::MappingError)?;
                let sql = self.get_sql_to_create(CatalogSchema::ItemVariationData);
                if !self.exists(&entry.item_id).await? {
                    return Err(CatalogError::CatalogBadRequest);
                }
                (data, sql, "Variation")
            }
            CatalogObject::Modification(entry) => {
                let data = serde_json::to_value(entry).map_err(|_| CatalogError::MappingError)?;
                let sql = self.get_sql_to_create(CatalogSchema::ItemModificationData);
                if !self.exists(&entry.item_id).await? {
                    return Err(CatalogError::CatalogBadRequest);
                }
                (data, sql, "Modification")
            }
        };

        let mut pool = self
            .pool
            .acquire()
            .await
            .map_err(|_| CatalogError::StorageError)?;

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

    async fn exists(&self, id: &Id) -> Result<bool, CatalogError> {
        let mut pool = self
            .pool
            .acquire()
            .await
            .map_err(|_| CatalogError::StorageError)?;

        let catalog_row: Count = sqlx::query_as(self.get_sql_to_exists().as_str())
            .bind(id)
            .fetch_one(&mut pool)
            .await
            .map_err(|err| match err {
                sqlx::Error::RowNotFound => CatalogError::CatalogEntryNotFound(id.to_string()),
                _ => CatalogError::StorageError,
            })?;

        Ok(catalog_row.count != 0)
    }

    async fn read(&self, id: Id) -> Result<CatalogObjectDocument, CatalogError> {
        let mut pool = self
            .pool
            .acquire()
            .await
            .map_err(|_| CatalogError::StorageError)?;
        let catalog_row: CatalogObjectRow = sqlx::query_as(self.get_sql_to_read().as_str())
            .bind(id)
            .fetch_one(&mut pool)
            .await
            .map_err(|err| match err {
                sqlx::Error::RowNotFound => CatalogError::CatalogEntryNotFound(id.to_string()),
                _ => CatalogError::StorageError,
            })?;

        Ok(catalog_row.to_catalog_entry_document()?)
    }

    async fn update(
        &self,
        id: Id,
        catalog_entry: &CatalogObject,
    ) -> Result<CatalogObjectDocument, CatalogError> {
        let (data, sql) = match catalog_entry {
            CatalogObject::Item(entry) => {
                let data = serde_json::to_value(entry).map_err(|_| CatalogError::MappingError)?;
                let sql = self.get_sql_to_update(CatalogSchema::ItemData, "Item");
                (data, sql)
            }
            CatalogObject::Variation(entry) => {
                let data = serde_json::to_value(entry).map_err(|_| CatalogError::MappingError)?;
                let sql = self.get_sql_to_update(CatalogSchema::ItemVariationData, "Variation");
                if !self.exists(&entry.item_id).await? {
                    return Err(CatalogError::CatalogBadRequest);
                }
                (data, sql)
            }
            CatalogObject::Modification(entry) => {
                let data = serde_json::to_value(entry).map_err(|_| CatalogError::MappingError)?;
                let sql =
                    self.get_sql_to_update(CatalogSchema::ItemModificationData, "Modification");
                if !self.exists(&entry.item_id).await? {
                    return Err(CatalogError::CatalogBadRequest);
                }
                (data, sql)
            }
        };

        let mut pool = self
            .pool
            .acquire()
            .await
            .map_err(|_| CatalogError::StorageError)?;

        let result: CatalogObjectRow = sqlx::query_as(sql.as_str())
            .bind(Json(data))
            .bind(id)
            .fetch_one(&mut pool)
            .await
            .map_err(|err| match err {
                sqlx::Error::RowNotFound => CatalogError::CatalogEntryNotFound(id.to_string()),
                _ => CatalogError::StorageError,
            })?;

        Ok(result.to_catalog_entry_document()?)
    }

    async fn list(
        &self,
        account: &Self::Account,
        query: &Self::Query,
    ) -> Result<Vec<CatalogObjectDocument>, CatalogError> {
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
            .and_where(Expr::col(CatalogSchema::Account).eq(account.as_str()))
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
                        OrderField::Price => {
                            q.order_by_expr(
                                Expr::cust(
                                    format!(
                                        "json_extract({}, '$.price_amount')",
                                        CatalogSchema::ItemVariationData.to_string()
                                    )
                                    .as_str(),
                                ),
                                OrderSql::from(to_sql_ord(order_by.direction)),
                            );
                        }
                        OrderField::CreatedAt => {
                            q.order_by_expr(
                                Expr::cust(CatalogSchema::CreatedAt.to_string().as_str()),
                                OrderSql::from(to_sql_ord(order_by.direction)),
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
            .map_err(|_| CatalogError::StorageError)?;

        let result: Vec<CatalogObjectRow> = bind_query_as(sqlx::query_as(&sql), &values)
            .fetch_all(&mut pool)
            .await
            .map_err(|_| CatalogError::StorageError)?;

        Ok(result
            .into_iter()
            .map(|x| x.to_catalog_entry_document().unwrap())
            .collect())
    }
}

fn to_sql_ord(ord: common::query::Order) -> sea_query::Order {
    match ord {
        common::query::Order::Asc => sea_query::Order::Asc,
        common::query::Order::Desc => sea_query::Order::Desc,
    }
}

#[derive(Serialize, Deserialize, Debug, FromRow)]
struct Count {
    count: i64,
}
#[derive(Debug, FromRow)]
pub struct CatalogObjectRow {
    pub id: Id,
    pub account: String,
    pub version: u16,
    pub type_entry: String,
    pub item_data: Option<Json<models::Item>>,
    pub item_variation_data: Option<Json<ItemVariation>>,
    pub item_modification_data: Option<Json<ItemModification>>,
    pub created_at: NaiveDateTime,
}

impl CatalogObjectRow {
    pub fn to_catalog_entry(&self) -> Result<CatalogObject, CatalogError> {
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

        let value: CatalogObject = serde_json::from_value(serde_json::Value::Object(value))
            .map_err(|_| CatalogError::MappingError)?;

        Ok(value)
    }

    pub fn to_catalog_entry_document(self) -> Result<CatalogObjectDocument, CatalogError> {
        let entry = self.to_catalog_entry()?;
        Ok(CatalogObjectDocument {
            account: self.account,
            created_at: self.created_at.timestamp() as u32,
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
