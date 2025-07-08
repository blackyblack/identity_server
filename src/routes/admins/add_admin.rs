use std::collections::HashMap;

use serde::Deserialize;
use serde_json::json;
use tide::{Request, Response, http::mime};

use crate::{
    identity::UserAddress,
    routes::{State, verify_admin_action},
    verify::{admins::admin_message_prefix, nonce::Nonce},
};

#[derive(Deserialize)]
struct AdminRequest {
    from: UserAddress,
    signature: String,
    nonce: Nonce,
}

pub async fn route(mut req: Request<State>) -> tide::Result {
    let recipient = req.param("user")?.to_string();
    let body: AdminRequest = req.body_json().await?;
    let sender = body.from.clone();
    let message_prefix = admin_message_prefix(recipient.clone());

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

    if req
        .state()
        .admin_storage
        .add_admin(&sender, recipient.clone())
        .await
        .is_err()
    {
        return Ok(Response::builder(400)
            .body(json!({"error": "failed to add admin"}))
            .content_type(mime::JSON)
            .build());
    }

    let response: HashMap<String, serde_json::Value> = HashMap::from([
        ("admin".into(), recipient.into()),
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
    use std::{collections::HashSet, sync::Arc};

    use crate::{
        admins::{AdminStorage, InMemoryAdminStorage},
        verify::{random_keypair, sign_message},
    };

    use super::*;
    use serde_json::Value;
    use tide::http::{Request as HttpRequest, Response, Url};

    #[async_std::test]
    async fn test_basic() {
        let (private_key, admin_address) = random_keypair();
        let admins = HashSet::from([admin_address.clone()]);
        let admin_storage = Arc::new(InMemoryAdminStorage::new(admins, HashSet::new()));
        let state = State {
            admin_storage: admin_storage.clone(),
            ..Default::default()
        };

        let new_admin = "new_admin_user".to_string();
        let req_url = format!("/add_admin/{new_admin}");

        // sign the admin request
        let message_prefix = admin_message_prefix(new_admin.clone());
        let signature = sign_message(&private_key, &message_prefix, &*state.nonce_manager)
            .await
            .expect("Should sign");

        let body = json!({
            "from": signature.signer,
            "signature": signature.signature,
            "nonce": signature.nonce,
        });

        let mut req = HttpRequest::new(
            tide::http::Method::Post,
            Url::parse(&format!("http://example.com{}", req_url)).unwrap(),
        );
        req.set_body(serde_json::to_string(&body).unwrap());
        req.set_content_type(mime::JSON);

        let mut server = tide::with_state(state);
        server.at("/add_admin/:user").post(route);

        let mut response: Response = server.respond(req).await.unwrap();

        assert_eq!(response.status(), 200);
        let body: Value = response.body_json().await.unwrap();
        assert_eq!(body["admin"], new_admin.clone());
        assert_eq!(body["from"], admin_address);
        assert_eq!(body["nonce"], signature.nonce);

        // verify the user is now an admin
        assert!(admin_storage.check_admin(&new_admin).await.is_ok());
    }

    #[async_std::test]
    async fn test_no_privilege() {
        let (private_key, _) = random_keypair();
        let admins = HashSet::from(["other_admin".to_string()]);
        let admin_storage = Arc::new(InMemoryAdminStorage::new(admins, HashSet::new()));
        let state = State {
            admin_storage: admin_storage.clone(),
            ..Default::default()
        };

        let new_admin = "new_admin_user".to_string();
        let req_url = format!("/add_admin/{new_admin}");

        let message_prefix = admin_message_prefix(new_admin.clone());
        let signature = sign_message(&private_key, &message_prefix, &*state.nonce_manager)
            .await
            .expect("Should sign");

        let body = json!({
            "from": signature.signer,
            "signature": signature.signature,
            "nonce": signature.nonce,
        });

        let mut req = HttpRequest::new(
            tide::http::Method::Post,
            Url::parse(&format!("http://example.com{}", req_url)).unwrap(),
        );
        req.set_body(serde_json::to_string(&body).unwrap());
        req.set_content_type(mime::JSON);

        let mut server = tide::with_state(state);
        server.at("/add_admin/:user").post(route);

        let response: Response = server.respond(req).await.unwrap();

        assert_eq!(response.status(), 403);
    }
}
