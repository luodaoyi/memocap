use std::io::Read as IoRead;
use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;
use serde::Deserialize;
use tiny_http::{Header, Method, Response, Server, StatusCode};

use crate::store;

#[derive(Debug)]
pub struct Incoming {
    pub method: String,
    pub path: String,
    pub query: String,
    pub token: Option<String>,
    pub body: String,
}

#[derive(Debug)]
pub struct Outgoing {
    pub status: u16,
    pub body: String,
}

#[derive(Debug, Deserialize)]
struct RememberIn {
    content: String,
    #[serde(default = "default_kind")]
    r#type: String,
    #[serde(default)]
    tags: String,
}

fn default_kind() -> String {
    "context".to_owned()
}

#[derive(Debug, Deserialize)]
struct ForgetIn {
    id: i64,
}

#[must_use]
pub fn token_ok(expected: &str, provided: Option<&str>) -> bool {
    let Some(got) = provided else {
        return false;
    };
    if expected.is_empty() || got.len() != expected.len() {
        return false;
    }
    got.bytes()
        .zip(expected.bytes())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

fn query_param(query: &str, key: &str) -> Option<String> {
    for part in query.split('&') {
        let Some((k, v)) = part.split_once('=') else {
            continue;
        };
        if k == key {
            return Some(url_decode(v));
        }
    }
    None
}

fn url_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = &value[i + 1..i + 3];
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' {
            out.push(b' ');
        } else {
            out.push(bytes[i]);
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn json(status: u16, body: impl serde::Serialize) -> Outgoing {
    Outgoing {
        status,
        body: serde_json::to_string(&body).unwrap_or_else(|_| "{\"error\":\"encode\"}".to_owned()),
    }
}

pub fn handle(connection: &Connection, expected_token: &str, req: &Incoming) -> Outgoing {
    if !token_ok(expected_token, req.token.as_deref()) {
        return json(401, serde_json::json!({"error": "unauthorized"}));
    }
    match (req.method.as_str(), req.path.as_str()) {
        ("POST", "/remember") => remember(connection, &req.body),
        ("GET", "/recall") => {
            let query = query_param(&req.query, "q").unwrap_or_default();
            let limit = query_param(&req.query, "limit")
                .and_then(|v| v.parse().ok())
                .unwrap_or(5);
            match store::recall(connection, &query, limit) {
                Ok(memories) => json(200, serde_json::json!({"memories": memories})),
                Err(error) => json(500, serde_json::json!({"error": error.to_string()})),
            }
        }
        ("GET", "/list") => {
            let limit = query_param(&req.query, "limit")
                .and_then(|v| v.parse().ok())
                .unwrap_or(20);
            match store::list(connection, limit) {
                Ok(memories) => json(200, serde_json::json!({"memories": memories})),
                Err(error) => json(500, serde_json::json!({"error": error.to_string()})),
            }
        }
        ("POST", "/forget") => forget(connection, &req.body),
        ("GET", "/count") => match store::count(connection) {
            Ok(count) => json(200, serde_json::json!({"count": count})),
            Err(error) => json(500, serde_json::json!({"error": error.to_string()})),
        },
        _ => json(404, serde_json::json!({"error": "not found"})),
    }
}

fn remember(connection: &Connection, body: &str) -> Outgoing {
    let parsed: RememberIn = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(error) => return json(400, serde_json::json!({"error": error.to_string()})),
    };
    match store::remember(
        connection,
        &parsed.content,
        &parsed.r#type,
        &parsed.tags,
        "global",
    ) {
        Ok(id) => json(200, serde_json::json!({"id": id})),
        Err(error) => json(400, serde_json::json!({"error": error.to_string()})),
    }
}

fn forget(connection: &Connection, body: &str) -> Outgoing {
    let parsed: ForgetIn = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(error) => return json(400, serde_json::json!({"error": error.to_string()})),
    };
    match store::forget(connection, parsed.id) {
        Ok(deleted) => json(200, serde_json::json!({"deleted": deleted})),
        Err(error) => json(500, serde_json::json!({"error": error.to_string()})),
    }
}

