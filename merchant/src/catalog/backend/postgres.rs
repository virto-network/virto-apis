use async_trait::async_trait;

use serde::{Serialize, Deserialize};
use sqlx::{ PgPool, types::Json, types::{ Uuid }, FromRow};
use sea_query::{ Query as Qsql, Iden, Expr, PostgresQueryBuilder, Cond };

sea_query::sea_query_driver_postgres!();
use sea_query_driver_postgres::{ bind_query_as, bind_query };

use sea_query::{ Order as OrderSql };
use crate::catalog::service::{IncreaseItemVariationUnitsPayload, CatalogColumnOrder};
use super::super::models::{ItemVariation, ItemModification};
use super::super::models::{ CatalogObject, CatalogObjectDocument, Item };
use super::super::service::{ListCatalogQueryOptions, CatalogService, CatalogError, Commander, CatalogCmd };
use super::super::super::utils::query::{ Query, Order };

pub type SqlUuid = Uuid;
pub type SqlAccount = String;
pub type SQlCatalogCmd = CatalogCmd<SqlUuid>;
pub type SqlCatalogObjectDocument = CatalogObjectDocument<SqlUuid, SqlAccount>;
pub type SqlCatalogObject = CatalogObject<SqlUuid>;
pub type SqlCatalogItemVariation = ItemVariation<SqlUuid>;
pub type SqlCatalogQueryOptions = Query<ListCatalogQueryOptions, CatalogColumnOrder>;


impl From<Order> for OrderSql {
  fn from(order_service: Order) -> Self {
    match order_service {
      Order::Asc => {
        OrderSql::Asc
      },
      Order::Desc => {
        OrderSql::Desc
      }
    }
  }
}
#[derive(Clone)]
pub struct CatalogSQLService {
  pool: Box<PgPool>,
}

impl CatalogSQLService { 
  pub fn new(pool: Box<PgPool>) -> Self {
    Self {
      pool,
    }
  }

  fn get_sql_to_create(&self, field_data_name: CatalogSchema) -> String {
    let (sql, _) = Qsql::insert()
      .into_table(CatalogSchema::Table)
      .columns(vec![
        CatalogSchema::TypeEntry,
        CatalogSchema::Account,
        field_data_name,
      ])
      .exprs_panic(vec![
        Expr::value("$1"),
        Expr::value("$2"),
        Expr::value("$3"),
      ])
      .returning(Qsql::select().expr(Expr::asterisk()).take())
      .build(PostgresQueryBuilder);

    sql
  }

  fn get_sql_to_read(&self) -> String {
    let (sql, _) = Qsql::select() 
      .expr(Expr::asterisk())
      .from(CatalogSchema::Table)
      .and_where(Expr::col(CatalogSchema::Account).eq("-1"))
      .and_where(Expr::cust_with_values("uuid = ?", vec!["-1"]))
      .build(PostgresQueryBuilder);

    sql
  }

  fn get_sql_to_exists(&self) -> String {
    let (sql, _) = Qsql::select() 
      .expr(Expr::cust("COUNT(1) as count"))
      .from(CatalogSchema::Table)
      .and_where(Expr::col(CatalogSchema::Account).eq("-1"))
      .and_where(Expr::cust_with_values("uuid = ?", vec!["-1"]))
      .build(PostgresQueryBuilder);

    sql
  }

  fn get_sql_to_update(&self, field: CatalogSchema, type_entry: &str) -> String {
    let (sql, _) = Qsql::update() 
      .table(CatalogSchema::Table)
      .value_expr(CatalogSchema::Version, Expr::cust("now()"))
      .value(field, "-1".into())
      .and_where(Expr::col(CatalogSchema::Account).eq("1"))
      .and_where(Expr::cust(format!("{} = '{}'", CatalogSchema::TypeEntry.to_string(), type_entry).as_ref()))
      .and_where(Expr::cust_with_values("uuid = ?", vec!["-1"]))
      .returning(Qsql::select().expr(Expr::asterisk()).take())
      .build(PostgresQueryBuilder);

    sql
  }
}

