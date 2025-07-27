use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::json;
use tide::{Request, Response, http::mime};

use crate::{
    identity::{UserAddress, idt::balance, vouch::vouch, vouch_external::vouch_external},
    routes::State,
    verify::{nonce::Nonce, vouch::vouch_verify},
};

#[derive(Deserialize, Serialize, Clone)]
struct FromField {
    user: UserAddress,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    server: Option<UserAddress>,
}

#[derive(Deserialize)]
struct VouchRequest {
    from: FromField,
    signature: String,
    nonce: Nonce,
}

pub async fn route(mut req: Request<State>) -> tide::Result {
    let vouchee = req.param("user")?.to_string();
    let body: VouchRequest = req.body_json().await?;
    let voucher = body.from;
    let voucher_user = voucher.user.clone();
    if vouch_verify(
        body.signature,
        &voucher_user,
        body.nonce,
        vouchee.clone(),
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
    if let Some(server) = voucher.server.clone() {
        vouch_external(
            &req.state().identity_service,
            server,
            voucher_user.clone(),
            vouchee.clone(),
        )
        .await?;
    } else {
        vouch(
            &req.state().identity_service,
            voucher_user.clone(),
            vouchee.clone(),
        )
        .await?;
    }
    let voucher_balance = balance(&req.state().identity_service, &voucher_user).await?;
    let response: HashMap<String, serde_json::Value> = HashMap::from([
        ("from".into(), serde_json::to_value(&voucher)?),
        ("to".into(), vouchee.into()),
        ("idt".into(), voucher_balance.to_string().into()),
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
        identity::{
            proof::prove,
            tests::{MODERATOR, PROOF_ID, USER_A},
        },
        verify::{random_keypair, vouch::vouch_sign},
    };
    use serde_json::Value;
    use tide::http::{Request as HttpRequest, Response, Url};

    //TODO: add test that vouch from external server affects vouchee balance,
    //      requires `add_server` to be implemented first

    #[async_std::test]
    async fn test_basic_vouch() {
        let state = State::default();
        let (private_key, user_address) = random_keypair();
        let user_b = "userB";
        prove(
            &state.identity_service,
            user_address.clone(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
        )
        .await
        .unwrap();

        let req_url = format!("/vouch/{user_b}");
        let signature = vouch_sign(&private_key, user_b.to_string(), &*state.nonce_manager)
            .await
            .expect("Should sign successfully");
        let body = json!({
            "from": {"user": user_address},
            "signature": signature.signature,
            "nonce": signature.nonce,
        });

        let mut req = HttpRequest::new(
            tide::http::Method::Post,
            Url::parse(&format!("http://example.com{}", req_url)).unwrap(),
        );
        req.set_body(body);
        req.set_content_type(mime::JSON);

        let mut server = tide::with_state(state);
        server.at("/vouch/:user").post(route);

        let mut response: Response = server.respond(req).await.unwrap();

        assert_eq!(response.status(), 200);
        let body: Value = response.body_json().await.unwrap();
        assert!(body["from"].get("server").is_none());
        assert_eq!(body["from"]["user"], user_address);
        assert_eq!(body["to"], user_b);
        // user A balance
        assert_eq!(body["idt"], "100");
        assert_eq!(body["nonce"], signature.nonce);
    }

    #[async_std::test]
    async fn test_bad_request_format() {
        let state = State::default();
        let user_b = "userB";

        // bad JSON format (missing "from" field)
        let req_url = format!("/vouch/{user_b}");
        let body = json!({
            "wrong_field": USER_A
        });

        let mut req = HttpRequest::new(
            tide::http::Method::Post,
            Url::parse(&format!("http://example.com{}", req_url)).unwrap(),
        );
        req.set_body(body);
        req.set_content_type(mime::JSON);

        let mut server = tide::with_state(state);
        server.at("/vouch/:user").post(route);

        let response: Response = server.respond(req).await.unwrap();
        assert!(
            response.status().is_client_error(),
            "Expected a client error for bad request format"
        );
    }

    #[async_std::test]
    async fn test_invalid_user_signature() {
        let state = State::default();
        let (private_key, user_address) = random_keypair();
        let user_b = "userB";

        // generate valid signature
        let mut signature = vouch_sign(&private_key, user_b.to_string(), &*state.nonce_manager)
            .await
            .expect("Should sign successfully");

        // tamper with the signature
        signature.signature.push_str("bad");

        let body = json!({
            "from": {"user": user_address},
            "signature": signature.signature,
            "nonce": signature.nonce,
        });

        let req_url = format!("/vouch/{user_b}");
        let mut req = HttpRequest::new(
            tide::http::Method::Post,
            Url::parse(&format!("http://example.com{}", req_url)).unwrap(),
        );
        req.set_body(body);
        req.set_content_type(mime::JSON);

        let mut server = tide::with_state(state);
        server.at("/vouch/:user").post(route);

        let response: Response = server.respond(req).await.unwrap();
        assert!(
            response.status().is_client_error(),
            "Expected a client error for invalid signature"
        );
    }

    #[async_std::test]
    async fn test_external_server() {
        let state = State::default();
        let (private_key, user_address) = random_keypair();
        let user_b = "userB";
        prove(
            &state.identity_service,
            user_address.clone(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
        )
        .await
        .unwrap();

        let req_url = format!("/vouch/{user_b}");
        let signature = vouch_sign(&private_key, user_b.to_string(), &*state.nonce_manager)
            .await
            .expect("Should sign successfully");
        let body = json!({
            "from": {"user": user_address, "server": "server1"},
            "signature": signature.signature,
            "nonce": signature.nonce,
        });

        let mut req = HttpRequest::new(
            tide::http::Method::Post,
            Url::parse(&format!("http://example.com{}", req_url)).unwrap(),
        );
        req.set_body(body);
        req.set_content_type(mime::JSON);

        let mut server = tide::with_state(state);
        server.at("/vouch/:user").post(route);

        let mut response: Response = server.respond(req).await.unwrap();

        assert_eq!(response.status(), 200);
        let body: Value = response.body_json().await.unwrap();
        assert_eq!(body["from"]["user"], user_address);
        assert_eq!(body["from"]["server"], "server1");
    }
}
