use merchant::catalog::service::CatalogError;
use sqlx::{migrate::Migrator, sqlite::SqlitePoolOptions, SqlitePool as Pool};
use std::any::{Any, TypeId};

static MIGRATOR: Migrator = sqlx::migrate!();
pub type AnyHow = Box<dyn std::error::Error>;

pub async fn restore_db() -> Result<Pool, AnyHow> {
    let pool = get_conn().await?;
    MIGRATOR.run(&pool).await?;
    Ok(pool)
}

async fn get_conn() -> Result<Pool, AnyHow> {
    Ok(SqlitePoolOptions::new().connect("sqlite::memory:").await?)
}

pub trait InstanceOf
where
    Self: Any,
{
    fn instance_of<U: ?Sized + Any>(&self) -> bool {
        TypeId::of::<Self>() == TypeId::of::<U>()
    }
}

// implement this trait for every type that implements `Any` (which is most types)
impl<T: ?Sized + Any> InstanceOf for T {}

pub fn check_if_error_is(error: CatalogError, catalog_error: CatalogError) {
    assert!(
        error == catalog_error,
        "error {} doesnt match with the expected {:?}",
        error,
        catalog_error
    );
}

#[macro_export]
macro_rules! as_value {
    ($value:expr, $variant:path) => {
        match $value {
            $variant(x) => Some(x),
            _ => None,
        }
    };
}