#[async_trait]
impl CatalogService<SqlUuid, SqlAccount> for CatalogSQLService {
  async fn create(&self, account: SqlAccount, catalog_entry: &CatalogObject<SqlUuid> ) -> Result<SqlCatalogObjectDocument, CatalogError> {
    let mut pool = self.pool.clone().acquire().await.map_err(|_| CatalogError::DatabaseError )?;

    let (data, sql, type_entry) = match catalog_entry {
      CatalogObject::Item(entry) => {
        let data = serde_json::to_value(entry).map_err(|_| CatalogError::MappingError)?;
        let sql = self.get_sql_to_create(CatalogSchema::ItemData);
        (data, sql, "Item")
      }
      CatalogObject::Variation(entry) => {
        let data = serde_json::to_value(entry).map_err(|_| CatalogError::MappingError )?;
        let sql = self.get_sql_to_create(CatalogSchema::ItemVariationData);
        if !self.exists(account.to_string(), entry.item_uuid).await? {
          return Err(CatalogError::CatalogBadRequest);
        }
        (data, sql, "Variation")
      },
      CatalogObject::Modification(entry) => {
        let data = serde_json::to_value(entry).map_err(|_| CatalogError::MappingError )?;
        let sql = self.get_sql_to_create(CatalogSchema::ItemModificationData);
        if !self.exists(account.to_string(), entry.item_uuid).await? {
          return Err(CatalogError::CatalogBadRequest);
        }
        (data, sql, "Modification")
      },
    };

    let result: CatalogObjectRow = sqlx::query_as(sql.as_str())
      .bind(type_entry)
      .bind(account)
      .bind(Json(data))
      .fetch_one(&mut pool)
      .await
      .map_err(|_| CatalogError::MappingError )?;

    Ok(result.to_catalog_entry_document()?)
  }

  async fn exists(&self, account: SqlAccount,  uuid: SqlUuid) -> Result<bool, CatalogError> {
    let mut pool = self.pool.clone().acquire().await.map_err(|_| CatalogError::DatabaseError )?;

    let catalog_row: Count  = sqlx::query_as(self.get_sql_to_exists().as_str())
      .bind(account)
      .bind(uuid)
      .fetch_one(&mut pool)
      .await
      .map_err(|err| {
        match err {
          sqlx::Error::RowNotFound => CatalogError::CatalogEntryNotFound(uuid.to_string()),
          _ => CatalogError::DatabaseError
        }
      })?;
    
    Ok(catalog_row.count != 0)
  }


  async fn read(&self, account: SqlAccount,  uuid: SqlUuid) -> Result<SqlCatalogObjectDocument, CatalogError> {
    let mut pool = self.pool.clone().acquire().await.map_err(|_| CatalogError::DatabaseError )?;
    let catalog_row: CatalogObjectRow = sqlx::query_as(self.get_sql_to_read().as_str())
      .bind(account)
      .bind(uuid)
      .fetch_one(&mut pool)
      .await
      .map_err(|err| {
        match err {
          sqlx::Error::RowNotFound => CatalogError::CatalogEntryNotFound(uuid.to_string()),
          _ => CatalogError::DatabaseError
        }
      })?;

    Ok(catalog_row.to_catalog_entry_document()?)
  }

  async fn update(&self, account: SqlAccount, uuid: SqlUuid, catalog_entry: &CatalogObject<SqlUuid>) -> Result<SqlCatalogObjectDocument, CatalogError> {
    let mut pool = self.pool.clone().acquire().await.map_err(|_| CatalogError::DatabaseError )?;

    let (data, sql) = match catalog_entry {
      CatalogObject::Item(entry) => {
        let data = serde_json::to_value(entry).map_err(|_| CatalogError::MappingError)?;
        let sql = self.get_sql_to_update(CatalogSchema::ItemData, "Item");
        (data, sql)
      },
      CatalogObject::Variation(entry) => {
        let data = serde_json::to_value(entry).map_err(|_| CatalogError::MappingError)?;
        let sql = self.get_sql_to_update(CatalogSchema::ItemVariationData, "Variation");
        if !self.exists(account.to_string(), entry.item_uuid).await? {
          return Err(CatalogError::CatalogBadRequest);
        }
        (data, sql)
      },
      CatalogObject::Modification(entry) => {
        let data = serde_json::to_value(entry).map_err(|_| CatalogError::MappingError)?;
        let sql = self.get_sql_to_update(CatalogSchema::ItemModificationData, "Modification");
        if !self.exists(account.to_string(), entry.item_uuid).await? {
          return Err(CatalogError::CatalogBadRequest);
        }
        (data, sql)
      },
    };

    let result: CatalogObjectRow = sqlx::query_as(sql.as_str())
      .bind(Json(data))
      .bind(account.as_str())
      .bind(uuid)
      .fetch_one(&mut pool)
      .await
      .map_err(|err| match err {
        sqlx::Error::RowNotFound =>  CatalogError::CatalogEntryNotFound(uuid.to_string()),
        _ => CatalogError::DatabaseError
      })?;

    Ok(result.to_catalog_entry_document()?)
  }

