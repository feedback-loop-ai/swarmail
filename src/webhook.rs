//! Outbound webhooks: queued, retried, never blocking the SMTP path.
//!
//! Delivery speaks real HTTP through reqwest with rustls: `http://` targets
//! stay plain TCP, `https://` targets are TLS with certificate verification
//! on — the rustls webpki root store, plus any extra root a target brings in
//! `ca_pem` (the self-signed test-server case). No native-tls anywhere in
//! the tree. The wire header names are lowercased by hyper; HTTP/1.1 names
//! are case-insensitive, so the secret arrives under `X-Swarmail-Secret`
//! regardless of case.

use crate::store::Store;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::Duration;

/// One delivery attempt must not hang: the whole round trip — connect
/// (TLS handshake included), send and status — is bounded by this.
const POST_TIMEOUT: Duration = Duration::from_secs(5);

/// The client for targets that trust the built-in root store, built once.
static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Clients pinned to an extra root CA, keyed by that root's PEM. A `ca_pem`
/// target gets its own client so its root never leaks into other targets.
static CA_CLIENTS: OnceLock<Mutex<HashMap<String, reqwest::Client>>> = OnceLock::new();

fn ca_clients() -> &'static Mutex<HashMap<String, reqwest::Client>> {
    CA_CLIENTS.get_or_init(Mutex::default)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookTarget {
    /// Full URL, e.g. "http://127.0.0.1:9999/hooks/mail" or
    /// "https://hooks.internal/mail" (delivered over TLS via reqwest/rustls).
    pub url: String,
    /// Only fire for this inbox; None = every inbox.
    #[serde(default)]
    pub inbox: Option<String>,
    /// Sent verbatim as the X-Swarmail-Secret header.
    #[serde(default)]
    pub secret: Option<String>,
    /// Optional PEM of an extra root CA to trust for this target, on top of
    /// the built-in root store — how a self-signed test server is trusted
    /// while certificate verification stays on.
    #[serde(default)]
    pub ca_pem: Option<String>,
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
                    tokio::spawn(deliver(t.url, t.secret, t.ca_pem, body));
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

async fn deliver(url: String, secret: Option<String>, ca_pem: Option<String>, body: String) {
    for attempt in 0..4u32 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(100 * 4u64.pow(attempt - 1))).await;
        }
        match post_json(&url, &body, secret.as_deref(), ca_pem.as_deref()).await {
            Ok(status) if (200..300).contains(&status) => return,
            Ok(status) => tracing::debug!(%url, status, "webhook non-2xx"),
            Err(e) => tracing::debug!(%url, error = %e, "webhook delivery failed"),
        }
    }
    tracing::warn!(%url, "webhook dropped after retries");
}

/// Redirects are not part of the delivery contract. A webhook target is an
/// explicit endpoint chosen by configuration; if it answers 3xx, the
/// delivery is over — a non-2xx under the ordinary retry budget, not a new
/// destination. reqwest's default policy would forward the whole request to
/// wherever the target points, handing the `X-Swarmail-Secret` header (and
/// the body on 307/308) to that destination; so redirects are never
/// followed, and a redirecting target simply costs its retry attempts.
fn redirect_policy() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::none()
}

/// The client for targets that trust the built-in root store: rustls with
/// the webpki roots, certificate verification on, built once per process.
fn shared_client() -> reqwest::Client {
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .use_rustls_tls() // rustls only — no native-tls in the tree
                .redirect(redirect_policy())
                .build()
                .expect("the rustls webhook client must build")
        })
        .clone()
}

/// Pick the client for a target: the shared rustls client, or one pinned to
/// the target's own root CA, cached by PEM so retries reuse it.
fn client_for(ca_pem: Option<&str>) -> Result<reqwest::Client, String> {
    let Some(pem) = ca_pem else {
        return Ok(shared_client());
    };
    let mut cache = ca_clients().lock().unwrap();
    if let Some(client) = cache.get(pem) {
        return Ok(client.clone());
    }
    let root = reqwest::Certificate::from_pem(pem.as_bytes())
        .map_err(|e| format!("webhook ca_pem is not a valid PEM certificate: {e}"))?;
    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .redirect(redirect_policy()) // same no-redirect contract as the shared client
        .add_root_certificate(root)
        .build()
        .map_err(|e| format!("webhook ca_pem could not be loaded into the rustls client: {e}"))?;
    cache.insert(pem.to_string(), client.clone());
    Ok(client)
}

async fn post_json(
    url: &str,
    body: &str,
    secret: Option<&str>,
    ca_pem: Option<&str>,
) -> Result<u16, String> {
    let url = reqwest::Url::parse(url).map_err(|e| format!("invalid webhook URL: {e}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(format!("unsupported webhook URL scheme: {}", url.scheme()));
    }
    let client = client_for(ca_pem)?;
    let mut request = client
        .post(url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .timeout(POST_TIMEOUT)
        .body(body.to_string());
    if let Some(secret) = secret {
        request = request.header("X-Swarmail-Secret", secret);
    }
    let response = request.send().await.map_err(|e| e.to_string())?;
    Ok(response.status().as_u16())
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
            message_id: None,
            in_reply_to: None,
            references: vec![],
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
            message_id: None,
            in_reply_to: None,
            references: vec![],
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

#[cfg(test)]
mod delivery_tests {
    use super::*;

    /// A minimal but real self-signed CA, PEM-encoded, for client tests.
    fn ca_pem() -> String {
        let key = rcgen::KeyPair::generate().unwrap();
        let mut params = rcgen::CertificateParams::new(Vec::<String>::new()).unwrap();
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        params.key_usages = vec![rcgen::KeyUsagePurpose::KeyCertSign];
        params.self_signed(&key).unwrap().pem()
    }

    /// A URL that does not parse at all is a delivery error, not a panic.
    #[tokio::test]
    async fn unparseable_url_is_an_error() {
        let err = post_json("not a url", "{}", None, None).await.unwrap_err();
        assert!(err.contains("invalid webhook URL"), "{err}");
    }

    /// Schemes other than http/https are rejected before any connection.
    #[tokio::test]
    async fn unsupported_scheme_is_an_error() {
        let err = post_json("ftp://hooks.example/x", "{}", None, None)
            .await
            .unwrap_err();
        assert!(err.contains("unsupported webhook URL scheme: ftp"), "{err}");
    }

    /// A ca_pem that is not a PEM certificate fails before connecting.
    #[tokio::test]
    async fn bad_ca_pem_is_an_error() {
        let err = client_for(Some("-----BEGIN CERTIFICATE-----\nnope")).unwrap_err();
        assert!(err.contains("ca_pem"), "{err}");
    }

    /// The client cache: one client per distinct CA PEM, the shared client
    /// for targets without one.
    #[tokio::test]
    async fn clients_are_cached_per_ca() {
        let pem = ca_pem();
        client_for(None).unwrap();
        client_for(Some(&pem)).unwrap();
        client_for(Some(&pem)).unwrap(); // same PEM → cache hit, not a rebuild
        assert_eq!(ca_clients().lock().unwrap().len(), 1);
        client_for(None).unwrap();
    }
}
