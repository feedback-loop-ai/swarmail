//! smtp4dev's API, cited from the pinned archive's own API documentation
//! (`sources/smtp4dev/smtp4dev-api.xml`, `Rnwood.Smtp4dev.xml`). That file
//! documents controller members, not route templates: each citation binds
//! the member (`GetSummaries` with its `searchTerms`, `page` and `pageSize`,
//! `GetMessageSourceRaw`, `Delete`, `DeleteAll`), and the templates under
//! `/api/messages` are the ones the wire driver checked against the pinned
//! executable (2026-11-07) and the real API proof exercises. `searchTerms`
//! matches subject, from, to, cc, body and attachment names, so every
//! answer is matched exactly on the client. `DeleteAll` clears everything.

use super::{
    Capability, Cite, ClearScope, Conn, Fetched, Method, OpError, Operation, Page, Paging, Spend,
    Summary, addresses, escape, id_text, mime, scan, supported,
};
use serde_json::Value;
use tokio::time::Instant;

pub const PAGE: u32 = 250;
const DOC: &str = "smtp4dev-api.xml";
const SUMMARIES: Cite = Cite {
    doc: DOC,
    needle: "MessagesController.GetSummaries(System.String,System.String,System.String,System.String,System.Boolean,System.Int32,System.Int32)",
};
const TERMS: Cite = Cite {
    doc: DOC,
    needle: "<param name=\"searchTerms\">Case insensitive term to search for in subject, from, to, cc, body content, and attachment filenames</param>",
};
const RAW: Cite = Cite {
    doc: DOC,
    needle: "MessagesController.GetMessageSourceRaw(System.Guid)",
};
const DELETE: Cite = Cite {
    doc: DOC,
    needle: "MessagesController.Delete(System.Guid)",
};
const DELETE_ALL: Cite = Cite {
    doc: DOC,
    needle: "MessagesController.DeleteAll(System.String)",
};

pub fn capability(op: Operation) -> Capability {
    match op {
        Operation::List | Operation::Search => supported(
            Method::ServerFilter,
            &["GET /api/messages?searchTerms&page&pageSize"],
            &[SUMMARIES, TERMS],
        ),
        Operation::Get => supported(Method::ById, &["GET /api/messages/{id}/raw"], &[RAW]),
        Operation::Delete => supported(Method::ById, &["DELETE /api/messages/{id}"], &[DELETE]),
        Operation::Clear => Capability::Supported {
            method: Method::ById,
            routes: vec!["DELETE /api/messages/*"],
            cites: vec![DELETE_ALL],
            scope: Some(ClearScope::All),
        },
        _ => Capability::NotInspected {
            why: "derived elsewhere",
        },
    }
}

pub fn decode_page(v: &Value, start: u64) -> Option<Page> {
    let page_number = start / u64::from(PAGE) + 1;
    if !start.is_multiple_of(u64::from(PAGE)) || v.get("currentPage")?.as_u64()? != page_number {
        return None;
    }
    let total = v.get("rowCount")?.as_u64()?;
    let items = v
        .get("results")?
        .as_array()?
        .iter()
        .map(|m| {
            Some(Summary {
                id: id_text(m.get("id")?)?,
                to: addresses(m.get("to")?, None)?,
                subject: m.get("subject")?.as_str()?.to_owned(),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some((total, items))
}

/// Every summary `terms` matches (all of them for an empty term).
pub async fn search(
    c: &Conn,
    terms: &str,
    deadline: Instant,
    spend: &mut Spend,
) -> Result<Vec<Summary>, OpError> {
    let filter = if terms.is_empty() {
        String::new()
    } else {
        format!("searchTerms={}&", escape(terms))
    };
    scan(
        c,
        Paging::Number { size: PAGE },
        |_, page, size| format!("/api/messages?{filter}page={page}&pageSize={size}"),
        decode_page,
        deadline,
        spend,
    )
    .await
}

pub async fn get(
    c: &Conn,
    id: &str,
    deadline: Instant,
    spend: &mut Spend,
) -> Result<Fetched, OpError> {
    let raw = c
        .get_bytes(
            &format!("/api/messages/{}/raw", escape(id)),
            deadline,
            spend,
        )
        .await?;
    let p = mime::parse(&raw).ok_or(OpError::Malformed)?;
    Ok(Fetched {
        id: id.to_owned(),
        subject: p.subject,
        from: p.from,
        to: p.to,
        text: p.text,
        html: p.html,
        codes: None,
        links: None,
    })
}

pub async fn delete(
    c: &Conn,
    id: &str,
    deadline: Instant,
    spend: &mut Spend,
) -> Result<(), OpError> {
    let path = format!("/api/messages/{}", escape(id));
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

pub async fn clear(c: &Conn, deadline: Instant, spend: &mut Spend) -> Result<(), OpError> {
    c.mutate(
        reqwest::Method::DELETE,
        "/api/messages/*",
        None,
        &[200, 204],
        deadline,
        spend,
    )
    .await
}
