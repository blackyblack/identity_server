use std::collections::HashMap;

use serde_json::json;
use tide::{Request, Response, http::mime};

use crate::{identity::idt::balance, routes::State};

pub async fn route(req: Request<State>) -> tide::Result {
    let user = req.param("user")?;
    let balance = balance(&req.state().identity_service, &user.to_string()).await?;
    let response: HashMap<String, serde_json::Value> = HashMap::from([
        ("user".into(), user.into()),
        ("idt".into(), balance.to_string().into()),
    ]);
    let response = Response::builder(200)
        .body(json!(response))
        .content_type(mime::JSON)
        .build();
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{
        proof::prove,
        tests::{MODERATOR, PROOF_ID, USER_A},
    };
    use serde_json::Value;
    use tide::http::{Request as HttpRequest, Response, Url};

    #[async_std::test]
    async fn test_basic() {
        let state = State::default();

        prove(
            &state.identity_service,
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
        )
        .await
        .unwrap();

        let req_url = format!("/idt/{USER_A}");
        let req = HttpRequest::new(
            tide::http::Method::Get,
            Url::parse(&format!("http://example.com{}", req_url)).unwrap(),
        );
        let mut server = tide::with_state(state);
        server.at("/idt/:user").get(route);

        let mut response: Response = server.respond(req).await.unwrap();
        assert_eq!(response.status(), 200);
        let body: Value = response.body_json().await.unwrap();
        assert_eq!(body["user"], USER_A);
        assert_eq!(body["idt"], "100");
    }

    #[async_std::test]
    async fn test_bad_route() {
        let state = State::default();
        let req_url = format!("/idt");
        let req = HttpRequest::new(
            tide::http::Method::Get,
            Url::parse(&format!("http://example.com{}", req_url)).unwrap(),
        );
        let mut server = tide::with_state(state);
        server.at("/idt/:user").get(route);

        let response: Response = server.respond(req).await.unwrap();
        assert_eq!(response.status(), 404);
    }
}
