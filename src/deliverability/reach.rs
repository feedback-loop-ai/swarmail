//! The opt-in reachability probe (0121 §6). It is never part of the cached
//! report and never runs inside `evaluate`: the evaluator stays free of
//! network, clock and resolver access (0085). A caller that wants an
//! observation asks for it explicitly, and the frozen result is composed into
//! the deterministic report afterwards by `links::compose`.
//!
//! Every hop resolves the host itself, refuses the target unless **every**
//! answer is permitted, and pins the connection to the first one, so a name
//! cannot change its address between the check and the connect. Only status
//! and headers are read; the body never is.

use super::AnalysisError;
use futures_util::stream::{self, StreamExt};
use reqwest::{Url, header::LOCATION, redirect::Policy};
use std::collections::{BTreeMap, BTreeSet};
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::sync::Semaphore;

/// Distinct targets one request may probe; more is an explicit failure.
pub(crate) const MAX_TARGETS: usize = 32;
/// Targets probed at once by one request.
pub(crate) const LINKS_AT_ONCE: usize = 4;
/// Probe requests running at once per process.
pub(crate) const REQUESTS: usize = 2;
/// Redirects followed before `redirect_cap`.
pub(crate) const MAX_REDIRECTS: u8 = 5;
/// Budget for all hops of one target.
pub(crate) const BUDGET: Duration = Duration::from_secs(5);
/// Connect timeout of each hop.
pub(crate) const CONNECT: Duration = Duration::from_secs(2);
const USER_AGENT: &str = "swarmail-link-check";

/// What the server may contact (`--link-reachability`). `Off` is the default
/// and probes nothing; `Public` only globally routable unicast addresses;
/// `Loopback` also `127.0.0.0/8` and `::1`, for local development servers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum Mode {
    #[default]
    Off,
    Public,
    Loopback,
}

/// Ranges no mode contacts, as (network, prefix length). IPv4-mapped IPv6 is
/// unwrapped before either table is consulted.
const BLOCKED_V4: [(u32, u32); 12] = [
    (0x0000_0000, 8),  // 0.0.0.0/8
    (0x0a00_0000, 8),  // 10/8
    (0x6440_0000, 10), // 100.64/10 (CGNAT)
    (0xa9fe_0000, 16), // 169.254/16 (link-local, cloud metadata)
    (0xac10_0000, 12), // 172.16/12
    (0xc000_0000, 24), // 192.0.0/24
    (0xc000_0200, 24), // 192.0.2/24
    (0xc0a8_0000, 16), // 192.168/16
    (0xc612_0000, 15), // 198.18/15
    (0xc633_6400, 24), // 198.51.100/24
    (0xcb00_7100, 24), // 203.0.113/24
    (0xe000_0000, 3),  // 224/4 multicast and 240/4 reserved, with broadcast
];
const BLOCKED_V6: [(u128, u32); 6] = [
    (0, 128),                               // ::
    (0xfc00 << 112, 7),                     // fc00::/7
    (0xfe80 << 112, 10),                    // fe80::/10
    (0xff00 << 112, 8),                     // ff00::/8
    ((0x2001 << 112) | (0x0db8 << 96), 32), // 2001:db8::/32
    ((0x0064 << 112) | (0xff9b << 96), 96), // 64:ff9b::/96 NAT64 (stricter than the list)
];

/// Whether `mode` lets the probe contact `ip`.
pub(crate) fn permitted(ip: IpAddr, mode: Mode) -> bool {
    let ip = match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(ip, IpAddr::V4),
        v4 => v4,
    };
    if ip.is_loopback() {
        return mode == Mode::Loopback;
    }
    let blocked = match ip {
        IpAddr::V4(v4) => BLOCKED_V4
            .iter()
            .any(|&(net, bits)| (u32::from(v4) ^ net) >> (32 - bits) == 0),
        IpAddr::V6(v6) => BLOCKED_V6
            .iter()
            .any(|&(net, bits)| (u128::from(v6) ^ net) >> (128 - bits) == 0),
    };
    mode != Mode::Off && !blocked
}

/// How a target's probe ended. It carries no duration and no clock reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum End {
    Ok,
    Status(u16),
    Timeout,
    Connect,
    RedirectCap,
    BadRedirect,
    Refused,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Outcome {
    pub(crate) redirects: u8,
    /// Scheme, host and port of the last URL asked; never a path, query or
    /// userinfo.
    pub(crate) final_origin: String,
    pub(crate) end: End,
}

/// The frozen observation: one outcome per distinct target, order-free.
pub(crate) type Observation = BTreeMap<String, Outcome>;

