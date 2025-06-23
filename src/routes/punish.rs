use std::collections::HashMap;

use serde::Deserialize;
use serde_json::json;
use tide::{Request, Response, http::mime};

use crate::identity::{
    IdentityService, IdtAmount, ProofId, UserAddress, idt::balance, punish::punish,
};

#[derive(Deserialize)]
struct PunishRequest {
    moderator: UserAddress,
    amount: IdtAmount,
    proof_id: ProofId,
}

pub async fn route(mut req: Request<IdentityService>) -> tide::Result {
    let user = req.param("user")?.to_string();
    let body: PunishRequest = req.body_json().await?;
    let moderator = body.moderator;
    let amount = body.amount;
    let proof_id = body.proof_id;

    punish(
        req.state(),
        user.clone(),
        moderator.clone(),
        amount,
        proof_id,
    );

    let user_balance = balance(req.state(), &user).await;
    let response: HashMap<String, serde_json::Value> = HashMap::from([
        ("user".into(), user.into()),
        ("moderator".into(), moderator.into()),
        ("idt".into(), user_balance.to_string().into()),
        ("proof_id".into(), proof_id.to_string().into()),
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
    async fn test_basic_punish() {
        let service = IdentityService::default();
        let user_id = USER_A;
        let amount = 5000;

        let _ = prove(
            &service,
            USER_A.to_string(),
            MODERATOR.to_string(),
            10000,
            PROOF_ID,
        );

        let req_url = format!("/punish/{user_id}");
        let body = json!({
            "moderator": MODERATOR,
            "amount": amount,
            "proof_id": PROOF_ID
        });

        let mut req = HttpRequest::new(
            tide::http::Method::Post,
            Url::parse(&format!("http://example.com{}", req_url)).unwrap(),
        );
        req.set_body(serde_json::to_string(&body).unwrap());
        req.set_content_type(mime::JSON);

        let mut server = tide::with_state(service);
        server.at("/punish/:user").post(route);

        let mut response: Response = server.respond(req).await.unwrap();

        assert_eq!(response.status(), 200);
        let body: Value = response.body_json().await.unwrap();
        assert_eq!(body["user"], user_id);
        assert_eq!(body["moderator"], MODERATOR);
        // 10000 IDT minus 5000 IDT penalty
        assert_eq!(body["idt"], "5000");
        assert_eq!(body["proof_id"], PROOF_ID.to_string());
    }

    #[async_std::test]
    async fn test_bad_request_format() {
        let service = IdentityService::default();
        let user_id = USER_A;

        let req_url = format!("/punish/{user_id}");
        let body = json!({
            "moderator": MODERATOR,
            "wrong_field": "wrong_value",
            "proof_id": PROOF_ID
        });

        let mut req = HttpRequest::new(
            tide::http::Method::Post,
            Url::parse(&format!("http://example.com{}", req_url)).unwrap(),
        );
        req.set_body(serde_json::to_string(&body).unwrap());
        req.set_content_type(mime::JSON);

        let mut server = tide::with_state(service);
        server.at("/punish/:user").post(route);

        let response: Response = server.respond(req).await.unwrap();
        assert!(
            response.status().is_client_error(),
            "Expected a client error for bad request format"
        );
    }
}
