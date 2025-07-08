use std::collections::HashMap;

use serde::Deserialize;
use serde_json::json;
use tide::{Request, Response, http::mime};

use crate::{
    identity::UserAddress,
    routes::{State, verify_admin_action},
    verify::{admins::admin_set_moderator_message_prefix, nonce::Nonce},
};

#[derive(Deserialize)]
struct ModeratorRequest {
    from: UserAddress,
    signature: String,
    nonce: Nonce,
}

pub async fn route(mut req: Request<State>) -> tide::Result {
    let recipient = req.param("user")?.to_string();
    let body: ModeratorRequest = req.body_json().await?;
    let sender = body.from.clone();
    let message_prefix = admin_set_moderator_message_prefix(recipient.clone());

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
        .remove_moderator(&sender, recipient.clone())
        .await
        .is_err()
    {
        return Ok(Response::builder(400)
            .body(json!({"error": "failed to remove moderator"}))
            .content_type(mime::JSON)
            .build());
    }

    let response: HashMap<String, serde_json::Value> = HashMap::from([
        ("removed".into(), recipient.into()),
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
        let moderator = "moderator_user".to_string();
        let admins = HashSet::from([admin_address.clone()]);
        let moderators = HashSet::from([moderator.clone()]);
        let admin_storage = Arc::new(InMemoryAdminStorage::new(admins, moderators));
        let state = State {
            admin_storage: admin_storage.clone(),
            ..Default::default()
        };

        let req_url = format!("/remove_moderator/{moderator}");

        // sign the moderator request
        let message_prefix = admin_set_moderator_message_prefix(moderator.clone());
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
        server.at("/remove_moderator/:user").post(route);

        let mut response: Response = server.respond(req).await.unwrap();

        assert_eq!(response.status(), 200);
        let body: Value = response.body_json().await.unwrap();
        assert_eq!(body["removed"], moderator.clone());
        assert_eq!(body["from"], admin_address);
        assert_eq!(body["nonce"], signature.nonce);

        // verify the moderator was removed
        assert!(admin_storage.check_moderator(&moderator).await.is_err());
    }

    #[async_std::test]
    async fn test_no_privilege() {
        let (private_key, _) = random_keypair();
        let admins = HashSet::from(["other_admin".to_string()]);
        let moderator = "moderator_user".to_string();
        let moderators = HashSet::from([moderator.clone()]);
        let admin_storage = Arc::new(InMemoryAdminStorage::new(admins, moderators));
        let state = State {
            admin_storage: admin_storage.clone(),
            ..Default::default()
        };

        let req_url = format!("/remove_moderator/{moderator}");

        let message_prefix = admin_set_moderator_message_prefix(moderator);
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
        server.at("/remove_moderator/:user").post(route);

        let response: Response = server.respond(req).await.unwrap();

        assert_eq!(response.status(), 403);
    }
}
