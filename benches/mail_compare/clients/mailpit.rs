//! Mailpit's REST API, cited from the captured Swagger document
//! (`sources/mailpit/mailpit-swagger.json`). Its search query syntax is
//! documented only behind a link the lock does not capture, so no
//! recipient filter is used: `list` is a complete scan filtered on the
//! client, and `search` sends the subject token as a plain query (the
//! documented `GET /api/v1/search`) and matches recipient and subject
//! exactly on the client. A message is read from the documented
//! `GET /api/v1/message/{ID}`; delete by id and clear (every message) are
//! the documented `DELETE /api/v1/messages` with and without `IDs`.

use super::{
    Capability, Cite, ClearScope, Conn, Fetched, Method, OpError, Operation, Page, Paging, Spend,
    Summary, addresses, escape, id_text, scan, supported,
};
use serde_json::Value;
use tokio::time::Instant;

pub const PAGE: u32 = 250;
const DOC: &str = "mailpit-swagger.json";
const LIST: Cite = Cite {
    doc: DOC,
    needle: "\"/api/v1/messages\"",
};
const SEARCH: Cite = Cite {
    doc: DOC,
    needle: "\"/api/v1/search\"",
};
const READ: Cite = Cite {
    doc: DOC,
    needle: "\"/api/v1/message/{ID}\"",
};
const DELETE: Cite = Cite {
    doc: DOC,
    needle: "If no IDs are provided then all messages are deleted.",
};

pub fn capability(op: Operation) -> Capability {
    match op {
        Operation::List => supported(
            Method::ClientScan,
            &["GET /api/v1/messages?start&limit"],
            &[LIST],
        ),
        Operation::Search => supported(
            Method::ServerFilter,
            &["GET /api/v1/search?query&start&limit"],
            &[SEARCH],
        ),
        Operation::Get => supported(Method::ById, &["GET /api/v1/message/{ID}"], &[READ]),
        Operation::Delete => supported(
            Method::ById,
            &["DELETE /api/v1/messages {\"IDs\":[id]}"],
            &[DELETE],
        ),
        Operation::Clear => Capability::Supported {
            method: Method::ById,
            routes: vec!["DELETE /api/v1/messages {}"],
            cites: vec![DELETE],
            scope: Some(ClearScope::All),
        },
        _ => Capability::NotInspected {
            why: "derived elsewhere",
        },
    }
}

/// A listing or search page. Search keeps the mailbox size in `total` and
/// the matches in `messages_count`; both echo `start`, which must be the
/// position asked for.
pub fn decode_page(v: &Value, start: u64) -> Option<Page> {
    if v.get("start")?.as_u64()? != start {
        return None;
    }
    let total = v.get("messages_count")?.as_u64()?;
    let items = v
        .get("messages")?
        .as_array()?
        .iter()
        .map(|m| {
            Some(Summary {
                id: id_text(m.get("ID")?)?,
                to: addresses(m.get("To")?, Some("Address"))?,
                subject: m.get("Subject")?.as_str()?.to_owned(),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some((total, items))
}

pub async fn all(c: &Conn, deadline: Instant, spend: &mut Spend) -> Result<Vec<Summary>, OpError> {
    scan(
        c,
        Paging::Offset { size: PAGE },
        |start, _, size| format!("/api/v1/messages?start={start}&limit={size}"),
        decode_page,
        deadline,
        spend,
    )
    .await
}

pub async fn search(
    c: &Conn,
    token: &str,
    deadline: Instant,
    spend: &mut Spend,
) -> Result<Vec<Summary>, OpError> {
    let q = escape(token);
    scan(
        c,
        Paging::Offset { size: PAGE },
        |start, _, size| format!("/api/v1/search?query={q}&start={start}&limit={size}"),
        decode_page,
        deadline,
        spend,
    )
    .await
}

pub fn decode_message(v: &Value) -> Option<Fetched> {
    Some(Fetched {
        id: id_text(v.get("ID")?)?,
        subject: Some(v.get("Subject")?.as_str()?.to_owned()),
        from: v
            .get("From")
            .and_then(|f| f.get("Address"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        to: addresses(v.get("To")?, Some("Address"))?,
        // Mailpit states both bodies; an empty one is an absent part.
        text: Some(v.get("Text")?.as_str()?.to_owned()).filter(|s| !s.is_empty()),
        html: Some(v.get("HTML")?.as_str()?.to_owned()).filter(|s| !s.is_empty()),
        codes: None,
        links: None,
    })
}

pub async fn get(
    c: &Conn,
    id: &str,
    deadline: Instant,
    spend: &mut Spend,
) -> Result<Fetched, OpError> {
    let v = c
        .get_json(&format!("/api/v1/message/{}", escape(id)), deadline, spend)
        .await?;
    decode_message(&v).ok_or(OpError::Malformed)
}

/// Delete one message, or every message when `id` is `None`.
pub async fn delete(
    c: &Conn,
    id: Option<&str>,
    deadline: Instant,
    spend: &mut Spend,
) -> Result<(), OpError> {
    let body = match id {
        Some(id) => serde_json::json!({ "IDs": [id] }),
        None => serde_json::json!({}),
    };
    c.mutate(
        reqwest::Method::DELETE,
        "/api/v1/messages",
        Some(("application/json", body.to_string().into_bytes())),
        &[200],
        deadline,
        spend,
    )
    .await
}
