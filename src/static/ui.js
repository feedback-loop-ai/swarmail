// Swarmail run console: declarative over the JSON API and the feed. The
// server decides what a thread is; the browser only paints it. Mail-controlled
// and client-controlled text reaches the page through textContent, and the
// one HTML preview through the srcdoc of a bare-sandbox frame. Appearance is
// chosen by class from fixed tables, never from mail data, and the only link
// this script builds points at a same-origin console route.
const view = document.getElementById('view');
const inbox = view.dataset.inbox;
const threadKey = view.dataset.thread || null;
const panes = document.getElementById('panes');

const TICK_MS = 2000;
const CACHE_MAX = 512;
const SEEN_MAX = 512;
const LIST_CAP = 250;
const SEEN_KEY = 'swarmail.console.seen';
const SEVERITY_RANK = new Map([['clean', 0], ['info', 1], ['warning', 2], ['error', 3]]);
const SEVERITY_CLASS = new Map([['clean', 'clean'], ['info', 'info'], ['warning', 'warning'], ['error', 'error']]);
const CHAOS_EVENTS = ['connect', 'mail_from', 'rcpt', 'data'];

function $(id) {
  return document.getElementById(id);
}

// A node with a class and text. Everything mail-controlled goes through
// `text` so the browser can never parse it as markup.
function el(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined && text !== null) node.textContent = text;
  return node;
}

function button(className, text, onclick) {
  const node = el('button', className, text);
  node.type = 'button';
  node.addEventListener('click', onclick);
  return node;
}

function api(path) {
  return '/api/v1/inboxes/' + encodeURIComponent(inbox) + '/' + path;
}

function chip(severity, text) {
  return el('span', 'sev ' + (SEVERITY_CLASS.get(severity) || ''), text);
}

let toastTimer = 0;
function toast(message) {
  const node = $('toast');
  node.textContent = message;
  node.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    node.hidden = true;
  }, 1800);
}

function copyControl(text, what) {
  return button('btn', 'Copy', () => {
    const blocked = () => toast('Copy blocked here; select the text instead');
    try {
      navigator.clipboard.writeText(text).then(() => toast('Copied ' + what), blocked);
    } catch (e) {
      blocked();
    }
  });
}

// ---------- formatting ----------

function ago(ms) {
  const s = Math.max(0, Math.round((Date.now() - ms) / 1000));
  if (s < 60) return s + 's ago';
  if (s < 3600) return Math.round(s / 60) + 'm ago';
  if (s < 86400) return Math.round(s / 3600) + 'h ago';
  return Math.round(s / 86400) + 'd ago';
}

function clock(ms) {
  return new Date(ms).toLocaleTimeString([], { hour12: false });
}

function bytes(n) {
  if (n < 1024) return n + ' B';
  if (n < 1048576) return (n / 1024).toFixed(1) + ' KB';
  return (n / 1048576).toFixed(1) + ' MB';
}

function plural(n, one, many) {
  return n + ' ' + (n === 1 ? one : many);
}

function address(a) {
  if (!a) return '?';
  return a.name ? a.name + ' <' + a.address + '>' : a.address;
}

function locationText(l) {
  if (!l || typeof l !== 'object') return '';
  if (l.kind === 'header') {
    return 'header ' + l.name + (l.occurrence === null || l.occurrence === undefined ? '' : ' #' + l.occurrence);
  }
  const path = (p) => (Array.isArray(p) && p.length ? p.join('.') : 'root');
  if (l.kind === 'mime_part') return 'MIME part ' + path(l.path);
  if (l.kind === 'link') return 'link in part ' + path(l.part_path) + ', byte ' + l.byte_offset;
  return '';
}

