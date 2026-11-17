//! MailSlurper's service API, for the official release, the source build and
//! Alkemio's image, each cited from its own captured documents: the route
//! registrations (`mailslurper-setup-services-listener.go`, where captured)
//! and the service controller, whose `GetMailCollection` reads the
//! `pageNumber` and `to` query parameters. Pages hold a fixed 50 items.
//!
//! MailSlurper keeps one decoded body per message and no raw source; v3's
//! fidelity record documents that, so a read that misses a part is
//! `inexact_fidelity`, not `wrong_result`. Its only delete route prunes by
//! date (`DeleteMailsAfterDate`): there is no delete by id. The prune codes
//! that would clear everything are defined outside the captured files, so
//! `clear` is `not_inspected` until a lock v5 capture states them.

use super::{
    Capability, Cite, Conn, Fetched, Method, OpError, Operation, Page, Paging, Scope, Spend,
    Summary, addresses, escape, id_text, mime, scan, supported,
};
use serde_json::Value;
use tokio::time::Instant;

/// The server's fixed page size.
pub const PAGE: u32 = 50;

/// The citations of one MailSlurper product: its own document names.
fn docs(target: &str) -> (&'static str, Option<&'static str>) {
    match target {
        "alkemio-mailslurper" => ("alkemio-service-controller.go", None),
        _ => (
            "mailslurper-service-controller.go",
            Some("mailslurper-setup-services-listener.go"),
        ),
    }
}

pub fn capability(target: &str, op: Operation) -> Capability {
    let (controller, listener) = docs(target);
    let to = Cite {
        doc: controller,
        needle: "To:      context.QueryParam(\"to\"),",
    };
    let page = Cite {
        doc: controller,
        needle: "context.QueryParam(\"pageNumber\")",
    };
    let read = Cite {
        doc: controller,
        needle: "GET: /mail/{id}",
    };
    let prune = Cite {
        doc: controller,
        needle: "DeleteMailsAfterDate(startDate)",
    };
    let registered = |needle: &'static str| listener.map(|doc| Cite { doc, needle });
    match op {
        Operation::List | Operation::Search => supported(
            Method::ServerFilter,
            &["GET /mail?pageNumber&to"],
            &[to, page],
        ),
        Operation::Get => {
            let mut cites = vec![read];
            cites.extend(registered("service.GET(\"/mail/:id\""));
            Capability::Supported {
                method: Method::ById,
                routes: vec!["GET /mail/{id}"],
                cites,
                scope: None,
            }
        }
        Operation::Delete => {
            let mut cites = vec![prune];
            cites.extend(registered("service.DELETE(\"/mail\""));
            Capability::NotSupported {
                reason: "the only delete route prunes by date; no delete by id is registered",
                cites,
            }
        }
        Operation::Clear => Capability::NotInspected {
            why: "the prune codes that would clear every message are not in a captured document",
        },
        _ => Capability::NotInspected {
            why: "derived elsewhere",
        },
    }
}

pub fn decode_page(v: &Value, start: u64) -> Option<Page> {
    if !start.is_multiple_of(u64::from(PAGE)) {
        return None;
    }
    let total = v.get("totalRecords")?.as_u64()?;
    // MailSlurper leaves `mailItems` out (or null) on an empty page.
    let items = match v.get("mailItems") {
        None | Some(Value::Null) => Vec::new(),
        Some(list) => list
            .as_array()?
            .iter()
            .map(|m| {
                Some(Summary {
                    id: id_text(m.get("id")?)?,
                    to: addresses(m.get("toAddresses")?, None)?,
                    subject: m.get("subject")?.as_str()?.to_owned(),
                })
            })
            .collect::<Option<Vec<_>>>()?,
    };
    Some((total, items))
}

pub async fn to(
    c: &Conn,
    scope: &Scope,
    deadline: Instant,
    spend: &mut Spend,
) -> Result<Vec<Summary>, OpError> {
    let q = escape(&scope.address);
    scan(
        c,
        Paging::Number { size: PAGE },
        |_, page, _| format!("/mail?pageNumber={page}&to={q}"),
        decode_page,
        deadline,
        spend,
    )
    .await
}

pub async fn all(c: &Conn, deadline: Instant, spend: &mut Spend) -> Result<Vec<Summary>, OpError> {
    scan(
        c,
        Paging::Number { size: PAGE },
        |_, page, _| format!("/mail?pageNumber={page}"),
        decode_page,
        deadline,
        spend,
    )
    .await
}

/// One stored item: its decoded fields. `TextBody` and `HTMLBody` where the
/// server filled them, else the one `body` under its content type.
pub fn decode_message(v: &Value) -> Option<Fetched> {
    let s = |k: &str| v.get(k).and_then(Value::as_str).filter(|x| !x.is_empty());
    let (ctype, _) = mime::content_type(s("contentType").unwrap_or("text/plain"));
    let body = v.get("body")?.as_str()?.to_owned();
    let (mut text, mut html) = (
        s("TextBody").map(str::to_owned),
        s("HTMLBody").map(str::to_owned),
    );
    if text.is_none() && html.is_none() {
        match ctype.as_str() {
            "text/html" => html = Some(body),
            _ => text = Some(body),
        }
    }
    Some(Fetched {
        id: id_text(v.get("id")?)?,
        subject: Some(v.get("subject")?.as_str()?.to_owned()),
        from: s("fromAddress").map(super::bare),
        to: addresses(v.get("toAddresses")?, None)?,
        text,
        html,
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
        .get_json(&format!("/mail/{}", escape(id)), deadline, spend)
        .await?;
    decode_message(&v).ok_or(OpError::Malformed)
}
