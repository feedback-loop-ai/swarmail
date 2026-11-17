# Browser proof for the preview (T8): recorded 2026-11-17

[Tasks](tasks.md) · [0135](../../docs/decisions/0135-preview-images.md)

Run on the dispatcher host with Playwright, headless Chromium and Firefox,
local origins only, by `swarmail-landing/browser-proof-374/proof.mjs` against
`a8af5be`. Every expectation passed except step 6's literal "no Referer" for
the font a remote stylesheet requests, which the operator ruled met under its
intent (0135, clarification of 2026-11-17). The recorded result follows the
procedure.

## Procedure

1. Start the candidate: `swarmail serve --http-listen 127.0.0.1:8025
   --smtp-listen 127.0.0.1:1025`.
2. Start a second origin that logs every request, for example
   `python3 -m http.server 9000 --bind 127.0.0.1` in a directory that holds
   `p.png`, `s.css` (with `body{background:#0f0}` and an `@font-face` whose
   `src` is `url(http://127.0.0.1:9000/f.woff)`) and `f.woff`.
3. Send, over SMTP, a `multipart/related` mail whose HTML has a large
   solid-colour PNG at `<img src="cid:logo">` with a matching inline part,
   plus `<img src=http://127.0.0.1:9000/p.png>`,
   `<link rel=stylesheet href=http://127.0.0.1:9000/s.css>`, a
   `<meta http-equiv=refresh content="1;url=http://127.0.0.1:9000/refresh">`,
   `<link rel=dns-prefetch href=//127.0.0.1:9000>`,
   `<link rel=preconnect href=http://127.0.0.1:9000>`,
   `<script>parent.document.title='pwned'</script>`,
   `<form action=http://127.0.0.1:9000/f><input autofocus><button>go</button></form>`,
   `<a href=http://127.0.0.1:9000/click target=_top>click</a>`,
   `<iframe src=http://127.0.0.1:9000/frame></iframe>` and an `<svg>` that
   holds `<a href=http://127.0.0.1:9000/svg><text>svg link</text></a>`.
4. Open `http://127.0.0.1:8025/ui/inbox/<inbox>` and select the mail. Expect
   the CID pixels to be visible, **zero** requests to :9000, the title
   unchanged, and the warning beside **Load remote content**.
5. Click every link and the button in the frame. Expect no navigation, no
   form submission and no requests.
6. Click **Load remote content**. Expect requests for `p.png`, `s.css` and
   `f.woff` only, with no `Referer` header. There must be no `/refresh`,
   `/click`, `/frame`, `/f` or `/svg` request, and the button must be
   disabled.
7. Switch to the Text tab and back. Expect no reload, so no new :9000
   requests. Select another mail, then the first one again: remote content
   is off again and the button is enabled.
8. Open `/ui/message/<id>/preview` directly. DevTools must show an opaque
   origin, scripts blocked and no request to :9000. Frame the preview URL
   from a page served on :9000. Expect the browser to refuse the frame
   (`frame-ancestors 'self'`, `SAMEORIGIN`), and the console URL to refuse
   framing too (`DENY`).

## Recorded result


**Overall: every step passed; step 6's font Referer ruled met (see 0135 clarification)** — candidate `feat/ui-preview-images-374` @ `a8af5be`, run 2026-11-17T20:49:23.104Z.

Procedure: `specs/preview-images/browser-proof.md` steps 1–8, automated by `proof.mjs` (Playwright 1.64.1, headless). Swarmail on `127.0.0.1:18025` (HTTP) / `127.0.0.1:11025` (SMTP) / `127.0.0.1:11110` (POP3); logging second origin on `127.0.0.1:19000` (Node, logs every request and TCP connection). The fixture used port 19000 where the spec says 9000.

