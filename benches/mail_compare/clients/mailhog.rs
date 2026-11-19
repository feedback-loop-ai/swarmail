//! MailHog's API v2, cited from the captured Swagger document
//! (`sources/mailhog/mailhog-swagger.json`): `GET /api/v2/messages` and
//! `GET /api/v2/search?kind=to|from|containing`. The pinned executable
//! answers with `items` where the document says `messages`; the client reads
//! what the executable sends, as the wire driver and the 0129 adapter do.
//!
//! No captured document states a route to one message, a delete or a clear:
//! MailHog's `APIv1.md` is not in the lock. So `get` scans the store for the
//! id (labelled `client_scan`), and `delete` and `clear` are
//! `not_inspected` until a lock v5 capture states them; they are never
//! published as `not_supported` on missing evidence (0132 §6.3).

use super::{
    Capability, Cite, Conn, Fetched, Method, OpError, Operation, Page, Paging, Spend, Summary,
    escape, id_text, mime, scan, supported,
};
use serde_json::Value;
use tokio::time::Instant;

pub const PAGE: u32 = 250;
const DOC: &str = "mailhog-swagger.json";
const SEARCH: Cite = Cite {
    doc: DOC,
    needle: "\"/api/v2/search\"",
};
const KINDS: Cite = Cite {
    doc: DOC,
    needle: "\"containing\"",
};
const LIST: Cite = Cite {
    doc: DOC,
    needle: "\"/api/v2/messages\"",
};

pub fn capability(op: Operation) -> Capability {
    match op {
        Operation::List | Operation::Search => supported(
            Method::ServerFilter,
            &["GET /api/v2/search?kind=to&query&start&limit"],
            &[SEARCH, KINDS],
        ),
        Operation::Get => supported(
            Method::ClientScan,
            &["GET /api/v2/messages?start&limit"],
            &[LIST],
        ),
        Operation::Delete | Operation::Clear => Capability::NotInspected {
            why: "MailHog's APIv1.md, which its README names for delete, is not captured in lock v5",
        },
        _ => Capability::NotInspected {
            why: "derived elsewhere",
        },
    }
}

/// The first header value as MailHog's `Content.Headers` keeps it.
fn header<'a>(m: &'a Value, name: &str) -> Option<&'a str> {
    m.get("Content")?
        .get("Headers")?
        .get(name)?
        .get(0)?
        .as_str()
}

pub fn decode_page(v: &Value, start: u64) -> Option<Page> {
    if v.get("start")?.as_u64()? != start {
        return None;
    }
    let total = v.get("total")?.as_u64()?;
    let items = v
        .get("items")?
        .as_array()?
        .iter()
        .map(|m| {
            Some(Summary {
                id: id_text(m.get("ID")?)?,
                to: header(m, "To")?
                    .split(',')
                    .map(super::bare)
                    .filter(|a| !a.is_empty())
                    .collect(),
                subject: header(m, "Subject").unwrap_or_default().to_owned(),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some((total, items))
}

pub async fn to(
    c: &Conn,
    scope: &super::Scope,
    deadline: Instant,
    spend: &mut Spend,
) -> Result<Vec<Summary>, OpError> {
    let q = escape(&scope.address);
    scan(
        c,
        Paging::Offset { size: PAGE },
        |start, _, size| format!("/api/v2/search?kind=to&query={q}&start={start}&limit={size}"),
        decode_page,
        deadline,
        spend,
    )
    .await
}

pub async fn all(c: &Conn, deadline: Instant, spend: &mut Spend) -> Result<Vec<Summary>, OpError> {
    scan(
        c,
        Paging::Offset { size: PAGE },
        |start, _, size| format!("/api/v2/messages?start={start}&limit={size}"),
        decode_page,
        deadline,
        spend,
    )
    .await
}

/// The item's raw message, read by the shared MIME reader.
pub fn decode_item(m: &Value) -> Option<Fetched> {
    let raw = m.get("Raw")?.get("Data")?.as_str()?;
    let p = mime::parse(raw.as_bytes())?;
    Some(Fetched {
        id: id_text(m.get("ID")?)?,
        subject: p.subject,
        from: p.from,
        to: p.to,
        text: p.text,
        html: p.html,
        codes: None,
        links: None,
    })
}

/// One message by id: the store paged until the id appears (MailHog
/// documents no read-by-id route). Paging keeps the scan's completeness
/// rules; an id the complete store does not hold is `status_404`.
pub async fn get(
    c: &Conn,
    id: &str,
    deadline: Instant,
    spend: &mut Spend,
) -> Result<Fetched, OpError> {
    let max_pages = super::MAX_STORE / u64::from(PAGE) + 2;
    let mut seen = 0u64;
    let mut expected = None;
    for _ in 0..max_pages {
        let v = c
            .get_json(
                &format!("/api/v2/messages?start={seen}&limit={PAGE}"),
                deadline,
                spend,
            )
            .await?;
        let (total, _) = decode_page(&v, seen).ok_or(OpError::Malformed)?;
        if *expected.get_or_insert(total) != total {
            return Err(OpError::Incomplete);
        }
        let items = v
            .get("items")
            .and_then(Value::as_array)
            .ok_or(OpError::Malformed)?;
        if let Some(m) = items
            .iter()
            .find(|m| m.get("ID").and_then(id_text).as_deref() == Some(id))
        {
            return decode_item(m).ok_or(OpError::Malformed);
        }
        seen += items.len() as u64;
        if seen >= total {
            return Err(OpError::Status(404));
        }
        if items.len() < PAGE as usize {
            return Err(OpError::Incomplete);
        }
    }
    Err(OpError::Oversized)
}
