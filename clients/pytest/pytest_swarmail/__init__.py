"""The swarmail REST client behind the pytest fixture.

It imports no pytest and has no dependency beyond the standard library. The
wait is push-based and reads only what the server stored at ingest (decision
0005): ``GET .../await?limit=0`` is the wake-up (decision 0002, subscribe
before scan), ``GET .../messages`` is the read, and the selection re-states
proposed 0094 section 5 (codes) and section 6 (links). It never decodes MIME,
parses HTML or runs a regex over a body.
"""

from __future__ import annotations

import http.client
import json
import math
import os
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid
from typing import Any, Callable, Dict, List, Optional

__all__ = [
    "Inbox",
    "SwarmailAmbiguous",
    "SwarmailClient",
    "SwarmailError",
    "SwarmailHttpError",
    "SwarmailTimeout",
    "new_inbox_name",
    "pick_code",
    "pick_link",
]

DEFAULT_URL = "http://127.0.0.1:8025"
DEFAULT_TIMEOUT_MS = 5000
# The server refuses a ``timeout_ms`` above 60000 (decision 0018); a longer
# wait is sliced into several awaits.
SLICE_MS = 60000
# The most the list returns in one page; a per-test inbox never holds this
# many matches, so a larger total is a test bug and fails loudly.
PAGE = 250
# Added to every request's socket timeout so a hung server cannot hang the
# test, without racing the server's own 408.
SOCKET_MARGIN_S = 5.0


class SwarmailError(Exception):
    """Base class; extra details are attributes."""

    def __init__(self, message: str, **details: Any) -> None:
        super().__init__(message)
        self.__dict__.update(details)


class SwarmailTimeout(SwarmailError):
    """The wait ran out of time with nothing selectable."""


class SwarmailAmbiguous(SwarmailError):
    """More than one candidate; the fixture never silently picks the first."""


class SwarmailHttpError(SwarmailError):
    """Any other refusal or transport failure; ``status`` is 0 for a transport failure."""


def new_inbox_name() -> str:
    """``t`` followed by a random UUID's 32 hex digits: the SMTP AUTH username (decision 0004)."""
    return "t" + uuid.uuid4().hex


# The server omits an empty ``codes`` or ``links`` from the JSON, so an
# absent field is an empty one.
def _codes(email: Dict[str, Any]) -> List[str]:
    return email.get("codes") or []


def _links(email: Dict[str, Any]) -> List[str]:
    return email.get("links") or []


def pick_code(emails: List[Dict[str, Any]]) -> Optional[Dict[str, Any]]:
    """The newest mail with any code decides (proposed 0094 section 5).

    Returns ``{"value": (code, email)}``, ``{"ambiguous": [codes]}`` or None.
    """
    newest = next((email for email in emails if _codes(email)), None)
    if newest is None:
        return None
    candidates = list(dict.fromkeys(_codes(newest)))
    if len(candidates) > 1:
        return {"ambiguous": candidates}
    return {"value": (candidates[0], newest)}


def pick_link(
    emails: List[Dict[str, Any]], match: Optional[Callable[[str], bool]]
) -> Optional[Dict[str, Any]]:
    """Every link across the whole filtered snapshot (proposed 0094 section 6)."""
    candidates: List[str] = []
    for email in emails:
        for link in _links(email):
            if link not in candidates and (match is None or match(link)):
                candidates.append(link)
    if not candidates:
        return None
    if len(candidates) > 1:
        return {"ambiguous": candidates}
    newest = next(email for email in emails if candidates[0] in _links(email))
    return {"value": (candidates[0], newest)}