  async fn list(&self, account: SqlAccount, query: &SqlCatalogQueryOptions ) -> Result<Vec<SqlCatalogObjectDocument>, CatalogError> {
    let mut pool = self.pool.clone().acquire().await.map_err(|_| CatalogError::DatabaseError )?;

    let name_is_like_expr = |name: &str| {
      Cond::any()
      .add( Expr::cust_with_values(format!("{} ->> 'name' LIKE ?", CatalogSchema::ItemData.to_string().as_str()).as_str(), vec![ format!("%{}%", name) ]))
      .add( Expr::cust_with_values(format!("{} ->> 'name' LIKE ?", CatalogSchema::ItemVariationData.to_string().as_str()).as_str(), vec![ format!("%{}%", name) ])) }
    ;

    let (sql, values) = Qsql::select()
      .expr(Expr::asterisk())
      .from(CatalogSchema::Table)
      .and_where(Expr::col(CatalogSchema::Account).eq(account.to_string()))
      .conditions(
        matches!(query.options.name, Some(_)),
        |q| {
          let name = query.options.name.as_ref().unwrap();
          q.cond_where(name_is_like_expr(name.as_str()));
        },
        |_| {}
      )
      .conditions(
        matches!(query.options.tags, Some(_)),
        |q| {
          let tags = query.options.tags.as_ref().unwrap();
          let value_array = serde_json::to_value(tags).map_err(|_| { CatalogError::MappingError }).unwrap();
          let str_json = serde_json::to_string(&value_array).map_err(|_| { CatalogError::MappingError }).unwrap();
          q.cond_where(
            Expr::cust_with_values(format!("({} ->> 'tags')::jsonb @> ?::jsonb", CatalogSchema::ItemData.to_string()).as_str(), vec![str_json])
          );
        },
        |_| {}
      )
      .conditions(
        matches!(query.options.max_price, Some(_)),
        |q| {
          let max_price = query.options.max_price.unwrap();
          q.cond_where(
            Expr::cust_with_values(format!("({} ->> 'price_amount')::real <= ?", CatalogSchema::ItemVariationData.to_string()).as_str(), vec![max_price])
          );
        },
        |_| {}
      )
      .conditions(
        matches!(query.options.min_price, Some(_)),
        |q| {
          let min_price = query.options.min_price.unwrap();
          q.cond_where(
            Expr::cust_with_values(format!("({} ->> 'price_amount')::real >= ?", CatalogSchema::ItemVariationData.to_string()).as_str(), vec![min_price])
          );
        },
        |_| {}
      )
      .conditions(
        matches!(query.order_by, Some(_)),
        |q| {
          let order_by = query.order_by.as_ref().unwrap();
          match order_by.column {
            CatalogColumnOrder::Price => {
              q.order_by_expr(
                Expr::cust(format!("({} ->> 'price_amount')::real", CatalogSchema::ItemVariationData.to_string()).as_str()),
                OrderSql::from(order_by.direction),
              );
            },
            CatalogColumnOrder::CreatedAt => {
              q.order_by_expr(
                Expr::cust(CatalogSchema::CreatedAt.to_string().as_str()),
                OrderSql::from(order_by.direction),
              );
            }
          };
        },
        |_| {}
      )
      .build(PostgresQueryBuilder);

      let result: Vec<CatalogObjectRow> = bind_query_as(sqlx::query_as(&sql), &values)
      .fetch_all(&mut pool)
      .await
      .map_err(|_| CatalogError::DatabaseError )?;
    
    Ok(result
        .iter()
        .map(|x|  x.to_catalog_entry_document().unwrap())
        .collect())
  }
}


