use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::json;

use crate::store::Memory;

#[derive(Debug, Deserialize)]
struct RememberReply {
    id: i64,
}

#[derive(Debug, Deserialize)]
struct MemoriesReply {
    memories: Vec<Memory>,
}

#[derive(Debug, Deserialize)]
struct ForgetReply {
    deleted: bool,
}

#[derive(Debug, Deserialize)]
struct CountReply {
    count: i64,
}

fn base(address: &str) -> String {
    address.trim().trim_end_matches('/').to_owned()
}

fn send(req: ureq::Request, token: &str, body: Option<serde_json::Value>) -> Result<String> {
    let req = req.set("Authorization", &format!("Bearer {token}"));
    let result = match body {
        Some(value) => req.send_json(value),
        None => req.call(),
    };
    match result {
        Ok(resp) => resp.into_string().context("read body"),
        Err(ureq::Error::Status(401, _)) => bail!("unauthorized: token rejected"),
        Err(ureq::Error::Status(code, resp)) => {
            let text = resp.into_string().unwrap_or_default();
            bail!("remote request failed ({code}): {text}")
        }
        Err(error) => Err(error.into()),
    }
}

pub fn remember(address: &str, token: &str, content: &str, kind: &str, tags: &str) -> Result<i64> {
    let text = send(
        ureq::post(&format!("{}/remember", base(address))),
        token,
        Some(json!({"content": content, "type": kind, "tags": tags})),
    )?;
    let reply: RememberReply = serde_json::from_str(&text).context("remember reply")?;
    Ok(reply.id)
}

pub fn recall(address: &str, token: &str, query: &str, limit: usize) -> Result<Vec<Memory>> {
    let text = send(
        ureq::get(&format!("{}/recall", base(address)))
            .query("q", query)
            .query("limit", &limit.to_string()),
        token,
        None,
    )?;
    let reply: MemoriesReply = serde_json::from_str(&text).context("recall reply")?;
    Ok(reply.memories)
}

pub fn list(address: &str, token: &str, limit: usize) -> Result<Vec<Memory>> {
    let text = send(
        ureq::get(&format!("{}/list", base(address))).query("limit", &limit.to_string()),
        token,
        None,
    )?;
    let reply: MemoriesReply = serde_json::from_str(&text).context("list reply")?;
    Ok(reply.memories)
}

pub fn forget(address: &str, token: &str, id: i64) -> Result<bool> {
    let text = send(
        ureq::post(&format!("{}/forget", base(address))),
        token,
        Some(json!({"id": id})),
    )?;
    let reply: ForgetReply = serde_json::from_str(&text).context("forget reply")?;
    Ok(reply.deleted)
}

pub fn count(address: &str, token: &str) -> Result<i64> {
    let text = send(ureq::get(&format!("{}/count", base(address))), token, None)?;
    let reply: CountReply = serde_json::from_str(&text).context("count reply")?;
    Ok(reply.count)
}
