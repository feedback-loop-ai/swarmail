//! Process-owned, no-queue CA execution. Logical charges follow 0078.
use super::WebhookTarget;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, mpsc};

pub(super) const WORKERS: usize = 2;
pub(super) const BYTE_CAP: usize = 67_108_864;
pub(crate) const UNAVAILABLE: &str = "webhook CA preparation unavailable";

#[derive(Clone, Default)]
pub(crate) struct Scope(Arc<Mutex<bool>>);
impl Scope {
    pub(crate) fn live(&self) -> Option<MutexGuard<'_, bool>> {
        let guard = self.0.lock().unwrap();
        (!*guard).then_some(guard)
    }
    fn admit(&self) -> Option<MutexGuard<'_, bool>> {
        let guard = self.0.try_lock().ok()?;
        (!*guard).then_some(guard)
    }
}
pub(crate) struct Owner(pub(crate) Scope);
impl Owner {
    pub(crate) fn close(&self) {
        *self.0.0.lock().unwrap() = true;
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.close();
    }
}

/// Descriptor permits arithmetic boundary tests without allocating huge inputs.
#[derive(Default, Clone, Copy)]
pub(super) struct Size {
    pub n: usize,
    pub u: usize,
    pub i: usize,
    pub h: usize,
    pub p: usize,
    pub vi: usize,
    pub vo: usize,
    pub r: usize,
    pub a: usize,
}
impl Size {
    pub fn charge(self) -> Option<usize> {
        let Self {
            n,
            u,
            i,
            h,
            p,
            vi,
            vo,
            r,
            a,
        } = self;
        let s = u.checked_add(i)?.checked_add(h)?.checked_add(p)?;
        let e = (p / 3).checked_add(r)?.checked_mul(4)?;
        let c = 55usize
            .checked_mul(r)?
            .checked_add(e)?
            .checked_add(e / 64)?;
        let o = 8usize
            .checked_mul(u)?
            .checked_add(256usize.checked_mul(n)?)?
            .checked_add(i)?
            .checked_add(c)?;
        let terms = [
            65536,
            512usize.checked_mul(n)?,
            vi,
            vo,
            4usize.checked_mul(s)?,
            8usize.checked_mul(o)?,
            8usize.checked_mul(p.checked_add(c)?)?,
            512usize.checked_mul(r)?,
            65536usize.checked_mul(a)?,
        ];
        terms.into_iter().try_fold(0usize, usize::checked_add)
    }
    pub fn targets(targets: &[WebhookTarget], capacity: usize) -> Option<Self> {
        let backing = std::mem::size_of::<WebhookTarget>();
        let mut size = Self {
            n: targets.len(),
            vi: capacity.checked_mul(backing)?,
            vo: targets.len().checked_mul(backing)?,
            ..Self::default()
        };
        for target in targets {
            size.u = size.u.checked_add(target.url.capacity())?;
            size.i = size
                .i
                .checked_add(target.inbox.as_ref().map_or(0, String::capacity))?;
            size.h = size
                .h
                .checked_add(target.secret.as_ref().map_or(0, String::capacity))?;
            if let Some(pem) = &target.ca_pem {
                size.p = size.p.checked_add(pem.capacity())?;
                size.r = size.r.checked_add(pem.len() / 27)?;
                size.a = 1;
            }
        }
        Some(size)
    }
    pub fn delivery(pem: &str) -> Self {
        Self {
            n: 1,
            p: pem.len(),
            r: pem.len() / 27,
            a: 1,
            ..Self::default()
        }
    }
}

pub(crate) enum Job {
    Get(Vec<WebhookTarget>),
    Put(Vec<WebhookTarget>),
    Delivery(String),
}
pub(crate) enum Output {
    Get(Vec<WebhookTarget>),
    Put(Vec<WebhookTarget>, Vec<WebhookTarget>),
    Delivery(String, reqwest::Client),
}
impl Job {
    fn run(self) -> Result<Output, super::TargetPreparationError> {
        use super::{TargetPreparationError, WebhookTarget};
        match self {
            Self::Get(targets) => {
                #[cfg(test)]
                super::panic_api_preparation_for_test(&targets);
                Ok(Output::Get(
                    targets.iter().map(WebhookTarget::redacted).collect(),
                ))
            }
            Self::Put(targets) => {
                #[cfg(test)]
                super::panic_api_preparation_for_test(&targets);
                for (index, target) in targets.iter().enumerate() {
                    target.validate().map_err(|error| {
                        TargetPreparationError::Invalid(format!("webhook target {index}: {error}"))
                    })?;
                }
                let echo = targets.iter().map(WebhookTarget::redacted).collect();
                Ok(Output::Put(targets, echo))
            }
            Self::Delivery(pem) => {
                #[cfg(test)]
                super::panic_ca_build_for_test(&pem);
                let client =
                    super::build_ca_client(&pem).map_err(TargetPreparationError::Invalid)?;
                super::CA_CACHE_BUILDS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Ok(Output::Delivery(pem, client))
            }
        }
    }
}

