use std::collections::HashMap;

use serde::Deserialize;
use serde_json::json;
use tide::{Request, Response, http::mime};

use crate::{
    identity::{IdtAmount, ProofId, UserAddress, idt::balance, punish::punish},
    routes::State,
    verify::{punish::punish_verify, signature::Signature},
};

#[derive(Deserialize)]
struct PunishRequest {
    from: UserAddress,
    amount: IdtAmount,
    proof_id: ProofId,
    signature: String,
    nonce: u64,
}

pub async fn route(mut req: Request<State>) -> tide::Result {
    let user = req.param("user")?.to_string();
    let body: PunishRequest = req.body_json().await?;
    let moderator = body.from;
    let amount = body.amount;
    let proof_id = body.proof_id;
    if !req.state().admin_storage.is_moderator(&moderator) {
        return Ok(Response::builder(403)
            .body(json!({"error": "not moderator"}))
            .content_type(mime::JSON)
            .build());
    }

    let current_nonce = req.state().nonce_manager.nonce(&moderator);
    {
        let signature = Signature {
            signer: moderator.clone(),
            signature: body.signature,
            nonce: body.nonce,
        };
        if punish_verify(
            &signature,
            user.clone(),
            amount,
            proof_id,
            &*req.state().nonce_manager,
        )
        .is_err()
        {
            return Ok(Response::builder(400)
                .body(json!({"error": "signature verification failed"}))
                .content_type(mime::JSON)
                .build());
        }
    }

    punish(
        &req.state().identity_service,
        user.clone(),
        moderator.clone(),
        amount,
        proof_id,
    );

    let user_balance = balance(&req.state().identity_service, &user).await;
    let response: HashMap<String, serde_json::Value> = HashMap::from([
        ("user".into(), user.into()),
        ("from".into(), moderator.into()),
        ("idt".into(), user_balance.to_string().into()),
        ("proof_id".into(), proof_id.to_string().into()),
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

    use super::*;
    use crate::{
        admins::AdminStorage,
        identity::{
            IdentityService,
            proof::prove,
            tests::{MODERATOR, PROOF_ID, USER_A},
        },
        verify::{nonce::InMemoryNonceManager, punish::punish_sign, random_keypair},
    };
    use serde_json::Value;
    use tide::http::{Request as HttpRequest, Response, Url};

    #[async_std::test]
    async fn test_basic_punish() {
        let (private_key, moderator) = random_keypair();
        let moderators = HashSet::from([moderator.clone()]);
        let admin_storage = Arc::new(AdminStorage::new(HashSet::new(), moderators));
        let state = State {
            identity_service: IdentityService::default(),
            admin_storage,
            nonce_manager: Arc::new(InMemoryNonceManager::default()),
        };
        let user_id = USER_A;
        let amount = 5000;

        let _ = prove(
            &state.identity_service,
            USER_A.to_string(),
            moderator.clone(),
            10000,
            PROOF_ID,
        );

        let req_url = format!("/punish/{user_id}");
        let signature = punish_sign(
            &private_key,
            user_id.to_string(),
            amount,
            PROOF_ID,
            &*state.nonce_manager,
        )
        .await
        .expect("Should sign successfully");
        let body = json!({
            "from": moderator,
            "amount": amount,
            "proof_id": PROOF_ID,
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
        server.at("/punish/:user").post(route);

        let mut response: Response = server.respond(req).await.unwrap();

        assert_eq!(response.status(), 200);
        let body: Value = response.body_json().await.unwrap();
        assert_eq!(body["user"], user_id);
        assert_eq!(body["from"], moderator);
        // 10000 IDT minus 5000 IDT penalty
        assert_eq!(body["idt"], "5000");
        assert_eq!(body["proof_id"], PROOF_ID.to_string());
        assert_eq!(body["nonce"], signature.nonce);
    }

    #[async_std::test]
    async fn test_bad_request_format() {
        let state = State::default();
        let user_id = USER_A;

        let req_url = format!("/punish/{user_id}");
        let body = json!({
            "from": MODERATOR,
            "wrong_field": "wrong_value",
            "proof_id": PROOF_ID
        });

        let mut req = HttpRequest::new(
            tide::http::Method::Post,
            Url::parse(&format!("http://example.com{}", req_url)).unwrap(),
        );
        req.set_body(serde_json::to_string(&body).unwrap());
        req.set_content_type(mime::JSON);

        let mut server = tide::with_state(state);
        server.at("/punish/:user").post(route);

        let response: Response = server.respond(req).await.unwrap();
        assert!(
            response.status().is_client_error(),
            "Expected a client error for bad request format"
        );
    }

    #[async_std::test]
    async fn test_unprivileged_request() {
        // create a random keypair for a non-privileged user
        let (private_key, _) = random_keypair();

        let moderators = HashSet::from(["other_moderator".to_string()]);
        let admin_storage = Arc::new(AdminStorage::new(HashSet::new(), moderators));

        let state = State {
            identity_service: IdentityService::default(),
            admin_storage,
            nonce_manager: Arc::new(InMemoryNonceManager::default()),
        };

        let target_user = "test_user".to_string();
        let amount = 5000;
        let req_url = format!("/punish/{target_user}");

        let signature = punish_sign(
            &private_key,
            target_user.clone(),
            amount,
            PROOF_ID,
            &*state.nonce_manager,
        )
        .await
        .expect("Should sign successfully");

        let body = json!({
            "from": signature.signer,
            "amount": amount,
            "proof_id": PROOF_ID,
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
        server.at("/punish/:user").post(route);

        let mut response: Response = server.respond(req).await.unwrap();
        assert_eq!(
            response.status(),
            403,
            "Expected a 403 Forbidden response for unprivileged request"
        );
        let body: Value = response.body_json().await.unwrap();
        assert_eq!(body["error"], "not moderator");
    }
}
