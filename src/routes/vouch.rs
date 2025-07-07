use std::collections::HashMap;

use serde::Deserialize;
use serde_json::json;
use tide::{Request, Response, http::mime};

#[cfg(test)]
use crate::servers::storage::ServerInfo;
use crate::{
    identity::{UserAddress, idt::balance, vouch::vouch},
    routes::State,
    verify::{nonce::Nonce, servers::server_verify, signature::Signature, vouch::vouch_verify},
};

#[derive(Deserialize)]
struct VouchRequest {
    from: UserAddress,
    signature: String,
    nonce: Nonce,
    server_signature: Option<Signature>,
}

pub async fn route(mut req: Request<State>) -> tide::Result {
    let vouchee = req.param("user")?.to_string();
    let body: VouchRequest = req.body_json().await?;
    let voucher = body.from;
    {
        let signature = Signature {
            signer: voucher.clone(),
            signature: body.signature.clone(),
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
        if let Some(server_sig) = &body.server_signature {
            let servers = req.state().server_storage.servers().await?;
            if !servers.contains_key(&server_sig.signer) {
                return Ok(Response::builder(403)
                    .body(json!({"error": "server not allowed"}))
                    .content_type(mime::JSON)
                    .build());
            }
            if server_verify(
                server_sig,
                signature.signature.clone(),
                &*req.state().nonce_manager,
            )
            .await
            .is_err()
            {
                return Ok(Response::builder(400)
                    .body(json!({"error": "server signature verification failed"}))
                    .content_type(mime::JSON)
                    .build());
            }
        }
    }
    vouch(
        &req.state().identity_service,
        voucher.clone(),
        vouchee.clone(),
    )
    .await?;
    let voucher_balance = balance(&req.state().identity_service, &voucher).await?;
    let response: HashMap<String, serde_json::Value> = HashMap::from([
        ("from".into(), voucher.into()),
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
        verify::{random_keypair, servers::server_sign, vouch::vouch_sign},
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
    async fn test_server_vouch() {
        let state = State::default();
        let (user_priv, user_address) = random_keypair();
        let (server_priv, server_addr) = random_keypair();
        state
            .server_storage
            .add_server(
                server_addr.clone(),
                ServerInfo {
                    url: "http://x".into(),
                    scale: 1.0,
                },
            )
            .await
            .unwrap();

        let user_b = "userB";

        let user_sig = vouch_sign(&user_priv, user_b.to_string(), &*state.nonce_manager)
            .await
            .expect("sign");
        let server_sig = server_sign(
            &server_priv,
            user_sig.signature.clone(),
            &*state.nonce_manager,
        )
        .await
        .expect("server sign");
        let body = json!({
            "from": user_address,
            "signature": user_sig.signature,
            "nonce": user_sig.nonce,
            "server_signature": server_sig,
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
        assert_eq!(response.status(), 200);
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
    async fn test_server_not_allowed() {
        let state = State::default();
        let (user_priv, user_address) = random_keypair();
        let (server_priv, _) = random_keypair();

        let user_b = "userB";

        let user_sig = vouch_sign(&user_priv, user_b.to_string(), &*state.nonce_manager)
            .await
            .expect("sign");
        let server_sig = server_sign(
            &server_priv,
            user_sig.signature.clone(),
            &*state.nonce_manager,
        )
        .await
        .expect("Server sign");
        let body = json!({
            "from": user_address,
            "signature": user_sig.signature,
            "nonce": user_sig.nonce,
            "server_signature": server_sig,
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
            "Expected a client error for request from an unallowed server"
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
            "from": user_address,
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
    async fn test_invalid_server_signature() {
        let state = State::default();
        let (user_priv, user_address) = random_keypair();
        let (server_priv, server_addr) = random_keypair();
        state
            .server_storage
            .add_server(
                server_addr.clone(),
                ServerInfo {
                    url: "http://x".into(),
                    scale: 1.0,
                },
            )
            .await
            .unwrap();

        let user_b = "userB";

        // generate valid user signature
        let user_sig = vouch_sign(&user_priv, user_b.to_string(), &*state.nonce_manager)
            .await
            .expect("User sign");

        // generate valid server signature
        let server_sig = server_sign(
            &server_priv,
            user_sig.signature.clone(),
            &*state.nonce_manager,
        )
        .await
        .expect("Server sign");

        // tamper with server signature
        let mut invalid_server_sig = server_sig.clone();
        invalid_server_sig.signature.push_str("bad");

        let body = json!({
            "from": user_address,
            "signature": user_sig.signature,
            "nonce": user_sig.nonce,
            "server_signature": invalid_server_sig,
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
            "Expected a client error for invalid server signature"
        );
    }
}
