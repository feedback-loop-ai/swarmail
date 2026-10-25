//! A request-local cancellation seam around the real PUT handler future.
//! Only an explicitly installed hook for this Webhooks instance is affected.
//! The production handler (including preparation and publication) is polled
//! unchanged; Abortable drops that entire future before acknowledging cancel.
use super::*;
use futures_util::future::{AbortHandle, AbortRegistration, Abortable};
use std::sync::Mutex;

struct Cancellation {
    owner: Arc<Webhooks>,
    registration: AbortRegistration,
    dropped: tokio::sync::oneshot::Sender<()>,
}
static CANCELLATION: Mutex<Option<Cancellation>> = Mutex::new(None);

pub(crate) fn cancel_put_for(
    owner: Arc<Webhooks>,
) -> (AbortHandle, tokio::sync::oneshot::Receiver<()>) {
    let (handle, registration) = AbortHandle::new_pair();
    let (dropped, observed) = tokio::sync::oneshot::channel();
    let mut hook = CANCELLATION.lock().unwrap();
    assert!(hook.is_none(), "one request-local cancellation at a time");
    *hook = Some(Cancellation {
        owner,
        registration,
        dropped,
    });
    (handle, observed)
}

pub(super) async fn cancellable_set_webhooks(
    State(state): State<AppState>,
    body: Result<Json<WebhookTargets>, JsonRejection>,
) -> Result<Json<Vec<WebhookTarget>>, BodyError> {
    let hook = {
        let mut hook = CANCELLATION.lock().unwrap();
        if hook
            .as_ref()
            .is_some_and(|hook| Arc::ptr_eq(&hook.owner, &state.webhooks))
        {
            hook.take()
        } else {
            None
        }
    };
    if let Some(hook) = hook {
        let outcome =
            Abortable::new(super::set_webhooks(State(state), body), hook.registration).await;
        assert!(
            outcome.is_err(),
            "cancel the real PUT owner before releasing preparation"
        );
        let _ = hook.dropped.send(());
        return Err(ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            "test cancelled PUT owner".into(),
        )
        .into());
    }
    super::set_webhooks(State(state), body).await
}

static GET_CANCELLATION: Mutex<Option<Cancellation>> = Mutex::new(None);
pub(crate) fn cancel_get_for(
    owner: Arc<Webhooks>,
) -> (AbortHandle, tokio::sync::oneshot::Receiver<()>) {
    let (handle, registration) = AbortHandle::new_pair();
    let (dropped, observed) = tokio::sync::oneshot::channel();
    let mut hook = GET_CANCELLATION.lock().unwrap();
    assert!(hook.is_none());
    *hook = Some(Cancellation {
        owner,
        registration,
        dropped,
    });
    (handle, observed)
}
pub(super) async fn cancellable_get_webhooks(
    State(state): State<AppState>,
    raw: RawQuery,
) -> ApiResult<Json<Vec<WebhookTarget>>> {
    let hook = {
        let mut hook = GET_CANCELLATION.lock().unwrap();
        if hook
            .as_ref()
            .is_some_and(|hook| Arc::ptr_eq(&hook.owner, &state.webhooks))
        {
            hook.take()
        } else {
            None
        }
    };
    if let Some(hook) = hook {
        let outcome =
            Abortable::new(super::get_webhooks(State(state), raw), hook.registration).await;
        assert!(outcome.is_err(), "cancel the actual GET owner");
        let _ = hook.dropped.send(());
        return Err(ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            "test cancelled GET owner".into(),
        ));
    }
    super::get_webhooks(State(state), raw).await
}
