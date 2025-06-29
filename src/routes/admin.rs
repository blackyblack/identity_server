use std::collections::HashMap;

use serde::Deserialize;
use serde_json::json;
use tide::{Request, Response, http::mime};

use crate::{
    identity::UserAddress,
    routes::State,
    verify::{admin::admin_verify, signature::Signature},
};

#[derive(Deserialize)]
struct AdminRequest {
    from: UserAddress,
    signature: String,
    nonce: u64,
}

// async is required by tide server
pub async fn get(req: Request<State>) -> tide::Result {
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

pub async fn post(mut req: Request<State>) -> tide::Result {
    let recepient = req.param("user")?.to_string();
    let body: AdminRequest = req.body_json().await?;
    let sender = body.from.clone();

    if !req.state().admin_storage.is_admin(&sender) {
        return Ok(Response::builder(403)
            .body(json!({"error": "not admin"}))
            .content_type(mime::JSON)
            .build());
    }

    let current_nonce = req.state().nonce_manager.nonce(&sender);

    {
        let signature = Signature {
            signer: sender.clone(),
            signature: body.signature,
            nonce: body.nonce,
        };
        if admin_verify(&signature, recepient.clone(), &*req.state().nonce_manager).is_err() {
            return Ok(Response::builder(400)
                .body(json!({"error": "signature verification failed"}))
                .content_type(mime::JSON)
                .build());
        }
    }

    if req
        .state()
        .admin_storage
        .add_admin(&sender, recepient.clone())
        .is_err()
    {
        return Ok(Response::builder(400)
            .body(json!({"error": "failed to add admin"}))
            .content_type(mime::JSON)
            .build());
    }

    let response: HashMap<String, serde_json::Value> = HashMap::from([
        ("admin".into(), recepient.into()),
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
        admins::AdminStorage,
        identity::IdentityService,
        verify::{admin::admin_sign, nonce::InMemoryNonceManager, random_keypair},
    };

    use super::*;
    use serde_json::Value;
    use tide::http::{Request as HttpRequest, Response, Url};

    #[async_std::test]
    async fn test_get_admin() {
        let admin = "admin_user".to_string();
        let admins = HashSet::from([admin.clone()]);
        let admin_storage = Arc::new(AdminStorage::new(admins, HashSet::new()));
        let state = State {
            identity_service: IdentityService::default(),
            admin_storage,
            nonce_manager: Arc::new(InMemoryNonceManager::default()),
        };

        // test for admin user
        let req_url = format!("/admin/{admin}");
        let req = HttpRequest::new(
            tide::http::Method::Get,
            Url::parse(&format!("http://example.com{}", req_url)).unwrap(),
        );

        let mut server = tide::with_state(state.clone());
        server.at("/admin/:user").get(get);

        let mut response: Response = server.respond(req).await.unwrap();

        assert_eq!(response.status(), 200);
        let body: Value = response.body_json().await.unwrap();
        assert_eq!(body["user"], admin);
        assert_eq!(body["is_admin"], true);

        // test for non-admin user
        let non_admin = "non_admin_user";
        let req_url = format!("/admin/{non_admin}");
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

    #[async_std::test]
    async fn test_post_admin() {
        let (private_key, admin_address) = random_keypair();
        let admins = HashSet::from([admin_address.clone()]);
        let admin_storage = Arc::new(AdminStorage::new(admins, HashSet::new()));
        let state = State {
            identity_service: IdentityService::default(),
            admin_storage: admin_storage.clone(),
            nonce_manager: Arc::new(InMemoryNonceManager::default()),
        };

        let new_admin = "new_admin_user".to_string();
        let req_url = format!("/admin/{new_admin}");

        // sign the admin request
        let signature = admin_sign(&private_key, new_admin.clone(), &*state.nonce_manager)
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
        server.at("/admin/:user").post(post);

        let mut response: Response = server.respond(req).await.unwrap();

        assert_eq!(response.status(), 200);
        let body: Value = response.body_json().await.unwrap();
        assert_eq!(body["admin"], new_admin.clone());
        assert_eq!(body["from"], admin_address);
        assert_eq!(body["nonce"], signature.nonce);

        // verify the user is now an admin
        assert!(admin_storage.is_admin(&new_admin));
    }

    #[async_std::test]
    async fn test_post_admin_no_privilege() {
        let (private_key, _) = random_keypair();
        let admins = HashSet::from(["other_admin".to_string()]);
        let admin_storage = Arc::new(AdminStorage::new(admins, HashSet::new()));
        let state = State {
            identity_service: IdentityService::default(),
            admin_storage,
            nonce_manager: Arc::new(InMemoryNonceManager::default()),
        };

        let new_admin = "new_admin_user".to_string();
        let req_url = format!("/admin/{new_admin}");

        let signature = admin_sign(&private_key, new_admin, &*state.nonce_manager)
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
        server.at("/admin/:user").post(post);

        let response: Response = server.respond(req).await.unwrap();

        assert_eq!(response.status(), 403);
    }
}
