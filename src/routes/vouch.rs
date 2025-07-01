use std::collections::HashMap;

use serde::Deserialize;
use serde_json::json;
use tide::{Request, Response, http::mime};

use crate::{
    identity::{UserAddress, idt::balance, vouch::vouch},
    routes::State,
    verify::{signature::Signature, vouch::vouch_verify},
};

#[derive(Deserialize)]
struct VouchRequest {
    from: UserAddress,
    signature: String,
    nonce: u64,
}

pub async fn route(mut req: Request<State>) -> tide::Result {
    let vouchee = req.param("user")?.to_string();
    let body: VouchRequest = req.body_json().await?;
    let voucher = body.from;
    let current_nonce = req.state().nonce_manager.nonce(&voucher).await?;
    {
        let signature = Signature {
            signer: voucher.clone(),
            signature: body.signature,
            nonce: body.nonce,
        };
        if vouch_verify(&signature, vouchee.clone(), &*req.state().nonce_manager)
            .await
            .is_err()
        {
            return Ok(Response::builder(400)
                .body(json!({"error": "signature verification failed"}))
                .content_type(mime::JSON)
                .build());
        }
    }
    vouch(
        &req.state().identity_service,
        voucher.clone(),
        vouchee.clone(),
    );
    let voucher_balance = balance(&req.state().identity_service, &voucher).await;
    let response: HashMap<String, serde_json::Value> = HashMap::from([
        ("from".into(), voucher.into()),
        ("to".into(), vouchee.into()),
        ("idt".into(), voucher_balance.to_string().into()),
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

    #[async_std::test]
    async fn test_basic_vouch() {
        let state = State::default();
        let (private_key, user_address) = random_keypair();
        let user_b = "userB";
        let _ = prove(
            &state.identity_service,
            user_address.clone(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
        );

        let req_url = format!("/vouch/{user_b}");
        let signature = vouch_sign(&private_key, user_b.to_string(), &*state.nonce_manager)
            .await
            .expect("Should sign successfully");
        let body = json!({
            "from": user_address,
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
        assert_eq!(body["from"], user_address);
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
}
