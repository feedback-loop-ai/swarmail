// Swarmail live views: declarative over the JSON API — the server decides
// what a thread is, the browser only paints it. Mail-controlled text is
// rendered with textContent only; it never reaches innerHTML.
const view = document.getElementById('view');
const inbox = view.dataset.inbox;
const threadKey = view.dataset.thread || null;

// A node with a class and text. Everything mail-controlled goes through
// `text` so the browser can never parse it as markup.
function el(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

function link(href, child) {
  const a = el('a');
  a.href = href;
  a.appendChild(child);
  return a;
}

function api(path) {
  return '/api/v1/inboxes/' + encodeURIComponent(inbox) + '/' + path;
}

// One mail: subject → the message view, plus from/time/size.
function messageRow(email) {
  const card = el('div', 'card');
  card.appendChild(
    link('/ui/message/' + encodeURIComponent(email.id), el('b', null, email.subject || '(no subject)'))
  );
  card.appendChild(
    el('span', 'muted', ' → ' + (email.from ? email.from.address : '?') + ' · ' + email.received_at + ' · ' + email.size + ' B')
  );
  return card;
}

// A thread card: subject and count → the thread view, conversation oldest first.
function threadCard(thread) {
  const card = el('div', 'card');
  card.appendChild(
    link(
      '/ui/inbox/' + encodeURIComponent(inbox) + '/thread/' + encodeURIComponent(thread.key),
      el('b', null, (thread.subject || '(no subject)') + ' (' + thread.count + ')')
    )
  );
  for (const email of thread.emails) card.appendChild(messageRow(email));
  return card;
}

// Paint a whole view (thread list, or a single thread's conversation).
function paint(threads) {
  view.replaceChildren();
  view.appendChild(
    el('h2', null, threadKey ? (threads[0] ? threads[0].subject || '(no subject)' : 'thread') : 'inbox: ' + inbox)
  );
  if (!threads.length) {
    view.appendChild(
      el('div', 'card muted', 'Empty — live. New mail appears on its own; no manual refresh.')
    );
    return;
  }
  if (threadKey) {
    for (const email of threads[0].emails) view.appendChild(messageRow(email));
  } else {
    for (const thread of threads) view.appendChild(threadCard(thread));
  }
}

async function load() {
  const response = await fetch(threadKey ? api('threads/' + encodeURIComponent(threadKey)) : api('threads'));
  const body = await response.json();
  if (!response.ok) {
    view.replaceChildren(el('div', 'card muted', body.error || 'not found'));
    return;
  }
  paint(Array.isArray(body) ? body : [body]);
}

load();

// Live: the feed pushes the full thread view on every accepted mail, so the
// page never polls and never needs a manual refresh.
const feed = new EventSource(api('feed'));
feed.addEventListener('threads', (event) => {
  const threads = JSON.parse(event.data);
  paint(threadKey ? threads.filter((thread) => thread.key === threadKey) : threads);
});