#[derive(Default)]
struct State {
    busy: [bool; WORKERS],
    bytes: usize,
}
struct Envelope {
    job: Job,
    scope: Scope,
    reply: tokio::sync::oneshot::Sender<Charged>,
    reservation: Reservation,
}
/// Fields drop in order: owned data is disposed before refunding the slot.
pub(crate) struct Charged {
    pub value: Result<Output, super::TargetPreparationError>,
    pub reservation: Reservation,
}
pub(crate) struct Reservation {
    executor: Arc<Executor>,
    slot: usize,
    charge: usize,
}
impl Drop for Reservation {
    fn drop(&mut self) {
        let mut state = self.executor.state.lock().unwrap();
        state.busy[self.slot] = false;
        state.bytes -= self.charge;
    }
}
pub(super) struct Executor {
    state: Mutex<State>,
    workers: Vec<mpsc::SyncSender<Envelope>>,
}
fn worker(rx: mpsc::Receiver<Envelope>) {
    while let Ok(envelope) = rx.recv() {
        execute_envelope(envelope);
    }
}
fn execute_envelope(envelope: Envelope) {
    let Envelope {
        job,
        scope,
        reply,
        reservation,
    } = envelope;
    if reply.is_closed() {
        drop(job);
        drop(reservation);
        return;
    }
    let value = if scope.live().is_none() {
        drop(job);
        Err(super::TargetPreparationError::Unavailable)
    } else {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| job.run()))
            .unwrap_or(Err(super::TargetPreparationError::Worker))
    };
    let _ = reply.send(Charged { value, reservation });
}

fn spawn_worker(rx: mpsc::Receiver<Envelope>) -> std::io::Result<std::thread::JoinHandle<()>> {
    std::thread::Builder::new()
        .name("swarmail-ca".into())
        .spawn(move || worker(rx))
}
impl Executor {
    fn start(
        spawn: fn(mpsc::Receiver<Envelope>) -> std::io::Result<std::thread::JoinHandle<()>>,
    ) -> std::io::Result<Arc<Self>> {
        let mut workers = Vec::new();
        let mut handles = Vec::new();
        for _ in 0..WORKERS {
            let (tx, rx) = mpsc::sync_channel(1);
            match spawn(rx) {
                Ok(handle) => {
                    workers.push(tx);
                    handles.push(handle);
                }
                Err(error) => {
                    drop(workers);
                    for handle in handles {
                        handle.join().expect("idle CA worker exits");
                    }
                    return Err(error);
                }
            }
        }
        // Successful workers are process-owned, independent of Tokio lifetimes.
        Ok(Arc::new(Self {
            state: Mutex::default(),
            workers,
        }))
    }
    pub fn reserve(
        self: &Arc<Self>,
        scope: &Scope,
        charge: Option<usize>,
    ) -> Result<Reservation, super::TargetPreparationError> {
        use super::TargetPreparationError::Unavailable;
        let _live = scope.admit().ok_or(Unavailable)?;
        let charge = charge.ok_or(Unavailable)?;
        let mut state = self.state.try_lock().map_err(|_| Unavailable)?;
        let total = state
            .bytes
            .checked_add(charge)
            .filter(|n| *n <= BYTE_CAP)
            .ok_or(Unavailable)?;
        let slot = state
            .busy
            .iter()
            .position(|busy| !busy)
            .ok_or(Unavailable)?;
        state.busy[slot] = true;
        state.bytes = total;
        Ok(Reservation {
            executor: self.clone(),
            slot,
            charge,
        })
    }
}
impl Reservation {
    pub async fn run(
        self,
        scope: &Scope,
        job: Job,
    ) -> Result<Charged, super::TargetPreparationError> {
        let (reply, receive) = tokio::sync::oneshot::channel();
        let executor = self.executor.clone();
        let slot = self.slot;
        executor.workers[slot]
            .try_send(Envelope {
                job,
                scope: scope.clone(),
                reply,
                reservation: self,
            })
            .map_err(|_| super::TargetPreparationError::Worker)?;
        receive
            .await
            .map_err(|_| super::TargetPreparationError::Worker)
    }
}
static EXECUTOR: OnceLock<Option<Arc<Executor>>> = OnceLock::new();
pub(crate) fn initialize() {
    let _ = EXECUTOR.get_or_init(|| Executor::start(spawn_worker).ok());
}
pub(super) fn reserve(
    scope: &Scope,
    size: Option<Size>,
) -> Result<Reservation, super::TargetPreparationError> {
    initialize();
    EXECUTOR
        .get()
        .unwrap()
        .as_ref()
        .ok_or(super::TargetPreparationError::Unavailable)?
        .reserve(scope, size.and_then(Size::charge))
}

#[cfg(test)]
mod executor_tests;