#[cfg(test)]
type Resolver = std::sync::Arc<dyn Fn(&str) -> std::io::Result<Vec<IpAddr>> + Send + Sync>;

pub(crate) struct Prober {
    mode: Mode,
    requests: Semaphore,
    budget: Duration,
    #[cfg(test)]
    resolver: Option<Resolver>,
}

impl Default for Prober {
    fn default() -> Self {
        Self {
            mode: Mode::Off,
            requests: Semaphore::new(REQUESTS),
            budget: BUDGET,
            #[cfg(test)]
            resolver: None,
        }
    }
}

/// One hop's verdict: the target is final, or the next URL to ask.
enum Hop {
    Done(End),
    Next(Url),
}

impl Prober {
    pub(crate) fn with_mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    pub(crate) fn mode(&self) -> Mode {
        self.mode
    }

    #[cfg(test)]
    pub(crate) fn with_budget(mut self, budget: Duration) -> Self {
        self.budget = budget;
        self
    }

    #[cfg(test)]
    pub(crate) fn with_resolver(mut self, resolver: Resolver) -> Self {
        self.resolver = Some(resolver);
        self
    }

    /// Probe every target, four at a time. More than [`MAX_TARGETS`] fails
    /// before any connection, a mode of `Off` is unavailable before any
    /// lookup, and a third concurrent request is unavailable. Dropping the
    /// returned future cancels the probes and releases the request permit.
    pub(crate) async fn observe(
        &self,
        targets: BTreeSet<String>,
    ) -> Result<Observation, AnalysisError> {
        if targets.len() > MAX_TARGETS {
            return Err(AnalysisError::Failed);
        }
        if self.mode == Mode::Off {
            return Err(AnalysisError::Unavailable);
        }
        let _permit = self
            .requests
            .try_acquire()
            .map_err(|_| AnalysisError::Unavailable)?;
        Ok(stream::iter(targets)
            .map(|target| async move {
                // Targets are `Url::as_str` of parsed URLs, so they parse again.
                let url = Url::parse(&target).expect("a target is a parsed URL");
                (target, self.probe(url).await)
            })
            .buffer_unordered(LINKS_AT_ONCE)
            .collect()
            .await)
    }

    /// Follow one target through its redirects under the per-link budget.
    async fn probe(&self, start: Url) -> Outcome {
        let origin = start.origin().ascii_serialization();
        tokio::time::timeout(self.budget, self.follow(start))
            .await
            .unwrap_or(Outcome {
                redirects: 0,
                final_origin: origin,
                end: End::Timeout,
            })
    }

    async fn follow(&self, mut url: Url) -> Outcome {
        let mut redirects = 0;
        loop {
            let end = match self.hop(&url).await {
                Hop::Next(_) if redirects == MAX_REDIRECTS => End::RedirectCap,
                Hop::Next(next) => {
                    redirects += 1;
                    url = next;
                    continue;
                }
                Hop::Done(end) => end,
            };
            return Outcome {
                redirects,
                final_origin: url.origin().ascii_serialization(),
                end,
            };
        }
    }

    async fn resolve(&self, host: &str) -> Vec<IpAddr> {
        let bare = host.trim_matches(['[', ']']);
        if let Ok(ip) = bare.parse::<IpAddr>() {
            return vec![ip];
        }
        #[cfg(test)]
        if let Some(resolver) = &self.resolver {
            return resolver(host).unwrap_or_default();
        }
        tokio::net::lookup_host((host, 0))
            .await
            .map(|found| found.map(|addr| addr.ip()).collect())
            .unwrap_or_default()
    }

