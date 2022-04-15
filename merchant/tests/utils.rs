use merchant::{Catalog, Context, Error, Event, Msg, State};
use serde::{Deserialize, Serialize};
use serde_json::{from_value, to_value};
use sqlx::{migrate::Migrator, sqlite::SqlitePoolOptions, SqlitePool as Pool};
use std::any::{Any, TypeId};

static MIGRATOR: Migrator = sqlx::migrate!();
pub type AnyHow = Box<dyn std::error::Error>;

pub async fn new_context() -> Result<Context, AnyHow> {
    let pool = get_conn().await?;
    MIGRATOR.run(&pool).await?;
    Ok(Context::new(
        State(Catalog::new(pool)).into(),
        [("account", "account")],
    ))
}

async fn get_conn() -> Result<Pool, AnyHow> {
    Ok(SqlitePoolOptions::new().connect("sqlite::memory:").await?)
}

pub async fn send<T>(cx: &mut Context, msg: impl Into<Msg>) -> Result<T, Error>
where
    T: for<'a> Deserialize<'a>,
{
    match merchant::dispatch(cx, msg.into())? {
        common::End::Async(fut) => {
            fut.await?;
        }
        common::End::WaitResult(fut) => {
            return Ok(fut.await?.to_obj());
        }
    };
    Ok(match cx.events().next().unwrap() {
        Event::CatalogCreated(doc) => doc.to_obj(),
    })
}

pub trait InstanceOf
where
    Self: Any,
{
    fn instance_of<U: ?Sized + Any>(&self) -> bool {
        TypeId::of::<Self>() == TypeId::of::<U>()
    }
}

pub trait ToObject
where
    Self: Serialize,
{
    fn to_obj<T>(&self) -> T
    where
        for<'de> T: Deserialize<'de>,
    {
        from_value(to_value(self).unwrap()).unwrap()
    }
}
impl<T: Serialize> ToObject for T {}

// implement this trait for every type that implements `Any` (which is most types)
impl<T: ?Sized + Any> InstanceOf for T {}

#[macro_export]
macro_rules! as_value {
    ($value:expr, $variant:path) => {
        match $value {
            $variant(x) => Some(x),
            _ => None,
        }
    };
}
