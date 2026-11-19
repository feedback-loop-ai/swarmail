//! Swarmail's REST API (the candidate). Each test's mail lands in its own
//! inbox, named by the SMTP AUTH identity (0004), so `list`, `search` and
//! `clear` address the measured inbox and never see the background fill;
//! codes and links are read from the fields ingest extracted (0005). Routes
//! cite the candidate's own route table and models (`repo:` paths).

use super::{
    Capability, Cite, ClearScope, Conn, Fetched, Method, OpError, Operation, Page, Paging, Scope,
    Spend, Summary, addresses, escape, id_text, scan, segment, supported,
};
use serde_json::Value;
use tokio::time::Instant;

/// The largest page the list route admits (`MAX_PAGE_SIZE`).
pub const PAGE: u32 = 250;

const ROUTES: Cite = Cite {
    doc: "repo:src/api.rs",
    needle: "\"/api/v1/inboxes/{inbox}/messages\",\n            get(list_messages).delete(clear_inbox),",
};
const BY_ID: Cite = Cite {
    doc: "repo:src/api.rs",
    needle: "\"/api/v1/messages/{id}\",\n            get(get_message).delete(delete_message),",
};
const FILTER: Cite = Cite {
    doc: "repo:src/api.rs",
    needle: "pub subject: Option<String>,",
};
const PAGE_SIZE: Cite = Cite {
    doc: "repo:src/api.rs",
    needle: "pub(crate) const MAX_PAGE_SIZE: usize = 250;",
};
const INGEST: Cite = Cite {
    doc: "repo:src/model.rs",
    needle: "pub codes: Vec<String>,",
};
const LINKS: Cite = Cite {
    doc: "repo:src/model.rs",
    needle: "pub links: Vec<String>,",
};
const INBOX: Cite = Cite {
    doc: "repo:docs/decisions/0004-per-test-inboxes-via-smtp-auth.md",
    needle: "AUTH",
};

pub fn capability(op: Operation) -> Capability {
    let list = "GET /api/v1/inboxes/{inbox}/messages?limit&offset";
    match op {
        Operation::List => supported(Method::Inbox, &[list], &[ROUTES, PAGE_SIZE, INBOX]),
        Operation::Search => supported(
            Method::Inbox,
            &["GET /api/v1/inboxes/{inbox}/messages?subject&limit&offset"],
            &[ROUTES, FILTER, PAGE_SIZE, INBOX],
        ),
        Operation::Get => supported(Method::ById, &["GET /api/v1/messages/{id}"], &[BY_ID]),
        Operation::ExtractCode => supported(
            Method::Ingest,
            &["GET /api/v1/messages/{id}"],
            &[BY_ID, INGEST],
        ),
        Operation::ExtractLink => supported(
            Method::Ingest,
            &["GET /api/v1/messages/{id}"],
            &[BY_ID, LINKS],
        ),
        Operation::Delete => supported(Method::ById, &["DELETE /api/v1/messages/{id}"], &[BY_ID]),
        Operation::Clear => Capability::Supported {
            method: Method::Inbox,
            routes: vec!["DELETE /api/v1/inboxes/{inbox}/messages"],
            cites: vec![ROUTES, INBOX],
            scope: Some(ClearScope::Recipient),
        },
        Operation::Wait => Capability::NotInspected {
            why: "the native wait belongs to story C",
        },
    }
}

pub fn decode_page(v: &Value, _start: u64) -> Option<Page> {
    let total = v.get("total")?.as_u64()?;
    let items = v
        .get("emails")?
        .as_array()?
        .iter()
        .map(|m| {
            Some(Summary {
                id: id_text(m.get("id")?)?,
                to: addresses(m.get("to")?, Some("address"))?,
                subject: m
                    .get("subject")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some((total, items))
}

pub async fn list(
    c: &Conn,
    scope: &Scope,
    subject: Option<&str>,
    deadline: Instant,
    spend: &mut Spend,
) -> Result<Vec<Summary>, OpError> {
    let inbox = escape(&scope.inbox);
    let filter = subject
        .map(|s| format!("&subject={}", escape(s)))
        .unwrap_or_default();
    scan(
        c,
        Paging::Offset { size: PAGE },
        |start, _, size| {
            format!("/api/v1/inboxes/{inbox}/messages?limit={size}&offset={start}{filter}")
        },
        decode_page,
        deadline,
        spend,
    )
    .await
}

pub fn decode_message(v: &Value) -> Option<Fetched> {
    let strings = |k: &str| -> Option<Vec<String>> {
        match v.get(k) {
            None => Some(Vec::new()),
            Some(Value::Array(a)) => a.iter().map(|x| x.as_str().map(str::to_owned)).collect(),
            Some(_) => None,
        }
    };
    Some(Fetched {
        id: id_text(v.get("id")?)?,
        subject: v.get("subject").and_then(Value::as_str).map(str::to_owned),
        from: v
            .get("from")
            .and_then(|f| f.get("address"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        to: addresses(v.get("to")?, Some("address"))?,
        text: v.get("text").and_then(Value::as_str).map(str::to_owned),
        html: v.get("html").and_then(Value::as_str).map(str::to_owned),
        // An absent list is a known-empty one (`skip_serializing_if`).
        codes: Some(strings("codes")?),
        links: Some(strings("links")?),
    })
}

pub async fn get(
    c: &Conn,
    id: &str,
    deadline: Instant,
    spend: &mut Spend,
) -> Result<Fetched, OpError> {
    let v = c
        .get_json(
            &format!("/api/v1/messages/{}", segment(id)?),
            deadline,
            spend,
        )
        .await?;
    decode_message(&v).ok_or(OpError::Malformed)
}

pub async fn delete(
    c: &Conn,
    id: &str,
    deadline: Instant,
    spend: &mut Spend,
) -> Result<(), OpError> {
    let path = format!("/api/v1/messages/{}", segment(id)?);
    c.mutate(
        reqwest::Method::DELETE,
        &path,
        None,
        &[200, 204],
        deadline,
        spend,
    )
    .await
}

pub async fn clear(
    c: &Conn,
    scope: &Scope,
    deadline: Instant,
    spend: &mut Spend,
) -> Result<(), OpError> {
    let path = format!("/api/v1/inboxes/{}/messages", escape(&scope.inbox));
    c.mutate(
        reqwest::Method::DELETE,
        &path,
        None,
        &[200, 204],
        deadline,
        spend,
    )
    .await
}