    async fn hop(&self, url: &Url) -> Hop {
        if !url.username().is_empty() || url.password().is_some() {
            return Hop::Done(End::Refused);
        }
        // Only http(s) URLs with a host reach here: targets are filtered by
        // scheme, and a redirect target is checked below.
        let host = url.host_str().unwrap_or_default();
        let addresses = self.resolve(host).await;
        let Some(&first) = addresses.first() else {
            return Hop::Done(End::Connect);
        };
        if !addresses.iter().all(|&ip| permitted(ip, self.mode)) {
            return Hop::Done(End::Refused);
        }
        let port = url.port_or_known_default().unwrap_or_default();
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .no_proxy()
            .redirect(Policy::none())
            .connect_timeout(CONNECT)
            .user_agent(USER_AGENT)
            .resolve_to_addrs(host, &[SocketAddr::new(first, port)])
            .build()
            .expect("the rustls link-check client must build");
        let Ok(response) = client.get(url.clone()).send().await else {
            return Hop::Done(End::Connect);
        };
        let status = response.status();
        if matches!(status.as_u16(), 301 | 302 | 303 | 307 | 308) {
            return match response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| url.join(value).ok())
                .filter(|next| matches!(next.scheme(), "http" | "https"))
            {
                Some(next) => Hop::Next(next),
                None => Hop::Done(End::BadRedirect),
            };
        }
        // The response drops here with its body unread.
        Hop::Done(if status.as_u16() >= 400 {
            End::Status(status.as_u16())
        } else {
            End::Ok
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };
    use std::time::Instant;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::task::{JoinHandle, JoinSet};

    /// A local HTTP server on 127.0.0.1 that counts accepted connections and
    /// answers each from `handler(index, request_head)`. Nothing here, and
    /// nothing in this module, names a public host.
    pub(super) struct Server {
        pub(super) addr: SocketAddr,
        accepts: Arc<AtomicUsize>,
        pub(super) accepted_at: Arc<Mutex<Vec<Instant>>>,
        pub(super) heads: Arc<Mutex<Vec<String>>>,
        task: JoinHandle<()>,
    }
    pub(super) struct Reply {
        pub(super) bytes: Vec<u8>,
        pub(super) hold: bool,
    }
    pub(super) fn reply(status: &str, headers: &str) -> Reply {
        Reply {
            bytes: format!(
                "HTTP/1.1 {status}\r\n{headers}Content-Length: 0\r\nConnection: close\r\n\r\n"
            )
            .into_bytes(),
            hold: false,
        }
    }
    /// A named handler, so tests whose server must never be reached still use
    /// a function that other tests execute.
    pub(super) fn ok(_: usize, _: &str) -> Reply {
        reply("200 OK", "")
    }
    pub(super) fn hold() -> Reply {
        Reply {
            bytes: Vec::new(),
            hold: true,
        }
    }
    impl Drop for Server {
        fn drop(&mut self) {
            self.task.abort();
        }
    }
    impl Server {
        pub(super) async fn start(
            handler: impl Fn(usize, &str) -> Reply + Send + Sync + 'static,
        ) -> Server {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let accepts = Arc::new(AtomicUsize::new(0));
            let accepted_at = Arc::new(Mutex::new(Vec::new()));
            let heads = Arc::new(Mutex::new(Vec::new()));
            let handler = Arc::new(handler);
            let (count, times, log) = (accepts.clone(), accepted_at.clone(), heads.clone());
            let task = tokio::spawn(async move {
                let mut connections = JoinSet::new();
                loop {
                    let (mut stream, _) = listener.accept().await.unwrap();
                    let index = count.fetch_add(1, Ordering::SeqCst);
                    times.lock().unwrap().push(Instant::now());
                    let (handler, log) = (handler.clone(), log.clone());
                    connections.spawn(async move {
                        let mut head = Vec::new();
                        let mut chunk = [0u8; 1024];
                        while !head.windows(4).any(|w| w == b"\r\n\r\n") {
                            match stream.read(&mut chunk).await {
                                Ok(0) | Err(_) => return,
                                Ok(n) => head.extend_from_slice(&chunk[..n]),
                            }
                        }
                        let head = String::from_utf8_lossy(&head).into_owned();
                        log.lock().unwrap().push(head.clone());
                        let answer = handler(index, &head);
                        let _ = stream.write_all(&answer.bytes).await;
                        // A held connection stays open until the client closes it.
                        if answer.hold {
                            let _ = stream.read_to_end(&mut Vec::new()).await;
                        }
                    });
                }
            });
            Server {
                addr,
                accepts,
                accepted_at,
                heads,
                task,
            }
        }
        pub(super) fn url(&self, path: &str) -> String {
            format!("http://{}{path}", self.addr)
        }
        pub(super) fn origin(&self) -> String {
            format!("http://{}", self.addr)
        }
        pub(super) fn accepted(&self) -> usize {
            self.accepts.load(Ordering::SeqCst)
        }
        /// Wait, to a real deadline, until `n` connections were accepted.
        pub(super) async fn accepted_at_least(&self, n: usize) {
            let deadline = Instant::now() + Duration::from_secs(20);
            while self.accepted() < n {
                assert!(
                    Instant::now() < deadline,
                    "only {} accepts",
                    self.accepted()
                );
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        }
    }

    fn set(urls: &[String]) -> BTreeSet<String> {
        urls.iter()
            .map(|u| Url::parse(u).unwrap().as_str().to_string())
            .collect()
    }
    async fn one(prober: &Prober, url: &str) -> Outcome {
        let key = Url::parse(url).unwrap().as_str().to_string();
        let mut observation = prober.observe(set(&[url.to_string()])).await.unwrap();
        observation.remove(&key).unwrap()
    }
    fn loopback() -> Prober {
        Prober::default().with_mode(Mode::Loopback)
    }

    #[test]
    fn address_policy_table() {
        let ip = |s: &str| s.parse::<IpAddr>().unwrap();
        let never = [
            ("0.0.0.0", "0.255.255.255"),
            ("10.0.0.0", "10.255.255.255"),
            ("100.64.0.0", "100.127.255.255"),
            ("169.254.0.0", "169.254.255.255"),
            ("172.16.0.0", "172.31.255.255"),
            ("192.0.0.0", "192.0.0.255"),
            ("192.0.2.0", "192.0.2.255"),
            ("192.168.0.0", "192.168.255.255"),
            ("198.18.0.0", "198.19.255.255"),
            ("198.51.100.0", "198.51.100.255"),
            ("203.0.113.0", "203.0.113.255"),
            ("224.0.0.0", "239.255.255.255"),
            ("240.0.0.0", "255.255.255.255"),
            ("::", "::"),
            ("fc00::", "fdff:ffff:ffff:ffff:ffff:ffff:ffff:ffff"),
            ("fe80::", "febf:ffff:ffff:ffff:ffff:ffff:ffff:ffff"),
            ("ff00::", "ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff"),
            ("2001:db8::", "2001:db8:ffff:ffff:ffff:ffff:ffff:ffff"),
            ("64:ff9b::", "64:ff9b::ffff:ffff"),
        ];
        for (first, last) in never {
            for edge in [first, last] {
                for mode in [Mode::Off, Mode::Public, Mode::Loopback] {
                    assert!(!permitted(ip(edge), mode), "{edge} {mode:?}");
                }
            }
        }
        // Mapped IPv6 is unwrapped before the table is read.
        for mapped in [
            "::ffff:10.0.0.1",
            "::ffff:169.254.169.254",
            "::ffff:192.168.0.1",
        ] {
            assert!(!permitted(ip(mapped), Mode::Loopback), "{mapped}");
        }
        // Just outside each range, and ordinary public addresses.
        for outside in [
            "1.0.0.0",
            "9.255.255.255",
            "11.0.0.0",
            "100.63.255.255",
            "100.128.0.0",
            "169.253.255.255",
            "169.255.0.0",
            "172.15.255.255",
            "172.32.0.0",
            "192.0.1.0",
            "192.0.3.0",
            "192.167.255.255",
            "192.169.0.0",
            "198.17.255.255",
            "198.20.0.0",
            "198.51.99.255",
            "198.51.101.0",
            "203.0.112.255",
            "203.0.114.0",
            "223.255.255.255",
            "8.8.8.8",
            "::ffff:8.8.8.8",
            "2606:4700:4700::1111",
            "2001:db9::1",
            "2001:db7:ffff::1",
            "fbff::1",
            "fec0::1",
            "64:ff9a::1",
            "64:ff9c::1",
        ] {
            assert!(!permitted(ip(outside), Mode::Off), "{outside} off");
            assert!(permitted(ip(outside), Mode::Public), "{outside} public");
            assert!(permitted(ip(outside), Mode::Loopback), "{outside} loopback");
        }
        // Loopback is the one range a mode changes.
        for local in ["127.0.0.1", "127.255.255.255", "::1", "::ffff:127.0.0.1"] {
            assert!(!permitted(ip(local), Mode::Off));
            assert!(!permitted(ip(local), Mode::Public), "{local}");
            assert!(permitted(ip(local), Mode::Loopback), "{local}");
        }
    }

    #[tokio::test]
    async fn probe_ok_and_status() {
        let server = Server::start(|_, head| match head.split(' ').nth(1).unwrap() {
            "/missing" => reply("404 Not Found", ""),
            "/broken" => reply("500 Internal Server Error", ""),
            "/not-modified" => reply("304 Not Modified", ""),
            _ => reply("200 OK", ""),
        })
        .await;
        let prober = loopback();
        for (path, end) in [
            ("/ok", End::Ok),
            ("/missing", End::Status(404)),
            ("/broken", End::Status(500)),
            ("/not-modified", End::Ok),
        ] {
            let outcome = one(&prober, &server.url(path)).await;
            let expected = Outcome {
                redirects: 0,
                final_origin: server.origin(),
                end,
            };
            assert_eq!(outcome, expected, "{path}");
        }
        let heads = server.heads.lock().unwrap().join("");
        assert!(heads.contains("GET /ok HTTP/1.1"), "{heads}");
        assert!(heads.contains("user-agent: swarmail-link-check"), "{heads}");
    }

    #[tokio::test]
    async fn probe_redirects_within_and_beyond_cap() {
        let server = Server::start(|_, head| match head.split(' ').nth(1).unwrap() {
            "/r0" => reply("301 Moved", "Location: /r1\r\n"),
            "/r1" => reply("307 Temporary Redirect", "Location: /final\r\n"),
            "/gone0" => reply("302 Found", "Location: /gone\r\n"),
            "/gone" => reply("410 Gone", ""),
            "/loop" => reply("302 Found", "Location: /loop\r\n"),
            _ => reply("200 OK", ""),
        })
        .await;
        let prober = loopback();
        let outcome = one(&prober, &server.url("/r0")).await;
        let expected = Outcome {
            redirects: 2,
            final_origin: server.origin(),
            end: End::Ok,
        };
        assert_eq!(outcome, expected);
        let outcome = one(&prober, &server.url("/gone0")).await;
        assert_eq!((outcome.redirects, outcome.end), (1, End::Status(410)));
        let before = server.accepted();
        let outcome = one(&prober, &server.url("/loop")).await;
        let expected = Outcome {
            redirects: MAX_REDIRECTS,
            final_origin: server.origin(),
            end: End::RedirectCap,
        };
        assert_eq!(outcome, expected);
        assert_eq!(
            server.accepted() - before,
            6,
            "the first ask and five redirects"
        );
    }

    #[tokio::test]
    async fn probe_bad_redirects() {
        let server = Server::start(|_, head| match head.split(' ').nth(1).unwrap() {
            "/none" => reply("302 Found", ""),
            "/ftp" => reply("302 Found", "Location: ftp://files.example/x\r\n"),
            "/broken" => reply("302 Found", "Location: http://[::1\r\n"),
            "/opaque" => reply("303 See Other", "Location: javascript:alert(1)\r\n"),
            _ => Reply {
                bytes: b"HTTP/1.1 308 Moved\r\nLocation: /caf\xe9\r\nContent-Length: 0\r\n\r\n"
                    .to_vec(),
                hold: false,
            },
        })
        .await;
        let prober = loopback();
        for path in ["/none", "/ftp", "/broken", "/opaque", "/latin1"] {
            let outcome = one(&prober, &server.url(path)).await;
            assert_eq!(
                (outcome.end, outcome.redirects),
                (End::BadRedirect, 0),
                "{path}"
            );
        }
    }

    #[tokio::test]
    async fn probe_timeout_on_silent_server() {
        let server = Server::start(|_, _| hold()).await;
        let prober = loopback().with_budget(Duration::from_millis(300));
        let outcome = one(&prober, &server.url("/silent")).await;
        let expected = Outcome {
            redirects: 0,
            final_origin: server.origin(),
            end: End::Timeout,
        };
        assert_eq!(outcome, expected);
        assert_eq!(server.accepted(), 1);
    }

    #[tokio::test]
    async fn probe_connect_on_closed_port() {
        let closed = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", closed.local_addr().unwrap());
        drop(closed);
        assert_eq!(one(&loopback(), &url).await.end, End::Connect);
    }

    #[tokio::test]
    async fn the_test_server_ignores_a_client_that_hangs_up() {
        let server = Server::start(ok).await;
        drop(tokio::net::TcpStream::connect(server.addr).await.unwrap());
        server.accepted_at_least(1).await;
        assert_eq!(one(&loopback(), &server.url("/after")).await.end, End::Ok);
    }

    #[tokio::test]
    async fn probe_never_reads_body() {
        let server = Server::start(|_, _| Reply {
            bytes: b"HTTP/1.1 200 OK\r\nContent-Length: 104857600\r\n\r\n".to_vec(),
            hold: true,
        })
        .await;
        assert_eq!(one(&loopback(), &server.url("/huge")).await.end, End::Ok);
    }

    #[tokio::test]
    async fn probe_refuses_userinfo() {
        let server = Server::start(ok).await;
        for mode in [Mode::Public, Mode::Loopback] {
            for userinfo in ["user:pass@", "user@", ":pass@"] {
                let url = format!("http://{userinfo}{}/", server.addr);
                let outcome = one(&Prober::default().with_mode(mode), &url).await;
                assert_eq!(outcome.end, End::Refused, "{url} {mode:?}");
            }
        }
        assert_eq!(server.accepted(), 0);
    }

    #[tokio::test]
    async fn probe_public_mode_refuses_loopback_literals_and_names() {
        let server = Server::start(ok).await;
        let public = Prober::default().with_mode(Mode::Public);
        assert_eq!(one(&public, &server.url("/")).await.end, End::Refused);
        // `localhost` goes through the real resolver and still lands on loopback.
        let by_name = format!("http://localhost:{}/", server.addr.port());
        assert_eq!(one(&public, &by_name).await.end, End::Refused);
        let ipv6 = format!("http://[::1]:{}/", server.addr.port());
        assert_eq!(one(&public, &ipv6).await.end, End::Refused);
        assert_eq!(server.accepted(), 0);
    }

    type Table = &'static [(&'static str, &'static [&'static str])];
    fn resolver(table: Table, asked: Arc<Mutex<Vec<String>>>) -> Resolver {
        Arc::new(move |host| {
            asked.lock().unwrap().push(host.to_string());
            match table.iter().find(|(name, _)| *name == host) {
                Some((_, answers)) => Ok(answers.iter().map(|a| a.parse().unwrap()).collect()),
                None => Err(std::io::Error::other("no such host")),
            }
        })
    }

    #[tokio::test]
    async fn probe_resolver_seam_pins_mixes_and_fails() {
        static TABLE: Table = &[
            ("pinned.test", &["127.0.0.1"]),
            ("mixed.test", &["127.0.0.1", "10.0.0.1"]),
            ("mixed-first.test", &["10.0.0.1", "127.0.0.1"]),
            ("empty.test", &[]),
        ];
        let server = Server::start(ok).await;
        let asked = Arc::new(Mutex::new(Vec::new()));
        let prober = loopback().with_resolver(resolver(TABLE, asked.clone()));
        let port = server.addr.port();
        // The name is connected through the validated address and sent as Host.
        let outcome = one(&prober, &format!("http://pinned.test:{port}/p")).await;
        assert_eq!(outcome.end, End::Ok);
        assert_eq!(server.accepted(), 1);
        let head = server.heads.lock().unwrap()[0].clone();
        assert!(
            head.contains(&format!("host: pinned.test:{port}\r\n")),
            "{head}"
        );
        // One forbidden answer refuses the target, whatever its position.
        for name in ["mixed.test", "mixed-first.test"] {
            let outcome = one(&prober, &format!("http://{name}:{port}/")).await;
            assert_eq!(outcome.end, End::Refused, "{name}");
        }
        // A resolver error and an empty answer are connect failures.
        for name in ["empty.test", "unknown.test"] {
            let outcome = one(&prober, &format!("http://{name}:{port}/")).await;
            assert_eq!(outcome.end, End::Connect, "{name}");
        }
        assert_eq!(server.accepted(), 1, "only the pinned name connected");
        assert_eq!(asked.lock().unwrap().len(), 5);
    }

    #[tokio::test]
    async fn probe_revalidates_each_hop() {
        let private = "Location: http://10.0.0.1/inner\r\n";
        let server = Server::start(move |_, _| reply("302 Found", private)).await;
        let outcome = one(&loopback(), &server.url("/start")).await;
        let expected = Outcome {
            redirects: 1,
            final_origin: "http://10.0.0.1".into(),
            end: End::Refused,
        };
        assert_eq!(outcome, expected);
        assert_eq!(server.accepted(), 1, "the private hop was never dialled");
        // A redirect carrying credentials is refused at the next hop too.
        let creds = "Location: http://u:p@127.0.0.1:9/\r\n";
        let server = Server::start(move |_, _| reply("302 Found", creds)).await;
        assert_eq!(one(&loopback(), &server.url("/")).await.end, End::Refused);
    }

    #[tokio::test]
    async fn observe_caps_targets_before_connecting() {
        let server = Server::start(ok).await;
        let urls: Vec<_> = (0..=MAX_TARGETS)
            .map(|n| server.url(&format!("/{n}")))
            .collect();
        let refused = loopback().observe(set(&urls)).await;
        assert_eq!(refused.unwrap_err(), AnalysisError::Failed);
        assert_eq!(server.accepted(), 0);
        let observation = loopback().observe(set(&urls[..MAX_TARGETS])).await.unwrap();
        assert_eq!(observation.len(), MAX_TARGETS);
        assert_eq!(server.accepted(), MAX_TARGETS);
    }

    #[tokio::test]
    async fn observe_is_unavailable_when_off_and_never_resolves() {
        static NONE: Table = &[];
        let asked = Arc::new(Mutex::new(Vec::new()));
        let off = Prober::default().with_resolver(resolver(NONE, asked.clone()));
        assert_eq!(off.mode(), Mode::Off);
        let targets = set(&["http://named.test/".to_string()]);
        let refused = off.observe(targets).await;
        assert_eq!(refused.unwrap_err(), AnalysisError::Unavailable);
        assert!(asked.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn observe_runs_four_at_a_time() {
        let server = Server::start(|_, _| hold()).await;
        let budget = Duration::from_secs(2);
        let prober = loopback().with_budget(budget);
        let urls: Vec<_> = (0..=LINKS_AT_ONCE)
            .map(|n| server.url(&format!("/{n}")))
            .collect();
        let observation = prober.observe(set(&urls)).await.unwrap();
        assert_eq!(observation.len(), 5);
        assert!(observation.values().all(|o| o.end == End::Timeout));
        assert_eq!(server.accepted(), 5);
        let at = server.accepted_at.lock().unwrap().clone();
        // Four connections were open together; the fifth waited for a slot,
        // which only a timeout could free.
        assert!(at[3] - at[0] < budget / 2, "{:?}", at[3] - at[0]);
        assert!(at[4] - at[0] >= budget / 2, "{:?}", at[4] - at[0]);
    }

    #[tokio::test]
    async fn observe_refuses_a_third_request_and_cancellation_releases_a_permit() {
        let silent = Server::start(|_, _| hold()).await;
        let prober = Arc::new(loopback().with_budget(Duration::from_secs(30)));
        let start = |n: usize| {
            let (prober, url) = (prober.clone(), silent.url(&format!("/held{n}")));
            tokio::spawn(async move { prober.observe(set(&[url])).await })
        };
        let (first, second) = (start(1), start(2));
        silent.accepted_at_least(REQUESTS).await;
        let third = prober.observe(set(&[silent.url("/third")])).await;
        assert_eq!(third.unwrap_err(), AnalysisError::Unavailable);
        // Dropping a held request cancels its probe and releases the permit.
        first.abort();
        assert!(first.await.unwrap_err().is_cancelled());
        let quick = Server::start(ok).await;
        let admitted = prober.observe(set(&[quick.url("/")])).await.unwrap();
        assert_eq!(admitted.len(), 1);
        second.abort();
    }

    use super::super::{JOBS, Location, Service, Severity};
    use crate::model::Email;

    fn link_mail(id: &str, html: &str) -> Arc<Email> {
        let mut mail = crate::store::email(id, "box", "a");
        mail.raw = format!("Content-Type: text/html; charset=utf-8\r\n\r\n{html}").into_bytes();
        Arc::new(mail)
    }
    fn probing(prober: Prober) -> Service {
        Service {
            prober,
            ..Service::default()
        }
    }

    #[tokio::test]
    async fn probed_reuses_slot_and_never_caches() {
        let server = Server::start(|_, head| match head.split(' ').nth(1).unwrap() {
            "/gone" => reply("404 Not Found", ""),
            _ => reply("200 OK", ""),
        })
        .await;
        let html = format!(
            "<a href='{}'>fine</a> <a href='{}'>gone</a>",
            server.url("/ok"),
            server.url("/gone")
        );
        let mail = link_mail("probed", &html);
        let service = probing(loopback());
        assert_eq!(service.link_reachability(), Mode::Loopback);
        let first = service.probed(mail.clone()).await.unwrap();
        assert_eq!(server.accepted(), 2);
        let found: Vec<_> = first
            .findings
            .iter()
            .filter(|f| f.rule_id.starts_with("link."))
            .collect();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].rule_id, "link.reachability.v1");
        assert_eq!(found[0].severity, Severity::Warning);
        assert!(found[0].fix_hint.contains("HTTP 404"));
        assert_eq!(
            found[0].location,
            Location::Link {
                part_path: vec![],
                byte_offset: html.find(&server.url("/gone")).unwrap() as u64
            }
        );
        // The deterministic slot is a ready-only hit and carries no observation.
        let cached = service.report(mail.clone(), true).await.unwrap();
        assert!(
            cached
                .findings
                .iter()
                .all(|f| !f.rule_id.starts_with("link."))
        );
        assert_eq!(cached.findings.len() + 1, first.findings.len());
        assert_eq!(service.slots.lock().unwrap().len(), 1);
        // A second probed request probes again: nothing was cached.
        let second = service.probed(mail.clone()).await.unwrap();
        assert_eq!(server.accepted(), 4);
        assert_eq!(second, first);
        assert_eq!(service.slots.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn probed_refusals_and_failures() {
        let server = Server::start(ok).await;
        let mail = link_mail("refusals", &format!("<a href='{}'>x</a>", server.url("/")));
        // `off` is unavailable and opens nothing.
        let off = Service::default();
        assert_eq!(
            off.probed(mail.clone()).await.unwrap_err(),
            AnalysisError::Unavailable
        );
        // More than 32 distinct targets fail before any connection.
        let many = (0..=MAX_TARGETS)
            .map(|n| format!("<a href='{}'>x</a>", server.url(&format!("/{n}"))))
            .collect::<String>();
        let service = probing(loopback());
        let refused = service.probed(link_mail("many", &many)).await;
        assert_eq!(refused.unwrap_err(), AnalysisError::Failed);
        assert_eq!(server.accepted(), 0);
        // With every job permit taken the lexical scans are unavailable.
        service.report(mail.clone(), false).await.unwrap();
        let held = service
            .jobs
            .clone()
            .acquire_many_owned(JOBS as u32)
            .await
            .unwrap();
        assert_eq!(
            service.probed(mail.clone()).await.unwrap_err(),
            AnalysisError::Unavailable
        );
        assert_eq!(server.accepted(), 0);
        drop(held);
        // A cold report that fails stays a failure of the probed request.
        let broken = link_mail(
            "broken",
            &format!("<a href='{}'>x</a><div", server.url("/")),
        );
        assert_eq!(
            service.probed(broken).await.unwrap_err(),
            AnalysisError::Failed
        );
        // Observation findings beyond the limits fail whole, never truncated.
        let gone = Server::start(|_, _| reply("404 Not Found", "")).await;
        let links = |n: usize| {
            (0..n)
                .map(|_| format!("<a href='{}'>x</a>", gone.url("/x")))
                .collect::<String>()
        };
        for n in [
            super::super::MAX_FINDINGS + 1,
            super::super::MAX_FINDINGS - 3,
        ] {
            let refused = service.probed(link_mail("flood", &links(n))).await;
            assert_eq!(refused.unwrap_err(), AnalysisError::Failed, "{n}");
        }
    }

    #[tokio::test]
    async fn probes_hold_no_job_permit_and_no_store_guard() {
        let silent = Server::start(|_, _| hold()).await;
        let service = Arc::new(probing(loopback()));
        let held = |n: usize| {
            let mail = link_mail(
                &format!("held{n}"),
                &format!("<a href='{}'>x</a>", silent.url(&format!("/{n}"))),
            );
            let service = service.clone();
            tokio::spawn(async move { service.probed(mail).await })
        };
        let (first, second) = (held(1), held(2));
        silent.accepted_at_least(REQUESTS).await;
        assert_eq!(service.jobs.available_permits(), JOBS);
        // A cold report for another mail completes while both probes wait.
        let other = link_mail("other", "<p>plain</p>");
        service.report(other, false).await.unwrap();
        // The third request is refused while two are held.
        let third = service
            .probed(link_mail(
                "third",
                &format!("<a href='{}'>x</a>", silent.url("/3")),
            ))
            .await;
        assert_eq!(third.unwrap_err(), AnalysisError::Unavailable);
        first.abort();
        second.abort();
    }

    /// Tests never touch the internet: every URL literal in the probe tests
    /// names loopback, a placeholder bound to a local listener, or a name that
    /// only the resolver seam answers (T7.3).
    #[test]
    fn probe_tests_use_loopback_only() {
        // The empty name is a `{}` placeholder for a local listener's address.
        const ALLOWED: [&str; 11] = [
            "",
            "127.0.0.1",
            "[::1]",
            "localhost",
            "10.0.0.1",
            "pinned.test",
            "mixed.test",
            "mixed-first.test",
            "empty.test",
            "unknown.test",
            "named.test",
        ];
        let mut seen = 0;
        for (file, source) in [
            ("reach.rs", include_str!("reach.rs")),
            (
                "tests/link_reachability.rs",
                include_str!("../../tests/link_reachability.rs"),
            ),
        ] {
            for scheme in [["http", "://"].concat(), ["https", "://"].concat()] {
                for (at, _) in source.match_indices(&scheme) {
                    let rest = &source[at + scheme.len()..];
                    let end = rest.find(['/', '"', '\'', ' ', '\\', '?', '#', '\n']);
                    let authority = &rest[..end.unwrap_or(rest.len())];
                    let host = authority.rsplit('@').next().unwrap();
                    let host = match host.strip_prefix('[') {
                        Some(v6) => format!("[{}]", v6.split(']').next().unwrap()),
                        None => host.split([':', '{']).next().unwrap().to_string(),
                    };
                    seen += 1;
                    assert!(
                        ALLOWED.contains(&host.as_str()),
                        "{file}: {authority} is not a loopback, placeholder or seam name"
                    );
                }
            }
        }
        assert!(seen > 10, "the guard scanned the probe tests: {seen}");
    }
}
