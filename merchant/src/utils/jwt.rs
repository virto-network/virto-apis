use jwks_client_rs::{source::WebSource, JwksClient};

use reqwest::Url;
use serde::Deserialize;
use std::{future::Future, pin::Pin, time::Duration};
use tide::{Next, Request, Response};

#[derive(Deserialize, Debug)]
pub struct Claims {
    sub: String,
    iat: usize,
    exp: usize,
}

pub fn get_jwks_client(jwks_host: &str) -> JwksClient<WebSource> {
    let url = Url::parse(jwks_host).unwrap();
    let timeout: Duration = Duration::new(1, 500);

    let source: WebSource = WebSource::builder().build(url).unwrap();

    JwksClient::builder().time_to_live(timeout).build(source)
}

pub fn unauthorized_error() -> Response {
    let mut res = Response::new(401);
    res.set_body("Unauthorized");

    res
}

pub fn jwt_middleware<'a, State: Clone + Send + Sync + 'static>(
    request: Request<State>,
    next: Next<'a, State>,
) -> Pin<Box<dyn Future<Output = tide::Result> + Send + 'a>> {
    Box::pin(async {
        let jwks_host: String = std::env::var("JWKS_HOST").unwrap_or_default();
        let jwks_client = get_jwks_client(&jwks_host.clone());
        let authorization_header = request.header("Authorization");

        if authorization_header.is_none() {
            return Ok(unauthorized_error());
        }

        let token = authorization_header
            .unwrap()
            .get(0)
            .unwrap()
            .to_string()
            .replace("Bearer ", "");

        let try_decode = jwks_client
            .decode::<Claims>(token.as_str(), &[] as &[String])
            .await;

        if try_decode.is_err() {
            println!("{}", try_decode.unwrap_err().to_string());
            return Ok(unauthorized_error());
        }

        let _decoded = &try_decode.unwrap();
        println!("{}", _decoded.sub);

        Ok(next.run(request).await)
    })
}