| Step | chromium 156.0.8078.4 | firefox 157.0 |
|---|---|---|
| Step 4 — open, CID visible, nothing remote | PASS (8/8) | PASS (8/8) |
| Step 5 — click every link/button | PASS (5/5) | PASS (5/5) |
| Step 6 — Load remote content | FAIL (5/6) | FAIL (5/6) |
| Step 7 — tabs and reselection | PASS (7/7) | PASS (7/7) |
| Step 8 — direct open and cross-origin framing | PASS (12/12) | PASS (12/12) |

### Finding — step 6 Referer expectation not met literally

The spec expects no `Referer` on any remote request after **Load remote content**. The image (`p.png`) and stylesheet (`s.css`) requests, which the preview document itself issues, carry none in either browser. The font (`f.woff`) request, which the remote stylesheet issues, carries a `Referer` naming the remote host itself:

- chromium 156.0.8078.4: `GET /f.woff` Referer `http://127.0.0.1:19000/s.css`
- firefox 157.0: `GET /f.woff` Referer `http://127.0.0.1:19000/`

The value is the stylesheet URL (Chromium) or its origin (Firefox): never a Swarmail URL, origin or message id. Requests made from a stylesheet take the stylesheet as their referrer and the stylesheet response's own `Referrer-Policy`, not the document's; the preview's `Referrer-Policy: no-referrer` header therefore cannot reach them. The control below confirms it: when the remote host serves `s.css` with `Referrer-Policy: no-referrer`, the font request carries no `Referer` in either browser. This is recorded as a failed check, for the dispatcher to rule on: either the expectation is read as "no Referer that reveals the Swarmail origin or the message" (met), or remote fonts loaded from remote stylesheets cannot meet it.

### Preview route response headers (fetched by Node)

| Request | Status | Content-Security-Policy | X-Frame-Options | X-Content-Type-Options | Referrer-Policy | Cache-Control |
|---|---|---|---|---|---|---|
| GET /ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview | 200 | `default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'none'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'self'; sandbox` | SAMEORIGIN | nosniff | no-referrer | no-store |
| GET /ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview?remote=1 | 200 | `default-src 'none'; img-src data: https: http:; style-src 'unsafe-inline' https: http:; font-src https: http:; script-src 'none'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'self'; sandbox` | SAMEORIGIN | nosniff | no-referrer | no-store |
| HEAD /ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview | 200 | `default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'none'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'self'; sandbox` | SAMEORIGIN | nosniff | no-referrer | no-store |
| GET /ui/inbox/default | 200 | `—` | DENY | — | — | — |

### Fixture

Sent over SMTP to `viewer@proof.test` (inbox `default`): a `multipart/related` mail (id `01a1eb44-571b-75a7-ad7f-c81885b89bd4`) whose HTML part holds every hostile element of step 3, and a 480×160 solid magenta PNG (646 bytes) at `Content-ID: <logo>`, `Content-Disposition: inline`; plus a plain mail (id `01a1eb44-571d-7295-ba6d-c8dfb1c0e7e7`) for step 7. HTML part:

```html
<html><head>
<meta http-equiv=refresh content="1;url=http://127.0.0.1:19000/refresh">
<link rel=dns-prefetch href=//127.0.0.1:19000>
<link rel=preconnect href=http://127.0.0.1:19000>
<link rel=stylesheet href=http://127.0.0.1:19000/s.css>
<title>fixture</title>
</head><body>
<p id=marker>Swarmail browser proof fixture 374</p>
<img id=cid src="cid:logo" alt="cid logo">
<img id=remote src=http://127.0.0.1:19000/p.png alt=remote>
<script>parent.document.title='pwned'</script>
<form action=http://127.0.0.1:19000/f><input autofocus name=q value=x><button>go</button></form>
<a href=http://127.0.0.1:19000/click target=_top>click</a>
<iframe src=http://127.0.0.1:19000/frame></iframe>
<svg width=200 height=40><a href=http://127.0.0.1:19000/svg><text x=5 y=20>svg link</text></a></svg>
</body></html>
```

Second origin serves `p.png` (64×64 blue), `s.css` (`@font-face{font-family:proof;src:url(http://127.0.0.1:19000/f.woff) format("woff")} body{background:#0f0;font-family:proof,sans-serif}`) and `f.woff`.

