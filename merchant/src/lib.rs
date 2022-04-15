use common::{End, EndResult};
use serde_json::Value;
use sqlx::Pool;

pub mod catalog;
pub use catalog::Catalog;

use catalog::{
    CatalogError, CatalogObject, CatalogObjectDocument, CatalogService,
    IncreaseItemVariationUnitsPayload, OrderField, QueryOptions,
};

pub type Context = common::Context<State, Event>;
type Query = common::query::Query<QueryOptions, OrderField>;
type Id = catalog::backend::Id;

/// Initialize this module with the given configuration
pub async fn create(cfg: &Value) -> Result<State, Error> {
    let db_url = cfg.get("db").expect("DB URL").as_str().unwrap();
    let conn = Pool::connect(db_url).await?;
    Ok(State(Catalog::new(conn)))
}

/// Dispatch a message to this module to be handled asynchronously
pub fn dispatch(cx: &Context, msg: Msg) -> EndResult<Error> {
    let account = cx.get_str("account").ok_or_else(|| Error::Metadata)?.into();
    let state = cx.state();
    Ok(match msg {
        Msg::CatalogQuery(q) => End::WaitResult(Box::pin(async move {
            let service = &state.0;
            let res = service.list(&account, &q).await?;
            Ok(Box::new(res) as Box<dyn common::Serialize>)
        })),
        Msg::CatalogQueryOne(id) => End::WaitResult(Box::pin(async move {
            let service = &state.0;
            let res = service.read(id).await?;
            Ok(Box::new(res) as Box<dyn common::Serialize>)
        })),
        Msg::CatalogCreate(obj) => End::Async(Box::pin(async move {
            let service = &state.0;
            let doc = service.create(&account, &obj).await?;
            cx.put_event(doc);
            Ok(())
        })),
        Msg::CatalogUpdate(id, obj) => End::Async(Box::pin(async move {
            let service = &state.0;
            let doc = service.update(id, &obj).await?;
            cx.put_event(doc);
            Ok(())
        })),
        Msg::IncreaseItemVariationUnits(options) => End::Async(Box::pin(async move {
            let service = &state.0;
            service.increase_item_variation_units(&options).await?;
            Ok(())
        })),
    })
}

pub enum Event {
    CatalogCreated(CatalogObjectDocument),
}
impl From<CatalogObjectDocument> for Event {
    fn from(doc: CatalogObjectDocument) -> Self {
        Self::CatalogCreated(doc)
    }
}

pub enum Msg {
    CatalogQuery(Query),
    CatalogQueryOne(Id),
    CatalogCreate(CatalogObject),
    CatalogUpdate(Id, CatalogObject),
    IncreaseItemVariationUnits(IncreaseItemVariationUnitsPayload<Id>),
}

impl From<IncreaseItemVariationUnitsPayload<Id>> for Msg {
    fn from(var: IncreaseItemVariationUnitsPayload<Id>) -> Self {
        Self::IncreaseItemVariationUnits(var)
    }
}
impl From<Id> for Msg {
    fn from(id: Id) -> Self {
        Self::CatalogQueryOne(id)
    }
}
impl From<Query> for Msg {
    fn from(q: Query) -> Self {
        Self::CatalogQuery(q)
    }
}

pub struct State(pub Catalog);

#[derive(Debug)]
pub enum Error {
    DB(sqlx::Error),
    Catalog(CatalogError),
    Metadata,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::DB(err) => write!(f, "{}", err),
            Error::Catalog(err) => write!(f, "{}", err),
            Error::Metadata => todo!(),
        }
    }
}

impl std::error::Error for Error {}

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        Self::DB(err)
    }
}
impl From<CatalogError> for Error {
    fn from(err: CatalogError) -> Self {
        Self::Catalog(err)
    }
}
