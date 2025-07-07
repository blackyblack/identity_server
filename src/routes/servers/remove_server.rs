use std::collections::HashMap;

use serde::Deserialize;
use serde_json::json;
use tide::{Request, Response, http::mime};

use crate::{
    identity::UserAddress,
    routes::State,
    verify::{admin::admin_verify, nonce::Nonce, signature::Signature},
};

#[derive(Deserialize)]
struct ServerRequest {
    from: UserAddress,
    signature: String,
    nonce: Nonce,
    address: UserAddress,
}

pub async fn route(mut req: Request<State>) -> tide::Result {
    let body: ServerRequest = req.body_json().await?;
    let sender = body.from.clone();

    if req
        .state()
        .admin_storage
        .check_admin(&sender)
        .await
        .is_err()
    {
        return Ok(Response::builder(403)
            .body(json!({"error": "not admin"}))
            .content_type(mime::JSON)
            .build());
    }

    {
        let signature = Signature {
            signer: sender.clone(),
            signature: body.signature,
            nonce: body.nonce,
        };
        if admin_verify(
            &signature,
            body.address.clone(),
            &*req.state().nonce_manager,
        )
        .await
        .is_err()
        {
            return Ok(Response::builder(400)
                .body(json!({"error": "signature verification failed"}))
                .content_type(mime::JSON)
                .build());
        }
    }

    if req
        .state()
        .server_storage
        .remove_server(body.address.clone())
        .await
        .is_err()
    {
        return Ok(Response::builder(400)
            .body(json!({"error": "failed to remove server"}))
            .content_type(mime::JSON)
            .build());
    }

    let response: HashMap<String, serde_json::Value> = HashMap::from([
        ("removed".into(), body.address.into()),
        ("from".into(), sender.into()),
        ("nonce".into(), body.nonce.into()),
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
    use crate::{
        admins::InMemoryAdminStorage,
        servers::storage::ServerInfo,
        verify::{admin::admin_sign, random_keypair},
    };
    use serde_json::Value;
    use std::{collections::HashSet, sync::Arc};
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
        state
            .server_storage
            .add_server(
                "server1".to_string(),
                ServerInfo {
                    url: "http://e".into(),
                    scale: 1.0,
                },
            )
            .await
            .unwrap();

        let signature = admin_sign(&admin_priv, "server1".to_string(), &*state.nonce_manager)
            .await
            .expect("sign");
        let body = json!({
            "from": signature.signer,
            "signature": signature.signature,
            "nonce": signature.nonce,
            "address": "server1"
        });

        let mut req = HttpRequest::new(
            tide::http::Method::Post,
            Url::parse("http://example.com/remove_server").unwrap(),
        );
        req.set_body(serde_json::to_string(&body).unwrap());
        req.set_content_type(mime::JSON);

        let mut server = tide::with_state(state.clone());
        server.at("/remove_server").post(route);

        let mut response: Response = server.respond(req).await.unwrap();

        assert_eq!(response.status(), 200);
        let body: Value = response.body_json().await.unwrap();
        assert_eq!(body["removed"], "server1");
        assert_eq!(body["from"], admin_addr);
        assert_eq!(body["nonce"], signature.nonce);
        assert!(
            !state
                .server_storage
                .servers()
                .await
                .unwrap()
                .contains_key("server1")
        );
    }
}
