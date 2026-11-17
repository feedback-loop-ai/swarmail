# Browser proof for the preview (T8): outstanding

[Tasks](tasks.md) · [Proposed 0135](../../docs/decisions/0135-preview-images.md)

No browser could run in the delivery box, so this evidence has **not** been
produced. Run it on a machine with a current Chromium and Firefox, and record
each browser's version, the fixture, and the request log of the second
origin. Use local origins only: no internet service is needed.

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