### chromium 156.0.8078.4 (Playwright 1.64.1)

**Step 4 — open, CID visible, nothing remote**

- PASS — CID image decoded (img#cid naturalWidth > 0, data: src) — `{"src":"data:image/png;base64,iVBORw0K","complete":true,"naturalWidth":480,"naturalHeight":160}`
- PASS — CID pixels visible in frame screenshot (magenta pixels) — `{"matching":76800,"total":242592,"width":672,"height":361}`
- PASS — zero HTTP requests to the second origin — `[]`
- PASS — zero TCP connections to the second origin (no preconnect/prefetch) — `0`
- PASS — parent title unchanged (not pwned) — `{"titleBefore":"default · Swarmail","titleAfter":"default · Swarmail"}`
- PASS — frame document title not pwned — `fixture`
- PASS — warning beside Load remote content — `Load remote content Loading contacts the sender's hosts. Your browser fetches the images, stylesheets and fonts itself: `
- PASS — frame is the fixture (marker present) — `Swarmail browser proof fixture 374`

**Step 5 — click every link/button**

- PASS — every element clicked — `[{"label":"a \"click\" (target=_top)","result":"clicked"},{"label":"svg link","result":"clicked"},{"label":"button \"go\"","result":"clicked"},{"label":"Enter in form input","result":"pressed Enter in input"}]`
- PASS — top URL unchanged — `http://127.0.0.1:18025/ui/inbox/default`
- PASS — frame URL unchanged (no navigation, no form submission) — `http://127.0.0.1:18025/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview`
- PASS — no popup opened — `0`
- PASS — zero new requests to the second origin — `[]`

**Step 6 — Load remote content**

- PASS — requests only for p.png, s.css, f.woff — `["/s.css","/p.png","/f.woff"]`
- PASS — each of p.png, s.css, f.woff requested — `["/s.css","/p.png","/f.woff"]`
- **FAIL** — no request carries a Referer header — `[{"path":"/s.css","referer":null},{"path":"/p.png","referer":null},{"path":"/f.woff","referer":"http://127.0.0.1:19000/s.css"}]`
- PASS — no /refresh /click /frame /f /svg request — `["/s.css","/p.png","/f.woff"]`
- PASS — button disabled afterwards
- PASS — remote content applied (body bg rgb(0,255,0), p.png decoded) — `{"bg":"rgb(0, 255, 0)","remoteImg":{"naturalWidth":64},"bluePixels":4096}`

**Step 7 — tabs and reselection**

- PASS — Text and back: zero new requests — `[]`
- PASS — Text and back: same frame element kept (no reload) — `{"sameFrame":true,"hiddenOnText":true}`
- PASS — Text and back: button still disabled
- PASS — reselected: frame src has no remote=1 — `/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview`
- PASS — reselected: Load remote content enabled
- PASS — reselected: zero requests to the second origin — `[]`
- PASS — reselected: CID image still renders — `480`

**Step 8 — direct open and cross-origin framing**

- PASS — direct open: document is the fixture — `Swarmail browser proof fixture 374`
- PASS — direct open: inline script did not run (title not pwned) — `{"title":"fixture","scriptElements":1}`
- PASS — direct open: opaque origin (self.origin "null") — `{"origin":{"selfOrigin":"null","locationOrigin":"http://127.0.0.1:18025"},"cookie":"threw SecurityError: Failed to read the 'cookie' property from 'Document': The document is sandboxed and lacks the 'allow-same-origin' flag."}`
- PASS — direct open: CSP sandbox header present — `default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'none'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'self'; sandbox`
- PASS — direct open: zero requests to the second origin — `[]`
- PASS — framed preview from :19000: refused (Swarmail content not rendered) — `{"childUrl":"chrome-error://chromewebdata/","childText":"","console":[{"type":"error","text":"Framing 'http://127.0.0.1:18025/' violates the following Content Security Policy directive: \"frame-ancestors 'self'\". The request has been blocked.\n"}]}`
- PASS — framed preview: X-Frame-Options SAMEORIGIN — `{"status":200,"xfo":"SAMEORIGIN","csp":"default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'none'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'self'; sandbox"}`
- PASS — framed preview: CSP frame-ancestors 'self' — `default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'none'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'self'; sandbox`
- PASS — framed preview: zero non-harness requests to the second origin — `[]`
- PASS — framed console from :19000: refused (Swarmail content not rendered) — `{"childUrl":"chrome-error://chromewebdata/","childText":"","console":[{"type":"error","text":"Refused to display 'http://127.0.0.1:18025/' in a frame because it set 'X-Frame-Options' to 'deny'."}]}`
- PASS — framed console: X-Frame-Options DENY — `{"status":200,"xfo":"DENY","csp":null}`
- PASS — framed console: zero non-harness requests to the second origin — `[]`

Preview frame navigations seen by the console page (response headers):

- 200 `/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview` — CSP `default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'none'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'self'; sandbox`, XFO `SAMEORIGIN`, nosniff `nosniff`, Referrer-Policy `no-referrer`
- 200 `/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview?remote=1` — CSP `default-src 'none'; img-src data: https: http:; style-src 'unsafe-inline' https: http:; font-src https: http:; script-src 'none'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'self'; sandbox`, XFO `SAMEORIGIN`, nosniff `nosniff`, Referrer-Policy `no-referrer`
- 200 `/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview` — CSP `default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'none'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'self'; sandbox`, XFO `SAMEORIGIN`, nosniff `nosniff`, Referrer-Policy `no-referrer`

Framing from `http://127.0.0.1:19000`:

- preview: child frame URL `chrome-error://chromewebdata/`, child text ``, response `{"status":200,"xfo":"SAMEORIGIN","csp":"default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'none'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'self'; sandbox"}`
  - console error: `Framing 'http://127.0.0.1:18025/' violates the following Content Security Policy directive: "frame-ancestors 'self'". The request has been blocked. `
- console: child frame URL `chrome-error://chromewebdata/`, child text ``, response `{"status":200,"xfo":"DENY","csp":null}`
  - console error: `Refused to display 'http://127.0.0.1:18025/' in a frame because it set 'X-Frame-Options' to 'deny'.`

Console messages (console page and direct open):

- [console] error: `Loading the stylesheet 'http://127.0.0.1:19000/s.css' violates the following Content Security Policy directive: "style-src 'unsafe-inline'". Note that 'style-src-elem' was not explicitly set, so 'style-src' is used as a fallback. The action has been blocked.`
- [console] error: `Loading the image 'http://127.0.0.1:19000/p.png' violates the following Content Security Policy directive: "img-src data:". The action has been blocked.`
- [console] error: `Blocked script execution in 'http://127.0.0.1:18025/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview' because the document's frame is sandboxed and the 'allow-scripts' permission is not set.`
- [console] error: `Blocked autofocusing on a <input> element because the element's frame is sandboxed and the 'allow-scripts' permission is not set.`
- [console] error: `Framing 'http://127.0.0.1:19000/' violates the following Content Security Policy directive: "frame-src 'none'". The request has been blocked. `
- [console] error: `Blocked form submission to 'http://127.0.0.1:19000/f' because the form's frame is sandboxed and the 'allow-forms' permission is not set.`
- [console] error: `Blocked form submission to 'http://127.0.0.1:19000/f' because the form's frame is sandboxed and the 'allow-forms' permission is not set.`
- [console] error: `Blocked script execution in 'http://127.0.0.1:18025/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview?remote=1' because the document's frame is sandboxed and the 'allow-scripts' permission is not set.`
- [console] error: `Blocked autofocusing on a <input> element because the element's frame is sandboxed and the 'allow-scripts' permission is not set.`
- [console] error: `Framing 'http://127.0.0.1:19000/' violates the following Content Security Policy directive: "frame-src 'none'". The request has been blocked. `
- [console] warning: `Failed to decode downloaded font: http://127.0.0.1:19000/f.woff`
- [console] warning: `OTS parsing error: invalid sfntVersion: 0`
- [console] error: `Loading the stylesheet 'http://127.0.0.1:19000/s.css' violates the following Content Security Policy directive: "style-src 'unsafe-inline'". Note that 'style-src-elem' was not explicitly set, so 'style-src' is used as a fallback. The action has been blocked.`
- [console] error: `Loading the image 'http://127.0.0.1:19000/p.png' violates the following Content Security Policy directive: "img-src data:". The action has been blocked.`
- [console] error: `Blocked script execution in 'http://127.0.0.1:18025/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview' because the document's frame is sandboxed and the 'allow-scripts' permission is not set.`
- [console] error: `Blocked autofocusing on a <input> element because the element's frame is sandboxed and the 'allow-scripts' permission is not set.`
- [console] error: `Framing 'http://127.0.0.1:19000/' violates the following Content Security Policy directive: "frame-src 'none'". The request has been blocked. `
- [direct] error: `Loading the stylesheet 'http://127.0.0.1:19000/s.css' violates the following Content Security Policy directive: "style-src 'unsafe-inline'". Note that 'style-src-elem' was not explicitly set, so 'style-src' is used as a fallback. The action has been blocked.`
- [direct] error: `Loading the image 'http://127.0.0.1:19000/p.png' violates the following Content Security Policy directive: "img-src data:". The action has been blocked.`
- [direct] error: `Blocked script execution in 'http://127.0.0.1:18025/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview' because the document's frame is sandboxed and the 'allow-scripts' permission is not set.`
- [direct] error: `Blocked autofocusing on a <input> element because the element's frame is sandboxed and the 'allow-scripts' permission is not set.`
- [direct] error: `Framing 'http://127.0.0.1:19000/' violates the following Content Security Policy directive: "frame-src 'none'". The request has been blocked. `

Second-origin request log for this browser (5 requests, 5 TCP connections; `/host-*.html` are the step-8 harness pages loaded by the test itself, and a `/favicon.ico` whose Referer is such a host page is the browser's favicon fetch for that top-level harness page; both are excluded from the zero-request assertions):

| # | Method | Path | Referer | Origin |
|---|---|---|---|---|
| 3 | GET | `/s.css` | — | — |
| 4 | GET | `/p.png` | — | — |
| 6 | GET | `/f.woff` | `http://127.0.0.1:19000/s.css` | `null` |
| 8 | GET | `/host-preview.html` | — | — |
| 10 | GET | `/host-console.html` | — | — |

### firefox 157.0 (Playwright 1.64.1)

**Step 4 — open, CID visible, nothing remote**

- PASS — CID image decoded (img#cid naturalWidth > 0, data: src) — `{"src":"data:image/png;base64,iVBORw0K","complete":true,"naturalWidth":480,"naturalHeight":160}`
- PASS — CID pixels visible in frame screenshot (magenta pixels) — `{"matching":76800,"total":242592,"width":672,"height":361}`
- PASS — zero HTTP requests to the second origin — `[]`
- PASS — zero TCP connections to the second origin (no preconnect/prefetch) — `0`
- PASS — parent title unchanged (not pwned) — `{"titleBefore":"default · Swarmail","titleAfter":"default · Swarmail"}`
- PASS — frame document title not pwned — `fixture`
- PASS — warning beside Load remote content — `Load remote content Loading contacts the sender's hosts. Your browser fetches the images, stylesheets and fonts itself: `
- PASS — frame is the fixture (marker present) — `Swarmail browser proof fixture 374`

**Step 5 — click every link/button**

- PASS — every element clicked — `[{"label":"a \"click\" (target=_top)","result":"clicked"},{"label":"svg link","result":"clicked"},{"label":"button \"go\"","result":"clicked"},{"label":"Enter in form input","result":"pressed Enter in input"}]`
- PASS — top URL unchanged — `http://127.0.0.1:18025/ui/inbox/default`
- PASS — frame URL unchanged (no navigation, no form submission) — `http://127.0.0.1:18025/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview`
- PASS — no popup opened — `0`
- PASS — zero new requests to the second origin — `[]`

**Step 6 — Load remote content**

- PASS — requests only for p.png, s.css, f.woff — `["/s.css","/p.png","/f.woff"]`
- PASS — each of p.png, s.css, f.woff requested — `["/s.css","/p.png","/f.woff"]`
- **FAIL** — no request carries a Referer header — `[{"path":"/s.css","referer":null},{"path":"/p.png","referer":null},{"path":"/f.woff","referer":"http://127.0.0.1:19000/"}]`
- PASS — no /refresh /click /frame /f /svg request — `["/s.css","/p.png","/f.woff"]`
- PASS — button disabled afterwards
- PASS — remote content applied (body bg rgb(0,255,0), p.png decoded) — `{"bg":"rgb(0, 255, 0)","remoteImg":{"naturalWidth":64},"bluePixels":4096}`

**Step 7 — tabs and reselection**

- PASS — Text and back: zero new requests — `[]`
- PASS — Text and back: same frame element kept (no reload) — `{"sameFrame":true,"hiddenOnText":true}`
- PASS — Text and back: button still disabled
- PASS — reselected: frame src has no remote=1 — `/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview`
- PASS — reselected: Load remote content enabled
- PASS — reselected: zero requests to the second origin — `[]`
- PASS — reselected: CID image still renders — `480`

**Step 8 — direct open and cross-origin framing**

- PASS — direct open: document is the fixture — `Swarmail browser proof fixture 374`
- PASS — direct open: inline script did not run (title not pwned) — `{"title":"fixture","scriptElements":1}`
- PASS — direct open: opaque origin (self.origin "null") — `{"origin":{"selfOrigin":"null","locationOrigin":"http://127.0.0.1:18025"},"cookie":"threw SecurityError: Document.cookie getter: Forbidden in a sandboxed document without the 'allow-same-origin' flag."}`
- PASS — direct open: CSP sandbox header present — `default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'none'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'self'; sandbox`
- PASS — direct open: zero requests to the second origin — `[]`
- PASS — framed preview from :19000: refused (Swarmail content not rendered) — `{"childUrl":"","childText":{"error":"not evaluable"},"console":[{"type":"error","text":"[JavaScript Error: \"Content-Security-Policy: The page’s settings blocked the loading of a resource (frame-ancestors) at <unknown> because it violates the following directive: “frame-ancestors 'self'”\" {file: \"chrome://juggler/content/NetworkObserver.js\" line: 419}]"}]}`
- PASS — framed preview: X-Frame-Options SAMEORIGIN — `{"status":200,"xfo":"SAMEORIGIN","csp":"default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'none'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'self'; sandbox"}`
- PASS — framed preview: CSP frame-ancestors 'self' — `default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'none'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'self'; sandbox`
- PASS — framed preview: zero non-harness requests to the second origin — `[]`
- PASS — framed console from :19000: refused (Swarmail content not rendered) — `{"childUrl":"","childText":{"error":"not evaluable"},"console":[{"type":"error","text":"[JavaScript Error: \"The loading of “http://127.0.0.1:18025/ui/inbox/default” in a frame is denied by “X-Frame-Options“ directive set to “deny“.\" {file: \"http://127.0.0.1:18025/ui/inbox/default\" line: 0}]"}]}`
- PASS — framed console: X-Frame-Options DENY — `{"status":200,"xfo":"DENY","csp":null}`
- PASS — framed console: zero non-harness requests to the second origin — `[]`

Preview frame navigations seen by the console page (response headers):

- 200 `/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview` — CSP `default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'none'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'self'; sandbox`, XFO `SAMEORIGIN`, nosniff `nosniff`, Referrer-Policy `no-referrer`
- 200 `/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview?remote=1` — CSP `default-src 'none'; img-src data: https: http:; style-src 'unsafe-inline' https: http:; font-src https: http:; script-src 'none'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'self'; sandbox`, XFO `SAMEORIGIN`, nosniff `nosniff`, Referrer-Policy `no-referrer`
- 200 `/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview` — CSP `default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'none'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'self'; sandbox`, XFO `SAMEORIGIN`, nosniff `nosniff`, Referrer-Policy `no-referrer`

Framing from `http://127.0.0.1:19000`:

- preview: child frame URL ``, child text `{"error":"not evaluable"}`, response `{"status":200,"xfo":"SAMEORIGIN","csp":"default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'none'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'self'; sandbox"}`
  - console error: `[JavaScript Error: "Content-Security-Policy: The page’s settings blocked the loading of a resource (frame-ancestors) at <unknown> because it violates the following directive: “frame-ancestors 'self'”" {file: "chrome://juggler/content/NetworkObserver.js" line: 419}]`
- console: child frame URL ``, child text `{"error":"not evaluable"}`, response `{"status":200,"xfo":"DENY","csp":null}`
  - console error: `[JavaScript Error: "The loading of “http://127.0.0.1:18025/ui/inbox/default” in a frame is denied by “X-Frame-Options“ directive set to “deny“." {file: "http://127.0.0.1:18025/ui/inbox/default" line: 0}]`

Console messages (console page and direct open):

- [console] warning: `[JavaScript Warning: "Content-Security-Policy: Ignoring ‘x-frame-options’ because of ‘frame-ancestors’ directive."]`
- [console] warning: `[JavaScript Warning: "This page is in Quirks Mode. Page layout may be impacted. For Standards Mode use “<!DOCTYPE html>”." {file: "http://127.0.0.1:18025/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview" line: 0}]`
- [console] error: `[JavaScript Error: "Content-Security-Policy: The page’s settings blocked a style (style-src-elem) at http://127.0.0.1:19000/s.css from being applied because it violates the following directive: “style-src 'unsafe-inline'”" {file: "http://127.0.0.1:18025/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview" line: 0}]`
- [console] error: `[JavaScript Error: "Content-Security-Policy: The page’s settings blocked the loading of a resource (img-src) at http://127.0.0.1:19000/p.png because it violates the following directive: “img-src data:”" {file: "http://127.0.0.1:18025/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview" line: 0}]`
- [console] error: `[JavaScript Error: "Content-Security-Policy: The page’s settings blocked the loading of a resource (frame-src) at http://127.0.0.1:19000/frame because it violates the following directive: “frame-src 'none'”" {file: "http://127.0.0.1:18025/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview" line: 0}]`
- [console] warning: `[JavaScript Warning: "Content-Security-Policy: Ignoring ‘x-frame-options’ because of ‘frame-ancestors’ directive."]`
- [console] warning: `[JavaScript Warning: "This page is in Quirks Mode. Page layout may be impacted. For Standards Mode use “<!DOCTYPE html>”." {file: "http://127.0.0.1:18025/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview?remote=1" line: 0}]`
- [console] error: `[JavaScript Error: "Content-Security-Policy: The page’s settings blocked the loading of a resource (frame-src) at http://127.0.0.1:19000/frame because it violates the following directive: “frame-src 'none'”" {file: "http://127.0.0.1:18025/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview?remote=1" line: 0}]`
- [console] error: `[JavaScript Error: "downloadable font: invalid sfntVersion: 0 (font-family: "proof" style:normal weight:400 width:100 src index:0) source: http://127.0.0.1:19000/f.woff"]`
- [console] error: `[JavaScript Error: "downloadable font: rejected by sanitizer (font-family: "proof" style:normal weight:400 width:100 src index:0) source: http://127.0.0.1:19000/f.woff"]`
- [console] warning: `[JavaScript Warning: "Content-Security-Policy: Ignoring ‘x-frame-options’ because of ‘frame-ancestors’ directive."]`
- [console] warning: `[JavaScript Warning: "This page is in Quirks Mode. Page layout may be impacted. For Standards Mode use “<!DOCTYPE html>”." {file: "http://127.0.0.1:18025/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview" line: 0}]`
- [console] error: `[JavaScript Error: "Content-Security-Policy: The page’s settings blocked a style (style-src-elem) at http://127.0.0.1:19000/s.css from being applied because it violates the following directive: “style-src 'unsafe-inline'”" {file: "http://127.0.0.1:18025/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview" line: 0}]`
- [console] error: `[JavaScript Error: "Content-Security-Policy: The page’s settings blocked the loading of a resource (img-src) at http://127.0.0.1:19000/p.png because it violates the following directive: “img-src data:”" {file: "http://127.0.0.1:18025/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview" line: 0}]`
- [console] error: `[JavaScript Error: "Content-Security-Policy: The page’s settings blocked the loading of a resource (frame-src) at http://127.0.0.1:19000/frame because it violates the following directive: “frame-src 'none'”" {file: "http://127.0.0.1:18025/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview" line: 0}]`
- [direct] warning: `[JavaScript Warning: "This page is in Quirks Mode. Page layout may be impacted. For Standards Mode use “<!DOCTYPE html>”." {file: "http://127.0.0.1:18025/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview" line: 0}]`
- [direct] error: `[JavaScript Error: "Content-Security-Policy: The page’s settings blocked a style (style-src-elem) at http://127.0.0.1:19000/s.css from being applied because it violates the following directive: “style-src 'unsafe-inline'”" {file: "http://127.0.0.1:18025/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview" line: 0}]`
- [direct] error: `[JavaScript Error: "Content-Security-Policy: The page’s settings blocked the loading of a resource (img-src) at http://127.0.0.1:19000/p.png because it violates the following directive: “img-src data:”" {file: "http://127.0.0.1:18025/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview" line: 0}]`
- [direct] error: `[JavaScript Error: "Content-Security-Policy: The page’s settings blocked the loading of a resource (img-src) at http://127.0.0.1:18025/favicon.ico because it violates the following directive: “img-src data:”" {file: "resource:///modules/FaviconLoader.sys.mjs" line: 232}]`
- [direct] error: `[JavaScript Error: "Content-Security-Policy: The page’s settings blocked the loading of a resource (frame-src) at http://127.0.0.1:19000/frame because it violates the following directive: “frame-src 'none'”" {file: "http://127.0.0.1:18025/ui/message/01a1eb44-571b-75a7-ad7f-c81885b89bd4/preview" line: 0}]`

Second-origin request log for this browser (6 requests, 6 TCP connections; `/host-*.html` are the step-8 harness pages loaded by the test itself, and a `/favicon.ico` whose Referer is such a host page is the browser's favicon fetch for that top-level harness page; both are excluded from the zero-request assertions):

| # | Method | Path | Referer | Origin |
|---|---|---|---|---|
| 12 | GET | `/s.css` | — | `null` |
| 14 | GET | `/p.png` | — | `null` |
| 16 | GET | `/f.woff` | `http://127.0.0.1:19000/` | `null` |
| 18 | GET | `/host-preview.html` | — | — |
| 20 | GET | `/favicon.ico` | `http://127.0.0.1:19000/host-preview.html` | — |
| 22 | GET | `/host-console.html` | — | — |

### Control — who governs the font request's Referer

Same step-6 click in a fresh browser, but the second origin serves `s.css` with `Referrer-Policy: no-referrer` (the preview document and Swarmail are unchanged):

- chromium 156.0.8078.4: `GET /s.css` Referer —; `GET /p.png` Referer —; `GET /f.woff` Referer —
- firefox 157.0: `GET /s.css` Referer —; `GET /p.png` Referer —; `GET /f.woff` Referer —

Full request headers for every logged request are in `results.json` beside `proof.mjs`.

