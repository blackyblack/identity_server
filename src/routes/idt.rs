use std::collections::HashMap;

use serde_json::json;
use tide::{Request, Response, http::mime};

use crate::identity::{IdentityService, idt::balance};

pub async fn route(req: Request<IdentityService>) -> tide::Result {
    let user = req.param("user")?;
    let balance = balance(req.state(), &user.to_string()).await;
    let response: HashMap<String, serde_json::Value> = HashMap::from([
        ("user".into(), user.into()),
        ("idt".into(), balance.to_string().into()),
    ]);
    let response = Response::builder(200)
        .body(json!(response))
        .content_type(mime::JSON)
        .build();
    Ok(response)
}
