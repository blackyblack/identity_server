use std::collections::HashMap;

use serde::Deserialize;
use serde_json::json;
use tide::{Request, Response, http::mime};

use crate::identity::{IdentityService, UserAddress, idt::balance, vouch::vouch};

#[derive(Deserialize)]
struct VouchRequest {
    from: UserAddress,
}

pub async fn route(mut req: Request<IdentityService>) -> tide::Result {
    let vouchee = req.param("user")?.to_string();
    let body: VouchRequest = req.body_json().await?;
    let voucher = body.from;
    vouch(req.state(), voucher.clone(), vouchee.clone());
    let vouchee_balance = balance(req.state(), &vouchee).await;
    let response: HashMap<String, serde_json::Value> = HashMap::from([
        ("from".into(), voucher.into()),
        ("to".into(), vouchee.into()),
        ("idt".into(), vouchee_balance.to_string().into()),
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
    use crate::identity::{
        proof::prove,
        tests::{MODERATOR, PROOF_ID, USER_A},
    };
    use serde_json::Value;
    use tide::http::{Request as HttpRequest, Response, Url};

    #[async_std::test]
    async fn test_basic_vouch() {
        let service = IdentityService::default();
        let user_b = "userB";
        let _ = prove(
            &service,
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
        );

        let req_url = format!("/vouch/{user_b}");
        let body = json!({
            "from": USER_A
        });

        let mut req = HttpRequest::new(
            tide::http::Method::Post,
            Url::parse(&format!("http://example.com{}", req_url)).unwrap(),
        );
        req.set_body(body);
        req.set_content_type(mime::JSON);

        let mut server = tide::with_state(service);
        server.at("/vouch/:user").post(route);

        let mut response: Response = server.respond(req).await.unwrap();

        assert_eq!(response.status(), 200);
        let body: Value = response.body_json().await.unwrap();
        assert_eq!(body["from"], USER_A);
        assert_eq!(body["to"], user_b);
        // 10% from 100 IDT of the user A
        assert_eq!(body["idt"], "10");
    }

    #[async_std::test]
    async fn test_bad_request_format() {
        let service = IdentityService::default();
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

        let mut server = tide::with_state(service);
        server.at("/vouch/:user").post(route);

        let response: Response = server.respond(req).await.unwrap();
        assert!(
            response.status().is_client_error(),
            "Expected a client error for bad request format"
        );
    }
}