#[cfg(test)]
pub(super) fn jobs_for_test() -> usize {
    let executor = EXECUTOR.get().unwrap().as_ref().unwrap();
    executor
        .state
        .lock()
        .unwrap()
        .busy
        .iter()
        .filter(|b| **b)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ca_client_builds_without_tokio_context() {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()])
            .unwrap()
            .cert
            .pem();
        std::thread::spawn(move || {
            assert!(crate::webhook::build_ca_client(&cert).is_ok());
        })
        .join()
        .unwrap();
    }

    #[test]
    fn ca_executor_initialization_failure_is_sticky() {
        static CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        fn fail_second(
            rx: mpsc::Receiver<Envelope>,
        ) -> std::io::Result<std::thread::JoinHandle<()>> {
            if CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                spawn_worker(rx)
            } else {
                Err(std::io::Error::other(
                    "injected worker initialization failure",
                ))
            }
        }
        let once = OnceLock::new();
        for _ in 0..2 {
            assert!(
                once.get_or_init(|| Executor::start(fail_second).ok())
                    .is_none()
            );
        }
        assert_eq!(CALLS.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[test]
    fn ca_scope_close_races_publication() {
        let scope = Scope::default();
        let owner = Owner(scope.clone());
        let guard = scope.live().unwrap();
        let (entered, observed) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            entered.send(()).unwrap();
            owner.close();
        });
        observed.recv().unwrap();
        assert!(scope.admit().is_none());
        drop(guard);
        thread.join().unwrap();
        assert!(scope.live().is_none());
        let scope = Scope::default();
        drop(Owner(scope.clone()));
        assert!(scope.live().is_none());
    }

    #[test]
    fn ca_charge_boundaries_and_overflow() {
        assert_eq!(Size::default().charge(), Some(65536));
        assert_eq!(
            Size {
                n: 1,
                ..Size::default()
            }
            .charge(),
            Some(68096)
        );
        let mut targets = Vec::with_capacity(7);
        let mut url = String::with_capacity(151);
        url.push_str("http://localhost/");
        let mut pem = String::with_capacity(183);
        pem.push_str("unparsed");
        targets.push(WebhookTarget {
            url,
            inbox: Some("i".into()),
            secret: Some("s".into()),
            ca_pem: Some(pem),
        });
        targets.push(WebhookTarget {
            url: String::new(),
            inbox: None,
            secret: None,
            ca_pem: None,
        });
        let size = Size::targets(&targets, targets.capacity()).unwrap();
        assert_eq!(
            (size.n, size.u, size.i, size.h, size.p, size.r, size.a),
            (2, 151, 1, 1, 183, 0, 1)
        );
        assert_eq!(size.vi, 7 * std::mem::size_of::<WebhookTarget>());
        assert_eq!(size.vo, 2 * std::mem::size_of::<WebhookTarget>());
        assert!(
            size.charge().unwrap()
                > Size::targets(&targets, targets.len())
                    .unwrap()
                    .charge()
                    .unwrap()
        );
        assert!(Size::targets(&targets, usize::MAX).is_none());
        // Exercise each checked intermediate with descriptors, without materializing input.
        for value in [
            usize::MAX,
            usize::MAX / 2,
            usize::MAX / 4,
            usize::MAX / 8,
            usize::MAX / 55,
            usize::MAX / 512,
            usize::MAX / 65536,
        ] {
            for field in 0..10 {
                let mut s = Size::default();
                match field {
                    0 => s.n = value,
                    1 => s.u = value,
                    2 => s.i = value,
                    3 => s.h = value,
                    4 => s.p = value,
                    5 => s.vi = value,
                    6 => s.vo = value,
                    7 => s.r = value,
                    8 => s.a = value,
                    _ => {
                        s.u = value;
                        s.i = value;
                        s.p = value;
                    }
                }
                let n = s.n as u128;
                let u = s.u as u128;
                let i = s.i as u128;
                let p = s.p as u128;
                let r = s.r as u128;
                let e = 4 * (p / 3 + r);
                let c = 55 * r + e + e / 64;
                let o = 8 * u + 256 * n + i + c;
                let reference = 65536
                    + 512 * n
                    + s.vi as u128
                    + s.vo as u128
                    + 4 * (u + i + s.h as u128 + p)
                    + 8 * o
                    + 8 * (p + c)
                    + 512 * r
                    + 65536 * s.a as u128;
                assert_eq!(s.charge(), usize::try_from(reference).ok());
            }
        }
        assert!(
            Size {
                u: usize::MAX,
                i: 1,
                ..Size::default()
            }
            .charge()
            .is_none()
        );
        for charge in [BYTE_CAP - 1, BYTE_CAP, BYTE_CAP + 1] {
            assert_eq!(
                Size {
                    vi: charge - 65536,
                    ..Size::default()
                }
                .charge(),
                Some(charge)
            );
        }
    }

    #[test]
    fn canonical_ca_output_fits_charge_envelope() {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()])
            .unwrap()
            .cert
            .pem();
        for pem in [
            String::new(),
            "x".into(),
            "-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n".into(),
            cert.clone(),
            cert.repeat(40),
        ] {
            let p = pem.len();
            let r = p / 27;
            let e = 4 * (p / 3 + r);
            let c = 55 * r + e + e / 64;
            assert!(crate::webhook::public_ca_pem(&pem).len() <= c);
            assert!(Size::delivery(&pem).charge().is_some());
        }
    }
}
