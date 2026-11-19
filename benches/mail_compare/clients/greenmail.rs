//! GreenMail's REST API, cited from the pinned jar's OpenAPI document
//! (`sources/greenmail/greenmail-openapi.yml`; 0130's correction of record
//! and acceptance answer 8: v4 reads GreenMail over REST, v3 keeps POP3).
//! It lists one user's folder with every message's whole MIME source; there
//! is no route to one message, so `get` reads the user's folder and picks the
//! uid (a scan of the mailbox). It documents no delete of a message, and
//! its only clear is the purge of every mail.

use super::{
    Capability, Cite, ClearScope, Conn, Fetched, Method, OpError, Operation, Page, Paging, Scope,
    Spend, Summary, escape, id_text, mime, scan, supported,
};
use serde_json::Value;
use tokio::time::Instant;

const DOC: &str = "greenmail-openapi.yml";
const MESSAGES: Cite = Cite {
    doc: DOC,
    needle: "/api/user/{emailOrId}/messages/{folderName}:",
};
const SCHEMA: Cite = Cite {
    doc: DOC,
    needle: "                - mimeMessage",
};
const PURGE: Cite = Cite {
    doc: DOC,
    needle: "/api/mail/purge:",
};
const ONLY_GET: Cite = Cite {
    doc: DOC,
    needle: "operationId: get_user_messages",
};

pub fn capability(op: Operation) -> Capability {
    let folder = "GET /api/user/{email}/messages/INBOX";
    match op {
        Operation::List | Operation::Search => {
            supported(Method::Inbox, &[folder], &[MESSAGES, SCHEMA])
        }
        Operation::Get => supported(Method::ClientScan, &[folder], &[MESSAGES, SCHEMA]),
        Operation::Delete => Capability::NotSupported {
            reason: "the OpenAPI document lists no delete of a message, only the user folder read and the purge of all mail",
            cites: vec![ONLY_GET, PURGE],
        },
        Operation::Clear => Capability::Supported {
            method: Method::ById,
            routes: vec!["POST /api/mail/purge"],
            cites: vec![PURGE],
            scope: Some(ClearScope::All),
        },
        _ => Capability::NotInspected {
            why: "derived elsewhere",
        },
    }
}

fn message(m: &Value) -> Option<(Summary, Fetched)> {
    let id = id_text(m.get("uid")?)?;
    let p = mime::parse(m.get("mimeMessage")?.as_str()?.as_bytes())?;
    let summary = Summary {
        id: id.clone(),
        to: p.to.clone(),
        subject: p.subject.clone().unwrap_or_default(),
    };
    let fetched = Fetched {
        id,
        subject: p.subject,
        from: p.from,
        to: p.to,
        text: p.text,
        html: p.html,
        codes: None,
        links: None,
    };
    Some((summary, fetched))
}

/// One unpaged folder listing.
pub fn decode_page(v: &Value, _start: u64) -> Option<Page> {
    let items = v
        .as_array()?
        .iter()
        .map(|m| message(m).map(|(s, _)| s))
        .collect::<Option<Vec<_>>>()?;
    Some((items.len() as u64, items))
}

fn folder(scope: &Scope) -> String {
    format!("/api/user/{}/messages/INBOX", escape(&scope.address))
}

pub async fn list(
    c: &Conn,
    scope: &Scope,
    deadline: Instant,
    spend: &mut Spend,
) -> Result<Vec<Summary>, OpError> {
    let path = folder(scope);
    scan(
        c,
        Paging::Whole,
        |_, _, _| path.clone(),
        decode_page,
        deadline,
        spend,
    )
    .await
}

pub async fn get(
    c: &Conn,
    scope: &Scope,
    id: &str,
    deadline: Instant,
    spend: &mut Spend,
) -> Result<Fetched, OpError> {
    let v = c.get_json(&folder(scope), deadline, spend).await?;
    let all = v.as_array().ok_or(OpError::Malformed)?;
    let mut found = None;
    for m in all {
        let (s, f) = message(m).ok_or(OpError::Malformed)?;
        if s.id == id {
            if found.is_some() {
                return Err(OpError::Incomplete);
            }
            found = Some(f);
        }
    }
    found.ok_or(OpError::Status(404))
}

pub async fn clear(c: &Conn, deadline: Instant, spend: &mut Spend) -> Result<(), OpError> {
    c.mutate(
        reqwest::Method::POST,
        "/api/mail/purge",
        None,
        &[200],
        deadline,
        spend,
    )
    .await
}
