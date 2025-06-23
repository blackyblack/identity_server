use std::collections::HashMap;

use serde::Deserialize;
use serde_json::json;
use tide::{Request, Response, http::mime};

use crate::identity::{IdentityService, UserAddress, forget::forget, idt::balance};

#[derive(Deserialize)]
struct ForgetRequest {
    from: UserAddress,
}

pub async fn route(mut req: Request<IdentityService>) -> tide::Result {
    let vouchee = req.param("user")?.to_string();
    let body: ForgetRequest = req.body_json().await?;
    let voucher = body.from;
    forget(req.state(), voucher.clone(), vouchee.clone()).await;
    let voucher_balance = balance(req.state(), &voucher).await;
    let response: HashMap<String, serde_json::Value> = HashMap::from([
        ("from".into(), voucher.into()),
        ("to".into(), vouchee.into()),
        ("idt".into(), voucher_balance.to_string().into()),
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
        vouch::vouch,
    };
    use serde_json::Value;
    use tide::http::{Request as HttpRequest, Response, Url};

    #[async_std::test]
    async fn test_basic_forget() {
        let service = IdentityService::default();
        let user_b = "userB";
        let _ = prove(
            &service,
            USER_A.to_string(),
            MODERATOR.to_string(),
            10000,
            PROOF_ID,
        );
        vouch(&service, USER_A.to_string(), user_b.to_string());

        let req_url = format!("/forget/{user_b}");
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
        server.at("/forget/:user").post(route);

        let mut response: Response = server.respond(req).await.unwrap();

        assert_eq!(response.status(), 200);
        let body: Value = response.body_json().await.unwrap();
        assert_eq!(body["from"], USER_A);
        assert_eq!(body["to"], user_b);
        // 500 IDT penalty for forgetting
        assert_eq!(body["idt"], "9500");
    }

    #[async_std::test]
    async fn test_bad_request_format() {
        let service = IdentityService::default();
        let user_b = "userB";

        // bad JSON format (missing "from" field)
        let req_url = format!("/forget/{user_b}");
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
        server.at("/forget/:user").post(route);

        let response: Response = server.respond(req).await.unwrap();
        assert!(
            response.status().is_client_error(),
            "Expected a client error for bad request format"
        );
    }
}
