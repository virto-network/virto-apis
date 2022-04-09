mod catalog;
mod utils;

use catalog::{
    backend::{CatalogSQLService, SQlCatalogCmd, SqlCatalogObject, SqlCatalogQueryOptions},
    models::CatalogObjectBulkDocument,
    service::{CatalogError, CatalogService, Commander},
};

use serde::Serialize;
use serde_json::json;
use sqlx::{migrate::Migrator, SqlitePool as Pool};
use tide::{Body, Request, Response};

static MIGRATOR: Migrator = sqlx::migrate!();
#[derive(Clone)]
struct MyState {
    catalog_service: CatalogSQLService,
}

impl MyState {
    fn new(catalog_service: CatalogSQLService) -> Self {
        Self { catalog_service }
    }
}

fn wrap_result<T: Serialize>(
    result: &Result<T, CatalogError>,
) -> Result<Response, Box<dyn std::error::Error>> {
    match result {
        Ok(result) => {
            let mut res = Response::new(200);
            res.set_body(Body::from_json(&result)?);
            Ok(res)
        }
        Err(err) => match err {
            CatalogError::BulkReferenceNotExist(id) => {
                let mut res = Response::new(400);
                res.set_body(json!({
                  "success": false,
                  "error": "E_BULK_ACTION",
                  "error_message": format!("not found the item {}", id)
                }));
                Ok(res)
            }
            CatalogError::CatalogEntryNotFound(id) => {
                let mut res = Response::new(400);
                res.set_body(json!({
                  "success": false,
                  "error": "E_NOT_FOUND",
                  "error_message": format!("not found the item {}", id)
                }));
                Ok(res)
            }
            CatalogError::CatalogBadRequest => {
                let mut res = Response::new(400);
                res.set_body(json!({
                  "success": false,
                  "error": "E_BAD_REQUEST",
                  "error_message": ""
                }));
                Ok(res)
            }
            CatalogError::DatabaseError => {
                let mut res = Response::new(500);
                res.set_body(json!({
                  "success": false,
                  "error": "E_DATABASE",
                  "error_message": "Please contact with administrator database is down"
                }));
                Ok(res)
            }
            CatalogError::MappingError => {
                let mut res = Response::new(500);
                res.set_body(json!({
                  "success": false,
                  "error": "E_MAPPING",
                  "error_message": "Data corrupted please contact with adminstrator"
                }));
                Ok(res)
            }
        },
    }
}

async fn read(request: Request<MyState>) -> tide::Result {
    let account_id = request.param("account")?;
    let id = request.param("id")?;
    println!("read({}, {})", account_id, id);
    let state = request.state().clone();
    let service = state.catalog_service.clone();
    println!("retriving the service id");
    let result = service.read(&account_id.to_string(), id.parse()?).await;
    Ok(wrap_result(&result).unwrap())
}

async fn list(request: Request<MyState>) -> tide::Result {
    let account_id = request.param("account")?;
    let query: SqlCatalogQueryOptions = request.query().unwrap();
    println!("List({}) - {:?}", account_id, query);
    let state = request.state().clone();
    let service = state.catalog_service.clone();
    let result = service.list(&account_id.to_string(), &query).await;
    Ok(wrap_result(&result).unwrap())
}

async fn create(mut request: Request<MyState>) -> tide::Result {
    let catalog: SqlCatalogObject = request.body_json().await?;
    let account_id = request.param("account")?;
    println!("Create({}) - {:?}", account_id, catalog);
    let state = request.state().clone();
    let service = state.catalog_service.clone();
    let result = service.create(&account_id.to_string(), &catalog).await;
    Ok(wrap_result(&result).unwrap())
}

async fn update(mut request: Request<MyState>) -> tide::Result {
    let catalog: SqlCatalogObject = request.body_json().await?;
    let account_id = request.param("account")?;
    println!("Create({}) - {:?}", account_id, catalog);
    let id = request.param("id")?;
    let state = request.state().clone();
    let service = state.catalog_service.clone();
    let result = service
        .update(&account_id.to_string(), id.parse()?, &catalog)
        .await;
    Ok(wrap_result(&result).unwrap())
}

async fn bulk_create(mut request: Request<MyState>) -> tide::Result {
    let catalog: Vec<CatalogObjectBulkDocument<String>> = request.body_json().await?;
    let account_id = request.param("account")?;
    println!("Bulk-Create({}) - {:?}", account_id, catalog);
    let state = request.state().clone();
    let service = state.catalog_service.clone();
    let result = service.bulk_create(&account_id.to_string(), catalog).await;
    Ok(wrap_result(&result).unwrap())
}

async fn cmd(mut request: Request<MyState>) -> tide::Result {
    let cmd: SQlCatalogCmd = request.body_json().await?;
    let account_id = request.param("account")?;
    let state = request.state().clone();
    let service = state.catalog_service.clone();
    service.cmd(&account_id.to_string(), cmd).await?;
    let mut res = Response::new(200);
    res.set_body(json!({
      "success": true
    }));
    Ok(res)
}

const DEFAULT_DB_FILE: &str = "sqlite:merchant.db";
const DEFAULT_PORT: &str = "5555";

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_file = std::env::args()
        .skip(1)
        .next()
        .map(|f| format!("sqlite:{}", f))
        .unwrap_or(DEFAULT_DB_FILE.into());
    let port = std::env::var("PORT").unwrap_or(DEFAULT_PORT.into());

    let conn = Pool::connect(&db_file).await?;
    MIGRATOR.run(&conn).await?;
    let mut app = tide::with_state(MyState::new(CatalogSQLService::new(conn)));

    app.at("/")
        .get(|_| async move { Ok(json!({ "version": "1" })) });

    app.at("/catalog/:account").get(list).post(create);

    app.at("/catalog/:account/_bulk").post(bulk_create);

    app.at("/catalog/:account/:id").get(read).put(update);

    app.at("/catalog/:account/cmd").post(cmd);

    let addr = format!("0.0.0.0:{}", port);
    println!("Listening on {}", &addr);
    app.listen(addr).await?;
    Ok(())
}