fn extract_token(headers: &[Header]) -> Option<String> {
    for header in headers {
        let name = header.field.as_str().as_str();
        if name.eq_ignore_ascii_case("authorization") {
            let value = header.value.as_str();
            return Some(
                value
                    .strip_prefix("Bearer ")
                    .unwrap_or(value)
                    .trim()
                    .to_owned(),
            );
        }
        if name.eq_ignore_ascii_case("x-memocap-token") {
            return Some(header.value.as_str().trim().to_owned());
        }
    }
    None
}

pub fn dispatch(
    method: &str,
    path: &str,
    authorized: bool,
    body: &str,
    database: &Path,
) -> (u16, String) {
    let connection = store::open(database).expect("open store");
    let (path, query) = path.split_once('?').unwrap_or((path, ""));
    let incoming = Incoming {
        method: method.to_owned(),
        path: path.to_owned(),
        query: query.to_owned(),
        token: authorized.then(|| "secret".to_owned()),
        body: body.to_owned(),
    };
    let outgoing = handle(&connection, "secret", &incoming);
    (outgoing.status, outgoing.body)
}

pub fn serve(bind: &str, token: &str, database: &Path) -> Result<()> {
    if token.trim().is_empty() {
        anyhow::bail!("MEMOCAP_TOKEN is required to serve");
    }
    let connection = store::open(database)?;
    let server = Server::http(bind).map_err(|error| anyhow::anyhow!(error.to_string()))?;
    for mut request in server.incoming_requests() {
        let url = request.url().to_string();
        let (path, query) = url.split_once('?').unwrap_or((url.as_str(), ""));
        let incoming = Incoming {
            method: method_name(request.method()),
            path: path.to_owned(),
            query: query.to_owned(),
            token: extract_token(request.headers()),
            body: {
                let mut body = String::new();
                IoRead::read_to_string(&mut request.as_reader(), &mut body).ok();
                body
            },
        };
        let outgoing = handle(&connection, token, &incoming);
        let response = Response::from_string(outgoing.body)
            .with_status_code(StatusCode(outgoing.status))
            .with_header(
                Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                    .expect("static header"),
            );
        request.respond(response).ok();
    }
    Ok(())
}

fn method_name(method: &Method) -> String {
    match method {
        Method::Get => "GET".to_owned(),
        Method::Post => "POST".to_owned(),
        other => format!("{other}").to_ascii_uppercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(token: Option<&str>) -> Incoming {
        Incoming {
            method: "POST".to_owned(),
            path: "/remember".to_owned(),
            query: String::new(),
            token: token.map(ToOwned::to_owned),
            body: "{\"content\":\"alpha\",\"type\":\"note\",\"tags\":\"\"}".to_owned(),
        }
    }

    #[test]
    fn token_reject_missing() {
        let dir = tempfile::tempdir().unwrap();
        let connection = store::open(&dir.path().join("db")).unwrap();
        let response = handle(&connection, "secret", &req(None));
        assert_eq!(response.status, 401);
        assert!(response.body.contains("unauthorized"));
        assert_eq!(store::count(&connection).unwrap(), 0);
    }

    #[test]
    fn token_reject_wrong() {
        let dir = tempfile::tempdir().unwrap();
        let connection = store::open(&dir.path().join("db")).unwrap();
        let response = handle(&connection, "secret", &req(Some("nope")));
        assert_eq!(response.status, 401);
        assert_eq!(store::count(&connection).unwrap(), 0);
    }

    #[test]
    fn token_ok_writes_same_store() {
        let dir = tempfile::tempdir().unwrap();
        let connection = store::open(&dir.path().join("db")).unwrap();
        let response = handle(&connection, "secret", &req(Some("secret")));
        assert_eq!(response.status, 200);
        assert_eq!(store::count(&connection).unwrap(), 1);
        let found = store::recall(&connection, "alpha", 5).unwrap();
        assert_eq!(found.len(), 1);
    }
}
