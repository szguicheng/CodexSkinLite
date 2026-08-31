use serde_json::{Value, json};

pub fn command(id: u64, method: &str, params: Value) -> Value {
    json!({ "id": id, "method": method, "params": params })
}

pub fn evaluate_params(expression: &str) -> Value {
    json!({
        "expression": expression,
        "returnByValue": true,
        "awaitPromise": false,
        "userGesture": false
    })
}
