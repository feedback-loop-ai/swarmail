//! Inbucket's REST API v1, cited from the captured example client
//! (`sources/inbucket/inbucket-rest-apiv1.sh`): a mailbox per recipient
//! (named by the address's local part), listed whole, a message's source by
//! id, delete by id and purge of one mailbox. The narrowest clear is
//! therefore one recipient's mailbox.

use super::{
    Capability, Cite, ClearScope, Conn, Fetched, Method, OpError, Operation, Page, Paging, Scope,
    Spend, Summary, addresses, escape, id_text, mime, scan, supported,
};
use serde_json::Value;
use tokio::time::Instant;

const DOC: &str = "inbucket-rest-apiv1.sh";
const LIST: Cite = Cite {
    doc: DOC,
    needle: "list <mailbox>           - list mailbox contents",
};
const SOURCE: Cite = Cite {
    doc: DOC,
    needle: "url=\"$URL_ROOT/mailbox/$1/$2/source\"",
};
const DELETE: Cite = Cite {
    doc: DOC,
    needle: "delete <mailbox> <id>    - delete message",
};
const PURGE: Cite = Cite {
    doc: DOC,
    needle: "purge <mailbox>          - delete all messages in mailbox",
};

pub fn capability(op: Operation) -> Capability {
    match op {
        Operation::List | Operation::Search => {
            supported(Method::Inbox, &["GET /api/v1/mailbox/{mailbox}"], &[LIST])
        }
        Operation::Get => supported(
            Method::ById,
            &["GET /api/v1/mailbox/{mailbox}/{id}/source"],
            &[SOURCE],
        ),
        Operation::Delete => supported(
            Method::ById,
            &["DELETE /api/v1/mailbox/{mailbox}/{id}"],
            &[DELETE],
        ),
        Operation::Clear => Capability::Supported {
            method: Method::Inbox,
            routes: vec!["DELETE /api/v1/mailbox/{mailbox}"],
            cites: vec![PURGE],
            scope: Some(ClearScope::Recipient),
        },
        _ => Capability::NotInspected {
            why: "derived elsewhere",
        },
    }
}

/// One unpaged mailbox listing: its length is its own count.
pub fn decode_page(v: &Value, _start: u64) -> Option<Page> {
    let items = v
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
    Some((items.len() as u64, items))
}

pub async fn list(
    c: &Conn,
    scope: &Scope,
    deadline: Instant,
    spend: &mut Spend,
) -> Result<Vec<Summary>, OpError> {
    let m = escape(scope.local());
    scan(
        c,
        Paging::Whole,
        |_, _, _| format!("/api/v1/mailbox/{m}"),
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
    let path = format!(
        "/api/v1/mailbox/{}/{}/source",
        escape(scope.local()),
        escape(id)
    );
    let raw = c.get_bytes(&path, deadline, spend).await?;
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

/// Delete one message, or purge the whole mailbox when `id` is `None`.
pub async fn delete(
    c: &Conn,
    scope: &Scope,
    id: Option<&str>,
    deadline: Instant,
    spend: &mut Spend,
) -> Result<(), OpError> {
    let mut path = format!("/api/v1/mailbox/{}", escape(scope.local()));
    if let Some(id) = id {
        path.push('/');
        path.push_str(&escape(id));
    }
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