class SwarmailClient:
    def __init__(self, base_url: Optional[str] = None) -> None:
        self.base_url = (base_url or os.environ.get("SWARMAIL_URL") or DEFAULT_URL).rstrip("/")

    def wait_for_code(self, inbox: str, **options: Any) -> "CodeMatch":
        picked = self._wait(inbox, options, pick_code)
        return CodeMatch(*picked)

    def wait_for_link(
        self, inbox: str, match: Optional[Callable[[str], bool]] = None, **options: Any
    ) -> "LinkMatch":
        if match is not None and not callable(match):
            raise TypeError("wait_for_link: match must be a function of the URL, or None")
        picked = self._wait(inbox, options, lambda emails: pick_link(emails, match))
        return LinkMatch(*picked)

    def _get(self, path: str, params: Dict[str, str], socket_s: float) -> "tuple[int, str]":
        url = f"{self.base_url}/api/v1/inboxes/{path}?{urllib.parse.urlencode(params)}"
        try:
            with urllib.request.urlopen(url, timeout=socket_s) as res:
                return res.status, res.read().decode("utf-8")
        except urllib.error.HTTPError as err:
            with err:
                return err.code, err.read().decode("utf-8", "replace")
        except (OSError, ValueError, http.client.HTTPException) as err:  # URLError, timeouts, resets, bad reads
            raise SwarmailHttpError(
                f"swarmail request failed: GET {url}: {err}", status=0, body=str(err)
            ) from err

    def _wait(
        self,
        inbox: str,
        options: Dict[str, Any],
        pick: Callable[[List[Dict[str, Any]]], Optional[Dict[str, Any]]],
    ) -> Any:
        timeout_ms = options.pop("timeout_ms", DEFAULT_TIMEOUT_MS)
        filters = {key: options.pop(key, None) for key in ("to", "from_", "subject", "since_ms")}
        if options:
            raise TypeError(f"unexpected wait options: {', '.join(sorted(options))}")
        if not isinstance(timeout_ms, int) or isinstance(timeout_ms, bool) or timeout_ms < 0:
            raise ValueError(f"timeout_ms must be a non-negative integer, got {timeout_ms!r}")
        query: Dict[str, str] = {}
        for key, value in (
            ("to", filters["to"]),
            ("from", filters["from_"]),
            ("subject", filters["subject"]),
            ("since_ms", filters["since_ms"]),
        ):
            if value is not None:
                query[key] = str(value)
        name = urllib.parse.quote(inbox, safe="")
        deadline = time.monotonic() + timeout_ms / 1000
        count = 1
        matched = 0
        while True:
            remaining_ms = min(timeout_ms, max(0, math.ceil((deadline - time.monotonic()) * 1000)))
            slice_ms = min(SLICE_MS, remaining_ms)
            wake = {**query, "count": str(count), "limit": "0", "timeout_ms": str(slice_ms)}
            status, body = self._get(f"{name}/await", wake, slice_ms / 1000 + SOCKET_MARGIN_S)
            if status == 408:
                if slice_ms < remaining_ms:
                    continue  # one slice of a longer wait
                raise SwarmailTimeout(
                    f"no selectable mail in inbox {inbox} within {timeout_ms} ms ({matched} matching)",
                    inbox=inbox,
                    filters=filters,
                    timeout_ms=timeout_ms,
                    matched=matched,
                )
            if status != 200:
                raise self._refused(status, body, inbox, "await")

            read = {**query, "limit": str(PAGE)}
            status, body = self._get(f"{name}/messages", read, remaining_ms / 1000 + SOCKET_MARGIN_S)
            if status != 200:
                raise self._refused(status, body, inbox, "messages")
            listed = json.loads(body)
            emails = listed["emails"]
            if listed["total"] > len(emails):
                raise SwarmailError(
                    f"inbox {inbox} holds {listed['total']} matching mails, more than the {PAGE} one read returns",
                    inbox=inbox,
                    filters=filters,
                    total=listed["total"],
                )
            matched = listed["total"]
            picked = pick(emails)
            if picked is not None and "ambiguous" in picked:
                candidates = picked["ambiguous"]
                raise SwarmailAmbiguous(
                    f"ambiguous in inbox {inbox}: {len(candidates)} candidates: {', '.join(candidates)}",
                    inbox=inbox,
                    filters=filters,
                    candidates=candidates,
                )
            if picked is not None:
                return picked["value"]
            count = listed["total"] + 1

    @staticmethod
    def _refused(status: int, body: str, inbox: str, route: str) -> SwarmailHttpError:
        return SwarmailHttpError(
            f"swarmail {route} for inbox {inbox} answered {status}: {body}", status=status, body=body
        )


class CodeMatch:
    """``code`` and the REST ``email`` object it came from."""

    def __init__(self, code: str, email: Dict[str, Any]) -> None:
        self.code = code
        self.email = email


class LinkMatch:
    """``link`` and the REST ``email`` object (the newest carrying it)."""

    def __init__(self, link: str, email: Dict[str, Any]) -> None:
        self.link = link
        self.email = email


class Inbox:
    """A per-test inbox: ``name`` is the SMTP AUTH username."""

    def __init__(self, client: SwarmailClient, name: str) -> None:
        self._client = client
        self.name = name

    def wait_for_code(self, **options: Any) -> CodeMatch:
        return self._client.wait_for_code(self.name, **options)

    def wait_for_link(self, match: Optional[Callable[[str], bool]] = None, **options: Any) -> LinkMatch:
        return self._client.wait_for_link(self.name, match, **options)