async fn increase_item_variation_units(pool: Box<PgPool>, account: SqlAccount, options: &IncreaseItemVariationUnitsPayload<SqlUuid>) -> Result<(), CatalogError> {
  let mut tx = pool.clone().begin().await.map_err(|_| CatalogError::DatabaseError )?;

  let sql_increase_expr = format!("jsonb_set({coloumn_name}, '{{available_units}}', (
      COALESCE({coloumn_name} #> '{{available_units}}', '0')::int + ?
    )::text::jsonb)", coloumn_name = CatalogSchema::ItemVariationData.to_string());

  let (sql, values) = Qsql::update()
    .table(CatalogSchema::Table)
    .value_expr(CatalogSchema::ItemVariationData, Expr::cust_with_values(sql_increase_expr.as_str(), vec![options.units] ))
    .and_where(Expr::cust_with_values("uuid::text = ?", vec![options.uuid.to_string()]))
    .and_where(
      Expr::col(CatalogSchema::Account).eq(account.to_string())
    )
    .build(PostgresQueryBuilder);

  bind_query(sqlx::query(&sql),&values)
    .execute(&mut tx)
    .await
    .map_err(|_| CatalogError::DatabaseError)?;
  
  tx.commit()
    .await
    .map_err(|_|  CatalogError::DatabaseError)?;
  Ok(())
}


#[async_trait]
impl Commander<SqlAccount> for CatalogSQLService {
  type Cmd = CatalogCmd<SqlUuid>;
  async fn cmd(&self, account: SqlAccount, cmd: Self::Cmd) -> Result<(), CatalogError> {
    match cmd {
      Self::Cmd::IncreaseItemVariationUnits(options)  => {
        increase_item_variation_units(Box::clone(&self.pool), account, &options).await?;
        Ok(())
      },
    }
  }
}
#[derive(Serialize, Deserialize, Debug, FromRow)]
struct Count {
  count: i64
}
#[derive(Serialize, Deserialize, Debug, FromRow)]
pub struct CatalogObjectRow {
  pub uuid: Uuid,
  pub account: String,
  pub version: chrono::NaiveDateTime,
  pub type_entry: String,
  pub item_data: Option<Json<Item>>,
  pub item_variation_data: Option<Json<ItemVariation<SqlUuid>>>,
  pub item_modification_data: Option<Json<ItemModification<SqlUuid>>>,
  pub created_at: chrono::NaiveDateTime,
}

impl CatalogObjectRow {
  pub fn to_catalog_entry(&self) -> Result<CatalogObject<SqlUuid>, CatalogError> {
    let mut value = serde_json::Map::new();
    let type_entry_str: &str = self.type_entry.as_str();
  
    value.insert("type".to_string(),  serde_json::Value::String(type_entry_str.to_string()));

    let data = match Some(type_entry_str) {
      Some("Item") => serde_json::to_value(self.item_data.as_ref().unwrap()) ,
      Some("Variation") =>  serde_json::to_value(self.item_variation_data.as_ref().unwrap()),
      Some("Modification") =>  serde_json::to_value(self.item_modification_data.as_ref().unwrap()),
      _ => return Err(CatalogError::CatalogBadRequest),
    }.map_err(|_| CatalogError::MappingError);

    value.insert("data".to_string(), data?);

    let value: CatalogObject<SqlUuid> = serde_json::from_value(serde_json::Value::Object(value)).map_err(|_| CatalogError::MappingError)?;

    Ok(value)
  }
  
  pub fn to_catalog_entry_document(&self) -> Result<SqlCatalogObjectDocument, CatalogError> {
    Ok(SqlCatalogObjectDocument {
      account: self.account.clone(),
      created_at: self.created_at,
      catalog_object: self.to_catalog_entry()?,
      uuid: self.uuid,
      version: self.version,
    })
  }
}

pub enum CatalogSchema {
  Table,
  Uuid,
  Account,
  TypeEntry,
  Version,
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
              Self::Uuid => "uuid",
              Self::Account => "account",
              Self::ItemData => "item_data",
              Self::ItemVariationData => "item_variation_data",
              Self::ItemModificationData => "item_modification_data",
              Self::TypeEntry => "type_entry",
              Self::Version => "version",
              Self::CreatedAt => "created_at",
          }
      )
      .unwrap();
  }
}

