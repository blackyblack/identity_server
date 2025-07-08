use serde_json::json;
use tide::{Request, Response, http::mime};

use crate::routes::State;

pub async fn route(req: Request<State>) -> tide::Result {
    let servers = req.state().server_storage.servers().await?;
    let response = Response::builder(200)
        .body(json!(servers))
        .content_type(mime::JSON)
        .build();
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{numbers::Rational, servers::storage::ServerInfo};
    use serde_json::Value;
    use tide::http::{Request as HttpRequest, Response, Url};

    #[async_std::test]
    async fn test_basic() {
        let state = State::default();
        state
            .server_storage
            .add_server(
                "server1".to_string(),
                ServerInfo {
                    url: "http://e".into(),
                    scale: Rational::default(),
                },
            )
            .await
            .unwrap();

        let req = HttpRequest::new(
            tide::http::Method::Get,
            Url::parse("http://example.com/servers").unwrap(),
        );

        let mut server = tide::with_state(state.clone());
        server.at("/servers").get(route);

        let mut response: Response = server.respond(req).await.unwrap();

        assert_eq!(response.status(), 200);
        let body: Value = response.body_json().await.unwrap();
        assert!(body.get("server1").is_some());
    }
}
