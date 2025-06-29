use std::collections::HashMap;

use serde_json::json;
use tide::{Request, Response, http::mime};

use crate::routes::State;

// async is required by tide server
pub async fn route(req: Request<State>) -> tide::Result {
    let user = req.param("user")?.to_string();
    let is_admin = req.state().admin_storage.is_admin(&user);
    let response: HashMap<String, serde_json::Value> = HashMap::from([
        ("user".into(), user.into()),
        ("is_admin".into(), is_admin.into()),
    ]);
    let response = Response::builder(200)
        .body(json!(response))
        .content_type(mime::JSON)
        .build();
    Ok(response)
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, sync::Arc};

    use crate::{
        admins::AdminStorage, identity::IdentityService, verify::nonce::InMemoryNonceManager,
    };

    use super::*;
    use serde_json::Value;
    use tide::http::{Request as HttpRequest, Response, Url};

    #[async_std::test]
    async fn test_basic() {
        let admin = "admin_user".to_string();
        let admins = HashSet::from([admin.clone()]);
        let admin_storage = Arc::new(AdminStorage::new(admins, HashSet::new()));
        let state = State {
            identity_service: IdentityService::default(),
            admin_storage,
            nonce_manager: Arc::new(InMemoryNonceManager::default()),
        };

        // test for admin user
        let req_url = format!("/is_admin/{admin}");
        let req = HttpRequest::new(
            tide::http::Method::Get,
            Url::parse(&format!("http://example.com{}", req_url)).unwrap(),
        );

        let mut server = tide::with_state(state.clone());
        server.at("/is_admin/:user").get(route);

        let mut response: Response = server.respond(req).await.unwrap();

        assert_eq!(response.status(), 200);
        let body: Value = response.body_json().await.unwrap();
        assert_eq!(body["user"], admin);
        assert_eq!(body["is_admin"], true);

        // test for non-admin user
        let non_admin = "non_admin_user";
        let req_url = format!("/is_admin/{non_admin}");
        let req = HttpRequest::new(
            tide::http::Method::Get,
            Url::parse(&format!("http://example.com{}", req_url)).unwrap(),
        );

        let mut response: Response = server.respond(req).await.unwrap();

        assert_eq!(response.status(), 200);
        let body: Value = response.body_json().await.unwrap();
        assert_eq!(body["user"], non_admin);
        assert_eq!(body["is_admin"], false);
    }
}
