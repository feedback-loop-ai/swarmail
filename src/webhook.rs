//! Outbound webhooks: queued, retried, never blocking the SMTP path.
//!
//! v0.1 speaks plain HTTP (the dominant dev-loop case — your test harness is
//! already listening on localhost). HTTPS targets land with the reqwest/rustls
//! switch in P3.

use crate::store::Store;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookTarget {
    /// Full URL, e.g. "http://127.0.0.1:9999/hooks/mail".
    pub url: String,
    /// Only fire for this inbox; None = every inbox.
    #[serde(default)]
    pub inbox: Option<String>,
    /// Sent verbatim as the X-Swarmail-Secret header.
    #[serde(default)]
    pub secret: Option<String>,
}

#[derive(Default)]
pub struct Webhooks {
    targets: RwLock<Vec<WebhookTarget>>,
}

impl Webhooks {
    pub fn set(&self, targets: Vec<WebhookTarget>) {
        *self.targets.write().unwrap() = targets;
    }

    pub fn current(&self) -> Vec<WebhookTarget> {
        self.targets.read().unwrap().clone()
    }
}

/// What a firehose wakeup means; a pure function so every arm is provable.
enum Wake {
    /// A new email to deliver.
    Mail(Arc<crate::model::Email>),
    /// A transient condition (lag) — keep looping, the store is the truth.
    Again,
    /// The channel closed — the store is gone; stop.
    Stop,
}

fn classify(
    res: Result<Arc<crate::model::Email>, tokio::sync::broadcast::error::RecvError>,
) -> Wake {
    match res {
        Ok(email) => Wake::Mail(email),
        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
            tracing::warn!(
                skipped = n,
                "webhook dispatcher lagged; store remains source of truth"
            );
            Wake::Again
        }
        Err(_) => Wake::Stop, // closed
    }
}

/// The dispatcher core: consume the firehose until it closes. Owned fn so
/// the wake arms are provable without a live server.
pub(crate) async fn dispatch_loop(
    mut rx: tokio::sync::broadcast::Receiver<Arc<crate::model::Email>>,
    webhooks: Arc<Webhooks>,
) {
    loop {
        match classify(rx.recv().await) {
            Wake::Mail(email) => {
                let targets = webhooks.current();
                for t in targets {
                    if t.inbox.as_deref().is_some_and(|i| i != email.inbox) {
                        continue;
                    }
                    let payload = serde_json::json!({
                        "event": "received",
                        "email": &*email,
                    });
                    let body = payload.to_string();
                    tokio::spawn(deliver(t.url, t.secret, body));
                }
            }
            Wake::Again => {}
            Wake::Stop => return,
        }
    }
}

/// Spawn the dispatcher: consume the global firehose, deliver with retry.
pub fn spawn_dispatcher(store: Arc<Store>, webhooks: Arc<Webhooks>) {
    let rx = store.subscribe_all();
    tokio::spawn(dispatch_loop(rx, webhooks));
}

async fn deliver(url: String, secret: Option<String>, body: String) {
    for attempt in 0..4u32 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(100 * 4u64.pow(attempt - 1))).await;
        }
        match post_json(&url, &body, secret.as_deref()).await {
            Ok(status) if (200..300).contains(&status) => return,
            Ok(status) => tracing::debug!(%url, status, "webhook non-2xx"),
            Err(e) => tracing::debug!(%url, error = %e, "webhook delivery failed"),
        }
    }
    tracing::warn!(%url, "webhook dropped after retries");
}

async fn post_json(url: &str, body: &str, secret: Option<&str>) -> Result<u16, String> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| "only http:// webhook targets are supported in v0.1".to_string())?;
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let mut stream = tokio::time::timeout(
        Duration::from_secs(5),
        TcpStream::connect(authority.to_string()),
    )
    .await
    .map_err(|_| "connect timeout".to_string())?
    .map_err(|e| e.to_string())?;

    let mut req = format!(
        "POST {path} HTTP/1.1\r\nHost: {authority}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    if let Some(s) = secret {
        req.push_str(&format!("X-Swarmail-Secret: {s}\r\n"));
    }
    req.push_str("\r\n");
    req.push_str(body);

    tokio::time::timeout(Duration::from_secs(5), stream.write_all(req.as_bytes()))
        .await
        .map_err(|_| "write timeout".to_string())?
        .map_err(|e| e.to_string())?;

    let mut buf = [0u8; 1024];
    let n = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buf))
        .await
        .map_err(|_| "read timeout".to_string())?
        .map_err(|e| e.to_string())?;
    let head = String::from_utf8_lossy(&buf[..n]);
    let status: u16 = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| "malformed response".to_string())?;
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Email;
    use tokio::sync::broadcast;

    fn email() -> Email {
        Email {
            id: "1".into(),
            inbox: "default".into(),
            from: None,
            to: vec![],
            cc: vec![],
            recipients: vec![],
            subject: None,
            received_at: String::new(),
            received_ms: 0,
            size: 0,
            text: None,
            html: None,
            links: vec![],
            codes: vec![],
            raw: vec![],
        }
    }

    /// Every arm of the firehose classifier, provable without a live server.
    #[test]
    fn classify_mail_again_stop() {
        assert!(matches!(classify(Ok(Arc::new(email()))), Wake::Mail(_)));
        assert!(matches!(
            classify(Err(broadcast::error::RecvError::Lagged(3))),
            Wake::Again
        ));
        assert!(matches!(
            classify(Err(broadcast::error::RecvError::Closed)),
            Wake::Stop
        ));
    }
}

#[cfg(test)]
mod loop_tests {
    use super::*;
    use crate::model::Email;
    use tokio::sync::broadcast;

    fn email() -> Email {
        Email {
            id: "1".into(),
            inbox: "default".into(),
            from: None,
            to: vec![],
            cc: vec![],
            recipients: vec![],
            subject: None,
            received_at: String::new(),
            received_ms: 0,
            size: 0,
            text: None,
            html: None,
            links: vec![],
            codes: vec![],
            raw: vec![],
        }
    }

    /// Mail is delivered, a lagging watcher retries, a closed firehose ends
    /// the loop — all three wake arms against a real broadcast channel.
    #[tokio::test]
    async fn dispatch_loop_handles_mail_lag_and_close() {
        let webhooks = Arc::new(Webhooks::default()); // no targets: delivery is a no-op
        let (tx, rx) = broadcast::channel(1);
        let loop_task = tokio::spawn(dispatch_loop(rx, webhooks));

        tx.send(Arc::new(email())).unwrap(); // Mail arm
        tx.send(Arc::new(email())).unwrap(); // capacity 1 → the next recv lags
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        drop(tx); // close the firehose → Stop → the loop returns

        tokio::time::timeout(std::time::Duration::from_secs(2), loop_task)
            .await
            .expect("the loop must end when the firehose closes")
            .unwrap();
    }
}