// A path segment safe to paste inside a single-quoted shell word: the
// percent-encoding encodeURIComponent applies, plus the five characters it
// leaves (a quote among them).
function shellSegment(name) {
  return encodeURIComponent(name).replace(/[!'()*]/g, (c) => '%' + c.charCodeAt(0).toString(16).toUpperCase());
}

function escapeRegex(text) {
  return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function rank(severity) {
  return SEVERITY_RANK.get(severity) || 0;
}

function worstOf(findings) {
  let worst = 'clean';
  for (const f of findings) if (rank(f.severity) > rank(worst)) worst = f.severity;
  return worst;
}

// ---------- reads ----------

// The body is parsed before anyone looks at the status: a refusal that names
// its cause is read by name, never guessed from the code. Rejects only when
// the fetch or the parse fails.
async function readJson(path) {
  const response = await fetch(path);
  const body = await response.json();
  return { status: response.status, body: body };
}

// ---------- status bar ----------

let tickCount = 0;
let ticking = false;

function listener(id, label, state, detail) {
  const node = $(id);
  node.className = 'ep ' + state;
  node.replaceChildren(document.createTextNode(label + ' '), el('b', null, detail || state));
}

function paintHealth(body) {
  const known = body && typeof body === 'object' && typeof body.status === 'string';
  const named = known && Array.isArray(body.degraded) ? body.degraded : [];
  const stateOf = (name) => {
    if (!known) return 'unknown';
    if (body.status === 'stopping') return 'stopping';
    if (body.status === 'degraded') return named.includes(name) ? 'degraded' : 'ok';
    return body.status === 'ok' ? 'ok' : 'unknown';
  };
  listener('listener-smtp', 'SMTP', stateOf('smtp'));
  listener('listener-pop3', 'POP3', stateOf('pop3'));
  // An answer came, so HTTP is up unless the server says it is leaving.
  const http = known && body.status !== 'stopping' ? 'ok' : known ? 'stopping' : 'unknown';
  listener('listener-http', 'HTTP', http, http === 'ok' ? location.host : location.host + ' (' + http + ')');
  // A stopping answer carries no counters: the bar keeps the last values.
  if (known && typeof body.emails_inserted_total === 'number') {
    $('count-accepted').textContent = body.emails_inserted_total.toLocaleString('en-US');
  }
  if (known && typeof body.emails_dropped_total === 'number') {
    $('count-dropped').textContent = body.emails_dropped_total.toLocaleString('en-US');
  }
}

async function readHealth() {
  try {
    paintHealth((await readJson('/healthz')).body);
  } catch (e) {
    paintHealth(null);
  }
}

function paintChaos(body) {
  const node = $('chaos');
  const complete = body && typeof body === 'object' && CHAOS_EVENTS.every((name) => name in body);
  if (!complete) {
    node.className = 'chip chaos-unknown';
    node.textContent = 'Chaos unknown';
    return;
  }
  const active = CHAOS_EVENTS.filter((name) => body[name]);
  if (!active.length) {
    node.className = 'chip';
    node.textContent = 'Chaos off';
    return;
  }
  node.className = 'chip chaos-on';
  node.textContent = 'Chaos on · ' + active.map((name) => name + ' ' + body[name].probability + '%').join(', ');
}

async function readChaos() {
  try {
    paintChaos((await readJson('/api/v1/chaos')).body);
  } catch (e) {
    paintChaos(null);
  }
}

function setLive(state) {
  const states = new Map([
    ['live', ['live live-on', 'Live']],
    ['retry', ['live live-retry', 'Reconnecting']],
    ['off', ['live live-off', 'Offline']],
    ['idle', ['live', 'Idle']],
  ]);
  const [className, text] = states.get(state);
  const node = $('live');
  node.className = className;
  node.lastChild.textContent = text;
}

// ---------- inboxes ----------

const railRows = $('inbox-rows');
const railEmpty = $('inbox-empty');
const railNote = $('inbox-note');
const railFilter = $('inbox-filter');
const railLimit = railRows.dataset.limit;
const railOffset = railRows.dataset.offset;

let railList = [];
let polled = false;
let railKey = null;
const inboxCounts = new Map();
const dotNames = new Set();

// The last server stamp seen per inbox. Only server stamps are compared, never
// the browser clock. Kept in sessionStorage so a dot survives the navigation
// that opens another inbox; storage that throws or holds anything unexpected
// is treated as empty and the page works from memory.
function loadSeen() {
  const map = new Map();
  try {
    const raw = sessionStorage.getItem(SEEN_KEY);
    const pairs = raw ? JSON.parse(raw) : [];
    if (!Array.isArray(pairs)) return map;
    for (const pair of pairs) {
      if (!Array.isArray(pair) || pair.length !== 2 || typeof pair[0] !== 'string' || typeof pair[1] !== 'number') {
        return new Map();
      }
      map.set(pair[0], pair[1]);
    }
  } catch (e) {
    return new Map();
  }
  while (map.size > SEEN_MAX) map.delete(map.keys().next().value);
  return map;
}

const lastSeen = loadSeen();
let seenDirty = false;

function remember(name, ms) {
  lastSeen.delete(name);
  lastSeen.set(name, ms);
  while (lastSeen.size > SEEN_MAX) lastSeen.delete(lastSeen.keys().next().value);
  seenDirty = true;
}

function saveSeen() {
  if (!seenDirty) return;
  seenDirty = false;
  try {
    sessionStorage.setItem(SEEN_KEY, JSON.stringify(Array.from(lastSeen)));
  } catch (e) {
    // memory only
  }
}

function wantsDot(name, stamp) {
  if (typeof stamp !== 'number') return false;
  if (name === inbox) {
    remember(name, stamp);
    return false;
  }
  if (!lastSeen.has(name)) {
    if (!polled) {
      remember(name, stamp);
      return false;
    }
    // First observed after the page's first poll: older than anything seen.
    remember(name, 0);
  }
  return stamp > lastSeen.get(name);
}

function railRow(item) {
  const a = el('a', 'inbox');
  a.href = '/ui/inbox/' + encodeURIComponent(item.name);
  if (item.name === inbox) a.setAttribute('aria-current', 'true');
  a.appendChild(el('span', 'name', item.name));
  a.appendChild(el('span', 'count', String(item.count)));
  const when = el('span', 'when');
  if (dotNames.has(item.name)) {
    const dot = el('span', 'newdot');
    dot.setAttribute('title', 'New mail');
    when.appendChild(dot);
  }
  when.appendChild(
    document.createTextNode(typeof item.last_received_ms === 'number' ? 'last mail ' + ago(item.last_received_ms) : 'empty, waiting')
  );
  a.appendChild(when);
  return a;
}

function renderRail() {
  const needle = railFilter.value.trim().toLowerCase();
  const shown = railList.filter((item) => item.name.toLowerCase().includes(needle));
  // Rebuild only when something visible changed: a fresh node every tick
  // would drop keyboard focus and swallow a click that straddles it.
  const key = JSON.stringify([
    inbox,
    railList.length,
    shown.map((item) => [item.name, item.count, dotNames.has(item.name), typeof item.last_received_ms === 'number' ? ago(item.last_received_ms) : null]),
  ]);
  if (key !== railKey) {
    railKey = key;
    railRows.replaceChildren();
    for (const item of shown) railRows.appendChild(railRow(item));
    if (railList.length && !shown.length) railRows.appendChild(el('div', 'empty', 'No inbox matches.'));
  }
  $('inbox-total').textContent = String(railList.length);
  railEmpty.hidden = railList.length > 0;
}

function applyPoll(list) {
  railList = list.filter((item) => item && typeof item.name === 'string' && typeof item.count === 'number');
  dotNames.clear();
  inboxCounts.clear();
  for (const item of railList) {
    inboxCounts.set(item.name, item.count);
    if (wantsDot(item.name, item.last_received_ms)) dotNames.add(item.name);
  }
  polled = true;
  saveSeen();
  railNote.hidden = true;
  renderRail();
  paintHeader();
}

async function readInboxes() {
  try {
    const result = await readJson('/api/v1/inboxes?limit=' + railLimit + '&offset=' + railOffset);
    if (!Array.isArray(result.body)) throw new Error('inbox list');
    applyPoll(result.body);
  } catch (e) {
    // A failed read keeps the rows it painted and never says "no mail".
    railNote.textContent = 'Inboxes unavailable — retrying';
    railNote.hidden = false;
  }
}

railFilter.addEventListener('input', () => {
  if (polled) renderRail();
});

// ---------- enrichment: projection and report, one request at a time ----------

const cache = new Map();
const queue = [];
const queued = new Set();
let pumping = false;
let fetching = null;

function settled(state) {
  return Boolean(state) && (state.state === 'ok' || state.state === 'gone');
}

function failed(state) {
  return Boolean(state) && state.state === 'failed';
}

// A refusal that repeating cannot change: the analysis failed for this mail
// (limits exceeded), unlike a busy 503, which a later tick may clear.
function terminal(state) {
  return failed(state) && state.terminal === true;
}

function cacheSet(id, record) {
  cache.delete(id);
  cache.set(id, record);
  while (cache.size > CACHE_MAX) cache.delete(cache.keys().next().value);
}

function enrich(id, first) {
  const record = cache.get(id);
  if (record && settled(record.projection) && settled(record.report)) return;
  if (record && record.retryAt > tickCount) return;
  // A fetch in flight settles or sets its own retry: never queue it twice.
  if (id === fetching) return;
  if (queued.has(id)) {
    if (first) {
      queue.splice(queue.indexOf(id), 1);
      queue.unshift(id);
    }
    return;
  }
  queued.add(id);
  if (first) queue.unshift(id);
  else queue.push(id);
}

async function fetchState(path) {
  try {
    const result = await readJson(path);
    if (result.status === 404) return { state: 'gone', body: null };
    if (result.status === 503 && result.body && result.body.error === 'deliverability analysis failed') {
      return { state: 'failed', terminal: true, body: null };
    }
    if (result.status === 200 && result.body && typeof result.body === 'object') return { state: 'ok', body: result.body };
  } catch (e) {
    // falls through: a failure is not cached
  }
  return { state: 'failed', body: null };
}

async function fetchOne(id) {
  let record = cache.get(id);
  if (!record) {
    record = { projection: null, report: null, retryAt: 0 };
    cacheSet(id, record);
  }
  if (!settled(record.projection) && !terminal(record.projection)) {
    record.projection = await fetchState('/api/v1/messages/' + encodeURIComponent(id) + '/projection');
    // A pruned or cleared mail is gone for both reads: no second request.
    if (record.projection.state === 'gone') record.report = record.projection;
    afterStep(id);
  }
  if (!settled(record.report) && !terminal(record.report)) {
    record.report = await fetchState('/api/v1/messages/' + encodeURIComponent(id) + '/deliverability');
    if (record.report.state === 'gone') record.projection = record.report;
  }
  if (failed(record.projection) || failed(record.report)) {
    const again = (failed(record.projection) && !terminal(record.projection)) || (failed(record.report) && !terminal(record.report));
    record.retryAt = again ? tickCount + 1 : Infinity;
  }
  afterStep(id);
}

async function pump() {
  if (pumping) return;
  pumping = true;
  try {
    while (queue.length) {
      const id = queue.shift();
      queued.delete(id);
      // Rows that left the view are not worth a request.
      if (rowSlots.has(id) || id === selectedId) {
        fetching = id;
        try {
          await fetchOne(id);
        } finally {
          fetching = null;
        }
      }
    }
  } finally {
    pumping = false;
  }
}

function afterStep(id) {
  paintSlot(id);
  if (id === selectedId) refreshReader();
}

function requeueFailures() {
  for (const id of rowSlots.keys()) enrich(id, false);
  if (selectedId !== null) enrich(selectedId, true);
  pump();
}

// ---------- stream ----------

let lastThreads = null;
let mailById = new Map();
let rowSlots = new Map();
let rowButtons = new Map();
let known = new Set();
const fresh = new Set();
const arrived = new Set();
let firstPainted = false;
let streamFailed = false;
let selectedId = null;
let clearState = 'idle';

function heldCount() {
  let held = 0;
  if (lastThreads) for (const thread of lastThreads) held += thread.emails.length;
  return held;
}

function paintHeader() {
  $('stream-title').textContent = inbox;
  const count = $('stream-count');
  if (!inbox || lastThreads === null) {
    count.textContent = '';
  } else {
    const held = heldCount();
    const total = inboxCounts.get(inbox);
    count.textContent =
      !threadKey && typeof total === 'number' && total > held ? 'newest ' + held + ' of ' + total + ' mails' : plural(held, 'mail', 'mails');
  }
  renderClear();
}

let clearKey = null;

function renderClear() {
  const slot = $('clear-slot');
  if (!inbox || heldCount() === 0) {
    clearState = 'idle';
    if (clearKey !== 'none') slot.replaceChildren();
    clearKey = 'none';
    return;
  }
  const total = inboxCounts.get(inbox);
  const n = typeof total === 'number' ? total : heldCount();
  // The poll repaints the header every tick: keep the buttons (and the
  // focus on them) unless what they say changed.
  const key = clearState + ':' + n;
  if (key === clearKey) return;
  clearKey = key;
  slot.replaceChildren();
  if (clearState === 'busy') {
    slot.appendChild(el('span', 'muted', 'Clearing…'));
  } else if (clearState === 'armed') {
    const confirm = el('span', 'confirm');
    confirm.appendChild(el('span', 'muted', 'Clear ' + n + '?'));
    confirm.appendChild(button('btn', 'Clear', clearInbox));
    confirm.appendChild(
      button('btn ghost', 'Keep', () => {
        clearState = 'idle';
        renderClear();
      })
    );
    slot.appendChild(confirm);
  } else {
    if (clearState === 'failed') slot.appendChild(el('span', 'muted', 'Clear failed '));
    slot.appendChild(
      button('btn ghost', 'Clear inbox', () => {
        clearState = 'armed';
        renderClear();
      })
    );
  }
}

// The only write the console makes. The stream is repainted from the
// server's next answer, never optimistically.
async function clearInbox() {
  clearState = 'busy';
  renderClear();
  try {
    const response = await fetch(api('messages'), { method: 'DELETE' });
    if (!response.ok) throw new Error('clear');
  } catch (e) {
    clearState = 'failed';
    renderClear();
    return;
  }
  clearState = 'idle';
  await load();
}

function notice(text, problem) {
  return el('div', problem ? 'empty problem' : 'empty', text);
}

function rowChips(id) {
  const record = cache.get(id);
  const nodes = [];
  if (!record || !record.report || !record.projection) {
    nodes.push(el('span', 'sev', 'checking…'));
    return nodes;
  }
  if (record.projection.state === 'gone' || record.report.state === 'gone') {
    nodes.push(chip('info', 'pruned'));
    return nodes;
  }
  if (failed(record.projection) || failed(record.report)) {
    nodes.push(chip('warning', 'unavailable'));
    return nodes;
  }
  const attachments = record.projection.body.attachments;
  if (attachments && attachments.total > 0) {
    nodes.push(el('span', 'tag', plural(attachments.total, 'attachment', 'attachments')));
    if (attachments.items.some((part) => part.ics)) nodes.push(el('span', 'tag', 'calendar'));
  }
  const findings = Array.isArray(record.report.body.findings) ? record.report.body.findings : [];
  nodes.push(findings.length ? chip(worstOf(findings), plural(findings.length, 'finding', 'findings')) : chip('clean', 'no findings'));
  return nodes;
}

function paintSlot(id) {
  const slot = rowSlots.get(id);
  if (slot) slot.replaceChildren(...rowChips(id));
}

function messageRow(email) {
  const row = el('button', 'msg' + (fresh.has(email.id) ? ' fresh' : '') + (arrived.has(email.id) ? ' arrive' : ''));
  row.type = 'button';
  row.setAttribute('aria-current', String(email.id === selectedId));
  row.appendChild(el('span', 'from', (email.from && email.from.name ? email.from.name + ' · ' : '') + (email.from ? email.from.address : '?')));
  row.appendChild(el('span', 'time', clock(email.received_ms)));
  row.appendChild(el('span', 'subj', email.subject || '(no subject)'));
  const tags = el('span', 'tags');
  for (const code of email.codes || []) tags.appendChild(el('span', 'tag code', code));
  const links = Array.isArray(email.links) ? email.links.length : 0;
  if (links) tags.appendChild(el('span', 'tag', plural(links, 'link', 'links')));
  const slot = el('span', 'enrich');
  tags.appendChild(slot);
  row.appendChild(tags);
  row.addEventListener('click', () => select(email.id, true));
  rowSlots.set(email.id, slot);
  rowButtons.set(email.id, row);
  paintSlot(email.id);
  return row;
}

function threadLabel(thread) {
  const label = el('div', 'thread-label');
  const a = el('a', null, 'Thread · ' + thread.count + ' mails');
  a.href = '/ui/inbox/' + encodeURIComponent(inbox) + '/thread/' + encodeURIComponent(thread.key);
  label.appendChild(a);
  return label;
}

// Paint a whole view (thread list, or a single thread's conversation). Every
// frame is a full snapshot, so there is nothing to patch.
function paint(threads) {
  const initial = !firstPainted;
  firstPainted = true;
  streamFailed = false;
  lastThreads = threads;
  mailById = new Map();
  rowSlots = new Map();
  rowButtons = new Map();
  const ids = new Set();
  const mails = [];
  for (const thread of threads) {
    for (const email of thread.emails) {
      mails.push(email);
      mailById.set(email.id, email);
      ids.add(email.id);
      if (!initial && !known.has(email.id)) {
        fresh.add(email.id);
        arrived.add(email.id);
      }
    }
  }
  for (const id of Array.from(fresh)) if (!ids.has(id)) fresh.delete(id);
  known = ids;

  view.replaceChildren();
  if (!threads.length) {
    view.appendChild(notice('Empty and live. Send mail with SMTP AUTH user "' + inbox + '" and it appears here as it is accepted.'));
  }
  for (const thread of threads) {
    if (thread.count > 1) view.appendChild(threadLabel(thread));
    // The API lists a conversation oldest first; the stream shows newest first.
    for (const email of thread.emails.slice().reverse()) view.appendChild(messageRow(email));
  }
  arrived.clear();
  paintHeader();

  if (initial && selectedId === null && threads.length) {
    const newest = threads[0].emails[threads[0].emails.length - 1];
    select(newest.id, false);
  } else if (selectedId !== null && !mailById.has(selectedId) !== readerMissing) {
    paintReader();
  }
  mails.sort((a, b) => b.received_ms - a.received_ms);
  for (const email of mails) enrich(email.id, false);
  pump();
}

function paintStreamNote(text, problem) {
  lastThreads = null;
  rowSlots = new Map();
  rowButtons = new Map();
  view.replaceChildren(notice(text, problem));
  paintHeader();
}

// The newest frame wins: the SSE listener bumps the token, and a `load()`
// response that resolves after one abandons its paint instead of painting
// over newer mail. A client-side token is the whole race.
let paintSeq = 0;

async function load() {
  const seen = paintSeq;
  let result;
  try {
    result = await readJson(threadKey ? api('threads/' + encodeURIComponent(threadKey)) : api('threads'));
  } catch (e) {
    if (seen !== paintSeq) return;
    streamFailed = true;
    paintStreamNote('Stream unavailable — retrying', true);
    return;
  }
  if (seen !== paintSeq) return;
  if (result.status === 404) {
    // Not a transient failure: the thread is not there.
    streamFailed = false;
    paintStreamNote(result.body && typeof result.body.error === 'string' ? result.body.error : 'not found', true);
    return;
  }
  if (result.status !== 200) {
    streamFailed = true;
    paintStreamNote('Stream unavailable — retrying', true);
    return;
  }
  paint(Array.isArray(result.body) ? result.body : [result.body]);
}

// ---------- reader ----------

let activeTab = 'preview';
let readerMissing = false;
let topZone = null;
let tabStrip = null;
let tabBody = null;

function select(id, show) {
  selectedId = id;
  fresh.delete(id);
  for (const [rowId, row] of rowButtons) {
    row.setAttribute('aria-current', String(rowId === id));
    if (rowId === id) row.classList.remove('fresh');
  }
  if (show) panes.dataset.pane = 'reader';
  enrich(id, true);
  pump();
  paintReader();
}

function stateOf(id, part) {
  const record = cache.get(id);
  return record ? record[part] : null;
}

function panel(label) {
  const box = el('div', 'panel');
  const head = el('div', 'panel-head');
  head.appendChild(el('span', 'label', label));
  box.appendChild(head);
  return box;
}

function linkLine(anchor, url, source) {
  const line = el('li', 'link');
  line.appendChild(el('span', 'anchor', anchor));
  const src = el('span', 'src');
  if (source) src.appendChild(el('span', 'tag', source));
  if (/^http:/i.test(url)) src.appendChild(chip('warning', 'http'));
  line.appendChild(src);
  line.appendChild(el('span', 'url', url));
  return line;
}

function attachmentLine(part) {
  const line = el('div', 'link');
  line.appendChild(el('span', 'anchor', part.filename || '(unnamed)'));
  const src = el('span', 'src');
  src.appendChild(el('span', 'tag', bytes(part.size)));
  line.appendChild(src);
  let detail = part.content_type || 'unknown type';
  if (part.ics) {
    // The calendar's own spelling, with its TZID beside it; nothing resolved.
    const when = (value, tzid) => (value ? value + (tzid ? ' (' + tzid + ')' : '') : '—');
    detail =
      'Invite: ' + (part.ics.summary || '(no summary)') + ' · starts ' + when(part.ics.dtstart, part.ics.dtstart_tzid) + ' · ends ' + when(part.ics.dtend, part.ics.dtend_tzid);
  }
  line.appendChild(el('span', 'url plain', detail));
  return line;
}

function extractedPanel(email, projection) {
  const box = panel('Extracted at ingest');
  const body = el('div', 'panel-body');
  const ready = projection && projection.state === 'ok' ? projection.body : null;
  const codes = email.codes || [];

  if (codes.length) {
    const row = el('div', 'codes');
    for (const code of codes) {
      const boxed = el('div', 'codebox');
      boxed.appendChild(el('span', null, code));
      boxed.appendChild(copyControl(code, 'code'));
      row.appendChild(boxed);
    }
    body.appendChild(row);
  }

  const list = el('ul', 'links');
  const links = ready ? ready.links : null;
  const bare = Array.isArray(email.links) ? email.links : [];
  const bareShown = Math.min(bare.length, LIST_CAP);
  if (links) {
    for (const item of links.items) list.appendChild(linkLine(item.anchor_text || '(no anchor text)', item.url, item.source));
  } else if (ready) {
    // The projection is settled and its occurrences are unknown: the bare
    // URLs the mail carried, capped like every list on this page.
    for (let i = 0; i < bareShown; i++) list.appendChild(linkLine('(anchor text unknown)', bare[i], null));
  }
  if (list.childNodes.length) body.appendChild(list);
  if (links && links.total > links.items.length) {
    body.appendChild(el('div', 'note', 'Links: showing ' + links.items.length + ' of ' + links.total));
  }
  if (ready && !links && list.childNodes.length) {
    body.appendChild(el('div', 'note', 'Anchor text is unknown for this mail: its link occurrences were not stored.'));
    if (bare.length > bareShown) body.appendChild(el('div', 'note', 'Links: showing ' + bareShown + ' of ' + bare.length));
  }
  if (!ready && bare.length) {
    body.appendChild(el('div', 'note', failed(projection) ? 'Links unavailable — retrying' : 'Links loading…'));
  }

  const attachments = ready ? ready.attachments : null;
  if (attachments) {
    for (const part of attachments.items) body.appendChild(attachmentLine(part));
    if (attachments.total > attachments.items.length) {
      body.appendChild(el('div', 'note', 'Attachments: showing ' + attachments.items.length + ' of ' + attachments.total));
    }
  } else if (ready) {
    body.appendChild(el('div', 'note', 'The attachment list is unknown for this mail.'));
  } else {
    body.appendChild(el('div', 'note', failed(projection) ? 'Attachments unavailable — retrying' : 'Attachments loading…'));
  }

  if (!codes.length && !list.childNodes.length && attachments && attachments.total === 0) {
    body.appendChild(el('div', 'note', 'No codes, links or attachments in this mail.'));
  }
  box.appendChild(body);
  return box;
}

function mechanismChip(findings, name) {
  let worst = null;
  for (const f of findings) {
    if (typeof f.rule_id === 'string' && f.rule_id.startsWith(name + '.') && (!worst || rank(f.severity) > rank(worst.severity))) worst = f;
  }
  if (!worst) return el('span', 'sev', name + ': no finding');
  const short = worst.rule_id.slice(name.length + 1).replace(/\.v\d+$/, '');
  return chip(worst.severity, name + ': ' + short);
}

// ARC has a typed chain state in the report (decision 0133 section 2), so
// its chip reads that member, not the findings: pass is a positive result.
function arcChip(arc) {
  const state = arc && typeof arc.state === 'string' ? arc.state : 'not_evaluated';
  let text = 'arc: ' + (state === 'not_evaluated' ? 'not evaluated' : state);
  if (state === 'fail' && Number.isInteger(arc.failing_instance)) text += ' (i=' + arc.failing_instance + ')';
  if (state === 'pass') return chip('clean', text);
  if (state === 'fail') return chip('error', text);
  if (state === 'none') return el('span', 'sev', text);
  return chip('info', text);
}

// The authentication row comes from the query-time report, not from ingest,
// and the report has no positive result: no finding is not a verdict.
function authRow(report) {
  const box = el('div', 'panel');
  const row = el('div', 'auth-row');
  row.appendChild(el('span', 'label', 'Authentication (query-time report)'));
  const auth = el('span', 'auth');
  if (!report) {
    auth.appendChild(el('span', 'sev', 'loading…'));
  } else if (report.state === 'failed') {
    auth.appendChild(chip('warning', 'Deliverability unavailable — retrying'));
  } else if (report.state === 'ok') {
    const findings = Array.isArray(report.body.findings) ? report.body.findings : [];
    for (const name of ['dkim', 'spf', 'dmarc']) auth.appendChild(mechanismChip(findings, name));
    auth.appendChild(arcChip(report.body.arc));
  }
  row.appendChild(auth);
  box.appendChild(row);
  return box;
}

function metaRow(list, key, value) {
  list.appendChild(el('dt', null, key));
  list.appendChild(el('dd', null, value));
}

function paintTop() {
  const email = mailById.get(selectedId);
  const projection = stateOf(selectedId, 'projection');
  const report = stateOf(selectedId, 'report');
  const nodes = [];

  const actions = el('div', 'note');
  const standalone = el('a', null, 'Open standalone');
  standalone.href = '/ui/message/' + encodeURIComponent(selectedId);
  actions.appendChild(standalone);
  nodes.push(actions);

  nodes.push(el('h1', 'subject', email.subject || '(no subject)'));
  const meta = el('dl', 'meta');
  metaRow(meta, 'From', address(email.from));
  metaRow(meta, 'To', (email.to || []).map(address).join(', ') || '—');
  const sender = email.envelope_from;
  metaRow(meta, 'Envelope', sender === null || sender === undefined ? '—' : sender === '' ? '(null sender)' : sender);
  metaRow(meta, 'Received', email.received_at + ' · ' + bytes(email.size));
  let client = '…';
  if (projection && projection.state === 'ok') {
    client = (projection.body.smtp_helo || '—') + ' (' + (projection.body.smtp_client_ip || '—') + ')';
  } else if (failed(projection)) {
    client = 'Details unavailable — retrying';
  }
  metaRow(meta, 'Client', client);
  nodes.push(meta);
  nodes.push(extractedPanel(email, projection));
  nodes.push(authRow(report));
  topZone.replaceChildren(...nodes);
}

function note(text) {
  return el('div', 'note', text);
}

function previewTab(email) {
  if (!email.html) return [note('No HTML part.')];
  // The bare sandbox: set before the frame has any content and before it is
  // attached, with no token added. Scripts, forms, popups and the page's own
  // origin stay off.
  const frame = document.createElement('iframe');
  frame.className = 'preview';
  frame.setAttribute('sandbox', '');
  frame.setAttribute('title', 'HTML preview, scripts disabled');
  frame.srcdoc = email.html;
  return [frame, note('Rendered in a sandbox with scripts off.')];
}

function textTab(email) {
  return [email.text ? el('pre', 'text', email.text) : note('No text part.')];
}

function deliverabilityTab(report) {
  const nodes = [];
  if (!report) {
    nodes.push(note('Loading the report…'));
  } else if (report.state === 'failed') {
    nodes.push(notice('Deliverability unavailable — retrying', true));
  } else if (report.state === 'ok') {
    const findings = Array.isArray(report.body.findings) ? report.body.findings : [];
    nodes.push(note('Rule set ' + report.body.rule_set_id + ' · weight ' + report.body.total + '. Computed from the stored bytes when asked for (query time), never on the SMTP path.'));
    if (!findings.length) nodes.push(chip('clean', 'No findings from this rule set.'));
    const list = el('ul', 'findings');
    for (const f of findings.slice().sort((a, b) => rank(b.severity) - rank(a.severity))) {
      const item = el('li', 'finding');
      item.appendChild(chip(f.severity, f.severity));
      item.appendChild(el('span', 'rule', f.rule_id));
      item.appendChild(el('span', 'loc', locationText(f.location)));
      item.appendChild(el('span', 'hint', f.fix_hint));
      list.appendChild(item);
    }
    nodes.push(list);
  }
  const foot = note('No findings does not mean the mail will be delivered. These are demo and bulk-sender checks only.');
  foot.classList.add('gap');
  nodes.push(foot);
  return nodes;
}

function snippet(title, text) {
  const box = el('div');
  box.appendChild(el('div', 'snip-title', title));
  const snip = el('div', 'snip');
  snip.appendChild(el('pre', null, text));
  snip.appendChild(copyControl(text, 'snippet'));
  box.appendChild(snip);
  return box;
}

function testsTab(email, projection) {
  const base = location.origin + '/api/v1/inboxes/' + shellSegment(inbox);
  const held = inboxCounts.has(inbox) ? inboxCounts.get(inbox) : heldCount();
  const args = { inbox: inbox, timeout_ms: 10000 };
  let tool = 'swarmail_wait_for_email';
  let title = 'Agent: wait for mail (MCP)';
  if (email.codes && email.codes.length) {
    tool = 'swarmail_wait_for_code';
    title = 'Agent: wait for the code (MCP)';
  } else if (email.links && email.links.length) {
    tool = 'swarmail_wait_for_link';
    title = 'Agent: wait for the link (MCP)';
    const items = projection && projection.state === 'ok' && projection.body.links ? projection.body.links.items : [];
    const labelled = items.find((item) => typeof item.anchor_text === 'string' && item.anchor_text && item.anchor_text.length <= 200);
    // The selector is a regular expression: quote the label.
    if (labelled) args.anchor_text = escapeRegex(labelled.anchor_text);
  }
  const wrap = el('div', 'panel-body');
  wrap.appendChild(snippet('Wait for the next mail (REST)', "curl -s '" + base + "/await?timeout_ms=10000'"));
  wrap.appendChild(snippet('Assert exactly ' + plural(held, 'mail', 'mails') + ' (REST)', "curl -s '" + base + '/assert?count=' + held + "'"));
  wrap.appendChild(snippet(title, JSON.stringify({ tool: tool, arguments: args }, null, 2)));
  return [wrap];
}

// key, element id, label: the ids are fixed names, never mail data.
const TABS = [
  ['preview', 'tab-preview', 'Preview'],
  ['text', 'tab-text', 'Text'],
  ['deliverability', 'tab-deliverability', 'Deliverability'],
  ['tests', 'tab-tests', 'For your test'],
];

function paintTabBody() {
  const email = mailById.get(selectedId);
  let nodes;
  if (activeTab === 'preview') nodes = previewTab(email);
  else if (activeTab === 'text') nodes = textTab(email);
  else if (activeTab === 'deliverability') nodes = deliverabilityTab(stateOf(selectedId, 'report'));
  else nodes = testsTab(email, stateOf(selectedId, 'projection'));
  tabBody.replaceChildren(...nodes);
  for (const tab of tabStrip.childNodes) tab.setAttribute('aria-selected', String(tab.dataset.key === activeTab));
}

function paintTabLabels() {
  const report = stateOf(selectedId, 'report');
  const n = report && report.state === 'ok' && Array.isArray(report.body.findings) ? report.body.findings.length : 0;
  $('tab-deliverability').textContent = 'Deliverability' + (n ? ' (' + n + ')' : '');
}

function paintReader() {
  const body = $('reader-body');
  body.replaceChildren();
  topZone = null;
  readerMissing = selectedId !== null && !mailById.has(selectedId);
  if (selectedId === null) {
    body.appendChild(notice('Pick a mail to read it.'));
    return;
  }
  const projection = stateOf(selectedId, 'projection');
  if (readerMissing || (projection && projection.state === 'gone')) {
    body.appendChild(notice('Message not found (pruned or cleared).', true));
    return;
  }
  topZone = el('div', 'reader-top');
  body.appendChild(topZone);
  const box = el('div', 'panel');
  tabStrip = el('div', 'tabs');
  tabStrip.setAttribute('role', 'tablist');
  for (const [key, id, label] of TABS) {
    const tab = button('tab', label, () => {
      activeTab = key;
      paintTabBody();
    });
    tab.id = id;
    tab.dataset.key = key;
    tab.setAttribute('role', 'tab');
    tabStrip.appendChild(tab);
  }
  box.appendChild(tabStrip);
  tabBody = el('div', 'tabpanel');
  tabBody.setAttribute('role', 'tabpanel');
  box.appendChild(tabBody);
  body.appendChild(box);
  paintTop();
  paintTabLabels();
  paintTabBody();
}

// New enrichment for the open mail: repaint what depends on it. The preview
// and text tabs do not, so their frame is not reloaded.
function refreshReader() {
  const projection = stateOf(selectedId, 'projection');
  if (!topZone || (projection && projection.state === 'gone')) {
    paintReader();
    return;
  }
  paintTop();
  paintTabLabels();
  if (activeTab === 'deliverability' || activeTab === 'tests') paintTabBody();
}

$('back').addEventListener('click', () => {
  panes.dataset.pane = 'stream';
});
$('back-rail').addEventListener('click', () => {
  panes.dataset.pane = 'inboxes';
});

// ---------- boot ----------

async function tick() {
  if (document.hidden || ticking) return;
  ticking = true;
  tickCount++;
  try {
    await Promise.all([readHealth(), readChaos(), readInboxes()]);
    if (inbox && streamFailed) load();
    requeueFailures();
  } finally {
    ticking = false;
  }
}

paintReader();
if (inbox) {
  load();
  // Live: the feed pushes the full thread view on every accepted mail, so the
  // page never polls the stream and never needs a manual refresh.
  const feed = new EventSource(api('feed'));
  feed.onopen = () => setLive('live');
  feed.onerror = () => setLive(feed.readyState === EventSource.CLOSED ? 'off' : 'retry');
  feed.addEventListener('threads', (event) => {
    paintSeq++;
    const threads = JSON.parse(event.data);
    paint(threadKey ? threads.filter((thread) => thread.key === threadKey) : threads);
  });
} else {
  setLive('idle');
  view.appendChild(notice('Pick an inbox to watch its mail.'));
}
tick();
setInterval(tick, TICK_MS);
