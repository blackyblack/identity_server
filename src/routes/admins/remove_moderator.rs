use std::collections::HashMap;

use serde::Deserialize;
use serde_json::json;
use tide::{Request, Response, http::mime};

use crate::{
    identity::UserAddress,
    routes::State,
    verify::{moderator::moderator_verify, signature::Signature},
};

#[derive(Deserialize)]
struct ModeratorRequest {
    from: UserAddress,
    signature: String,
    nonce: u64,
}

pub async fn route(mut req: Request<State>) -> tide::Result {
    let recipient = req.param("user")?.to_string();
    let body: ModeratorRequest = req.body_json().await?;
    let sender = body.from.clone();

    if req.state().admin_storage.is_admin(&sender).await.is_err() {
        return Ok(Response::builder(403)
            .body(json!({"error": "not admin"}))
            .content_type(mime::JSON)
            .build());
    }

    let current_nonce = req.state().nonce_manager.nonce(&sender).await?;

    {
        let signature = Signature {
            signer: sender.clone(),
            signature: body.signature,
            nonce: body.nonce,
        };
        if moderator_verify(&signature, recipient.clone(), &*req.state().nonce_manager)
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
        ("nonce".into(), current_nonce.into()),
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
        identity::IdentityService,
        verify::{moderator::moderator_sign, nonce::InMemoryNonceManager, random_keypair},
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
            identity_service: IdentityService::default(),
            admin_storage: admin_storage.clone(),
            nonce_manager: Arc::new(InMemoryNonceManager::default()),
        };

        let req_url = format!("/remove_moderator/{moderator}");

        // sign the moderator request
        let signature = moderator_sign(&private_key, moderator.clone(), &*state.nonce_manager)
            .await
            .expect("Should sign successfully");

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
        assert!(admin_storage.is_moderator(&moderator).await.is_err());
    }

    #[async_std::test]
    async fn test_no_privilege() {
        let (private_key, _) = random_keypair();
        let admins = HashSet::from(["other_admin".to_string()]);
        let moderator = "moderator_user".to_string();
        let moderators = HashSet::from([moderator.clone()]);
        let admin_storage = Arc::new(InMemoryAdminStorage::new(admins, moderators));
        let state = State {
            identity_service: IdentityService::default(),
            admin_storage,
            nonce_manager: Arc::new(InMemoryNonceManager::default()),
        };

        let req_url = format!("/remove_moderator/{moderator}");

        let signature = moderator_sign(&private_key, moderator, &*state.nonce_manager)
            .await
            .expect("Should sign successfully");

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
