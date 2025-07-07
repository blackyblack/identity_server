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
struct AdminRequest {
    from: UserAddress,
    signature: String,
    nonce: Nonce,
}

pub async fn route(mut req: Request<State>) -> tide::Result {
    let recipient = req.param("user")?.to_string();
    let body: AdminRequest = req.body_json().await?;
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
        if admin_verify(&signature, recipient.clone(), &*req.state().nonce_manager)
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
        .remove_admin(&sender, recipient.clone())
        .await
        .is_err()
    {
        return Ok(Response::builder(400)
            .body(json!({"error": "failed to add admin"}))
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
        verify::{admin::admin_sign, random_keypair},
    };

    use super::*;
    use serde_json::Value;
    use tide::http::{Request as HttpRequest, Response, Url};

    #[async_std::test]
    async fn test_basic() {
        let (private_key, admin_address) = random_keypair();
        let other_admin = "other_admin".to_string();
        let admins = HashSet::from([admin_address.clone(), other_admin.clone()]);
        let admin_storage = Arc::new(InMemoryAdminStorage::new(admins, HashSet::new()));
        let state = State {
            admin_storage: admin_storage.clone(),
            ..Default::default()
        };

        let req_url = format!("/remove_admin/{other_admin}");

        // sign the admin request
        let signature = admin_sign(&private_key, other_admin.clone(), &*state.nonce_manager)
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
        server.at("/remove_admin/:user").post(route);

        let mut response: Response = server.respond(req).await.unwrap();

        assert_eq!(response.status(), 200);
        let body: Value = response.body_json().await.unwrap();
        assert_eq!(body["removed"], other_admin.clone());
        assert_eq!(body["from"], admin_address);
        assert_eq!(body["nonce"], signature.nonce);

        // verify the old admin was removed
        assert!(admin_storage.check_admin(&other_admin).await.is_err());
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

        let user = "new_admin_user".to_string();
        let req_url = format!("/remove_admin/{user}");

        let signature = admin_sign(&private_key, user, &*state.nonce_manager)
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
        server.at("/remove_admin/:user").post(route);

        let response: Response = server.respond(req).await.unwrap();

        assert_eq!(response.status(), 403);
    }
}
