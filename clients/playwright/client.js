// The swarmail REST client behind the Playwright fixture. It imports no
// framework: Node 18+ (global `fetch`, `crypto.randomUUID`) is the only
// runtime, so the client can be driven from a plain script.
//
// The wait is push-based and reads only what the server stored at ingest
// (decision 0005): `GET .../await?limit=0` is the wake-up (decision 0002,
// subscribe before scan), `GET .../messages` is the read, and the selection
// below re-states proposed 0094 section 5 (codes) and section 6 (links).
// It never decodes MIME, parses HTML or runs a regex over a body.

import { randomUUID } from 'node:crypto';

const DEFAULT_URL = 'http://127.0.0.1:8025';
const DEFAULT_TIMEOUT_MS = 5000;
// The server refuses a `timeout_ms` above 60000 (decision 0018); a longer
// wait is sliced into several awaits.
const SLICE_MS = 60000;
// The most the list returns in one page; a per-test inbox never holds this
// many matches, so a larger total is a test bug and fails loudly.
const PAGE = 250;
// Added to every request's socket timeout so a hung server cannot hang the
// test, without racing the server's own 408.
const SOCKET_MARGIN_MS = 5000;

export class SwarmailError extends Error {
  constructor(message, details = {}) {
    super(message);
    this.name = new.target.name;
    Object.assign(this, details);
  }
}

/** The wait ran out of time with nothing selectable. */
export class SwarmailTimeout extends SwarmailError {}

/** More than one candidate; the fixture never silently picks the first. */
export class SwarmailAmbiguous extends SwarmailError {}

/** Any other refusal or transport failure; `status` is 0 for a transport failure. */
export class SwarmailHttpError extends SwarmailError {}

/** `t` followed by a random UUID's 32 hex digits: the SMTP AUTH username (decision 0004). */
export function newInboxName() {
  return `t${randomUUID().replaceAll('-', '')}`;
}

// The server omits an empty `codes` or `links` from the JSON (`Vec::is_empty`
// is skipped on the wire), so an absent field is an empty one.
const codesOf = (email) => email.codes ?? [];
const linksOf = (email) => email.links ?? [];

/** The newest mail with any code decides (proposed 0094 section 5). */
export function pickCode(emails) {
  const newest = emails.find((email) => codesOf(email).length > 0);
  if (!newest) return null;
  const candidates = [...new Set(codesOf(newest))];
  if (candidates.length > 1) return { ambiguous: candidates };
  return { value: { code: candidates[0], email: newest } };
}

/** Every link across the whole filtered snapshot (proposed 0094 section 6). */
export function pickLink(emails, match) {
  const candidates = [];
  for (const email of emails) {
    for (const link of linksOf(email)) {
      if (!candidates.includes(link) && (!match || match(link))) candidates.push(link);
    }
  }
  if (candidates.length === 0) return null;
  if (candidates.length > 1) return { ambiguous: candidates };
  const email = emails.find((mail) => linksOf(mail).includes(candidates[0]));
  return { value: { link: candidates[0], email } };
}

export class SwarmailClient {
  constructor({ baseUrl = process.env.SWARMAIL_URL || DEFAULT_URL } = {}) {
    this.baseUrl = baseUrl.replace(/\/+$/, '');
  }

  waitForCode(inbox, options = {}) {
    return this.#wait(inbox, options, pickCode);
  }

  waitForLink(inbox, match, options = {}) {
    if (match != null && typeof match !== 'function') {
      throw new TypeError('waitForLink: match must be a function of the URL, or omitted');
    }
    return this.#wait(inbox, options, (emails) => pickLink(emails, match));
  }

  async #get(path, params, socketMs) {
    const url = `${this.baseUrl}/api/v1/inboxes/${path}?${params}`;
    let res;
    let body;
    try {
      res = await fetch(url, { signal: AbortSignal.timeout(socketMs) });
      body = await res.text();
    } catch (cause) {
      throw new SwarmailHttpError(`swarmail request failed: GET ${url}: ${cause}`, {
        status: 0,
        body: String(cause),
        cause,
      });
    }
    return { status: res.status, body };
  }

  async #wait(inbox, options, pick) {
    const { timeoutMs = DEFAULT_TIMEOUT_MS, to, from, subject, sinceMs } = options;
    if (!Number.isInteger(timeoutMs) || timeoutMs < 0) {
      throw new RangeError(`timeoutMs must be a non-negative integer, got ${timeoutMs}`);
    }
    const filters = { to, from, subject, sinceMs };
    const query = new URLSearchParams();
    for (const [key, value] of [['to', to], ['from', from], ['subject', subject], ['since_ms', sinceMs]]) {
      if (value !== undefined && value !== null) query.set(key, String(value));
    }
    const name = encodeURIComponent(inbox);
    const deadline = performance.now() + timeoutMs;
    let count = 1;
    let matched = 0;
    for (;;) {
      const remaining = Math.max(0, Math.ceil(deadline - performance.now()));
      const slice = Math.min(SLICE_MS, remaining);
      const wake = new URLSearchParams(query);
      wake.set('count', String(count));
      wake.set('limit', '0');
      wake.set('timeout_ms', String(slice));
      const woke = await this.#get(`${name}/await`, wake, slice + SOCKET_MARGIN_MS);
      if (woke.status === 408) {
        if (slice < remaining) continue; // one slice of a longer wait
        throw new SwarmailTimeout(
          `no selectable mail in inbox ${inbox} within ${timeoutMs} ms (${matched} matching)`,
          { inbox, filters, timeoutMs, matched },
        );
      }
      if (woke.status !== 200) throw this.#refused(woke, inbox, 'await');

      const read = new URLSearchParams(query);
      read.set('limit', String(PAGE));
      const listed = await this.#get(`${name}/messages`, read, Math.max(remaining, 0) + SOCKET_MARGIN_MS);
      if (listed.status !== 200) throw this.#refused(listed, inbox, 'messages');
      const list = JSON.parse(listed.body);
      if (list.total > list.emails.length) {
        throw new SwarmailError(
          `inbox ${inbox} holds ${list.total} matching mails, more than the ${PAGE} one read returns`,
          { inbox, filters, total: list.total },
        );
      }
      matched = list.total;
      const picked = pick(list.emails);
      if (picked?.ambiguous) {
        throw new SwarmailAmbiguous(
          `ambiguous in inbox ${inbox}: ${picked.ambiguous.length} candidates: ${picked.ambiguous.join(', ')}`,
          { inbox, filters, candidates: picked.ambiguous },
        );
      }
      if (picked) return picked.value;
      count = list.total + 1;
    }
  }

  #refused(res, inbox, route) {
    return new SwarmailHttpError(`swarmail ${route} for inbox ${inbox} answered ${res.status}: ${res.body}`, {
      status: res.status,
      body: res.body,
    });
  }
}
