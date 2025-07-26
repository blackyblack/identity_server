use std::collections::HashMap;

use serde::Deserialize;
use serde_json::json;
use tide::{Request, Response, http::mime};

use crate::{
    identity::UserAddress,
    numbers::Rational,
    routes::{State, verify_admin_action},
    servers::storage::ServerInfo,
    verify::{admins::admin_set_server_message_prefix, nonce::Nonce},
};

#[derive(Deserialize)]
struct ServerRequest {
    from: UserAddress,
    signature: String,
    nonce: Nonce,
    address: UserAddress,
    url: String,
    scale: Rational,
}

pub async fn route(mut req: Request<State>) -> tide::Result {
    let body: ServerRequest = req.body_json().await?;
    let sender = body.from.clone();
    let message_prefix = admin_set_server_message_prefix(body.address.clone());

    if let Err(response) = verify_admin_action(
        req.state(),
        &sender,
        body.signature,
        body.nonce,
        &message_prefix,
    )
    .await
    {
        return Ok(response);
    }

    let info = ServerInfo {
        url: body.url.clone(),
        scale: body.scale.clone(),
    };
    if req
        .state()
        .server_storage
        .add_server(body.address.clone(), info)
        .await
        .is_err()
    {
        return Ok(Response::builder(400)
            .body(json!({"error": "failed to add server"}))
            .content_type(mime::JSON)
            .build());
    }

    let response: HashMap<String, serde_json::Value> = HashMap::from([
        ("server".into(), body.address.into()),
        ("from".into(), sender.into()),
        ("nonce".into(), body.nonce.into()),
        ("url".into(), body.url.into()),
        ("scale".into(), serde_json::to_value(body.scale)?),
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

    use super::*;
    use crate::{
        admins::InMemoryAdminStorage,
        numbers::Rational,
        verify::{random_keypair, sign_message},
    };
    use serde_json::Value;
    use tide::http::{Request as HttpRequest, Response, Url};

    #[async_std::test]
    async fn test_basic() {
        let (admin_priv, admin_addr) = random_keypair();
        let admins = HashSet::from([admin_addr.clone()]);
        let admin_storage = Arc::new(InMemoryAdminStorage::new(admins, HashSet::new()));
        let state = State {
            admin_storage: admin_storage.clone(),
            ..Default::default()
        };

        let req_url = "/add_server";
        let message_prefix = admin_set_server_message_prefix("server1".to_string());
        let signature = sign_message(&admin_priv, &message_prefix, &*state.nonce_manager)
            .await
            .expect("Should sign");
        let server_url = "http://example.com".to_string();
        let body = json!({
            "from": signature.signer,
            "signature": signature.signature,
            "nonce": signature.nonce,
            "address": "server1",
            "url": server_url.clone(),
            "scale": Rational::default(),
        });

        let mut req = HttpRequest::new(
            tide::http::Method::Post,
            Url::parse(&format!("http://example.com{}", req_url)).unwrap(),
        );
        req.set_body(serde_json::to_string(&body).unwrap());
        req.set_content_type(mime::JSON);

        let mut server = tide::with_state(state.clone());
        server.at("/add_server").post(route);

        let mut response: Response = server.respond(req).await.unwrap();

        assert_eq!(response.status(), 200);
        let body: Value = response.body_json().await.unwrap();
        assert_eq!(body["server"], "server1");
        assert_eq!(body["from"], admin_addr);
        assert_eq!(body["nonce"], signature.nonce);
        assert_eq!(body["url"], server_url);
        let scale: Rational =
            serde_json::from_value(body["scale"].clone()).expect("failed to deserialize scale");
        assert_eq!(scale, Rational::default());
        assert!(
            state
                .server_storage
                .servers()
                .await
                .unwrap()
                .contains_key("server1")
        );
    }

    #[async_std::test]
    async fn test_no_privilege() {
        // create a random keypair for a non-privileged user
        let (private_key, _) = random_keypair();
        let admins = HashSet::from(["other_admin".to_string()]);
        let admin_storage = Arc::new(InMemoryAdminStorage::new(admins, HashSet::new()));
        let state = State {
            admin_storage: admin_storage.clone(),
            ..Default::default()
        };

        let req_url = "/add_server";
        let message_prefix = admin_set_server_message_prefix("server1".to_string());
        let signature = sign_message(&private_key, &message_prefix, &*state.nonce_manager)
            .await
            .expect("Should sign");
        let body = json!({
            "from": signature.signer,
            "signature": signature.signature,
            "nonce": signature.nonce,
            "address": "server1",
            "url": "http://example.com",
            "scale": Rational::default(),
        });

        let mut req = HttpRequest::new(
            tide::http::Method::Post,
            Url::parse(&format!("http://example.com{}", req_url)).unwrap(),
        );
        req.set_body(serde_json::to_string(&body).unwrap());
        req.set_content_type(mime::JSON);

        let mut server = tide::with_state(state);
        server.at("/add_server").post(route);

        let response: Response = server.respond(req).await.unwrap();

        assert_eq!(response.status(), 403);
    }
}
