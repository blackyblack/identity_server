use std::collections::HashMap;

use serde_json::json;
use tide::{Request, Response, http::mime};

use crate::routes::State;

pub async fn route(req: Request<State>) -> tide::Result {
    let user = req.param("user")?.to_string();
    let is_moderator = req
        .state()
        .admin_storage
        .check_moderator(&user)
        .await
        .is_ok();
    let response: HashMap<String, serde_json::Value> = HashMap::from([
        ("user".into(), user.into()),
        ("is_moderator".into(), is_moderator.into()),
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
        admins::InMemoryAdminStorage, identity::IdentityService,
        config::ExternalServersSection,
        verify::nonce::InMemoryNonceManager,
    };

    use super::*;
    use serde_json::Value;
    use tide::http::{Request as HttpRequest, Response, Url};

    #[async_std::test]
    async fn test_basic() {
        let moderator = "moderator_user".to_string();
        let moderators = HashSet::from([moderator.clone()]);
        let admin_storage = Arc::new(InMemoryAdminStorage::new(HashSet::new(), moderators));
        let state = State {
            identity_service: IdentityService::default(),
            admin_storage,
            nonce_manager: Arc::new(InMemoryNonceManager::default()),
            external_servers: ExternalServersSection::default(),
        };

        // test for moderator user
        let req_url = format!("/is_moderator/{moderator}");
        let req = HttpRequest::new(
            tide::http::Method::Get,
            Url::parse(&format!("http://example.com{}", req_url)).unwrap(),
        );

        let mut server = tide::with_state(state.clone());
        server.at("/is_moderator/:user").get(route);

        let mut response: Response = server.respond(req).await.unwrap();

        assert_eq!(response.status(), 200);
        let body: Value = response.body_json().await.unwrap();
        assert_eq!(body["user"], moderator);
        assert_eq!(body["is_moderator"], true);

        // test for non-moderator user
        let non_moderator = "non_moderator_user";
        let req_url = format!("/is_moderator/{non_moderator}");
        let req = HttpRequest::new(
            tide::http::Method::Get,
            Url::parse(&format!("http://example.com{}", req_url)).unwrap(),
        );

        let mut response: Response = server.respond(req).await.unwrap();

        assert_eq!(response.status(), 200);
        let body: Value = response.body_json().await.unwrap();
        assert_eq!(body["user"], non_moderator);
        assert_eq!(body["is_moderator"], false);
    }
}
