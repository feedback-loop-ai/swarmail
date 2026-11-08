# MailSlurper release-1.15.0: cgo build from the pinned source ("built from source")

This covers ruling 1. The official Linux release binary cannot start on its default SQLite storage, so this directory supplies a cgo build of the same pinned source. That build is labelled **"built from source"**.

## Inputs

- **Source:** `../../2026-11-07-mailslurper-src/mailslurper-release-1.15.0-src.tar.gz`, sha256 `83f66f25ac000fc2f5680514719b7b75a29dafccb7e782597077325fb900359f`. The hash was re-checked before extraction. `git get-tar-commit-id` gives `068acffdcd8960f248ea438c3b7ee51ead601bd2`, the release-1.15.0 commit (see `../git-trees/`).
- **Fresh build directory:** the tarball was extracted into the new empty directory `/home/valentin/build-work-issue340-mscgo/src/`. It was under /home, not /tmp. The directory was deleted after the outputs were copied here.
- **Go version:** upstream's `Dockerfile` builds with `FROM golang:1.18.1-alpine3.15`. `go.mod` only says `go 1.13`, which is a language minimum and not a toolchain. Go **1.18.1** was used.
- **Builder image:** `docker.io/library/golang:1.18.1-bullseye@sha256:3b1a72af045ad0fff9fe8e00736baae76d70ff51325ac5bb814fe4754044b972`.
  - This is the multi-arch index digest. The linux/amd64 manifest is `sha256:5b75b529da0f2196ee8561a90e5b99aceee56e125c6ef09a3da4e32cf3cc6c20`, and the raw index is saved as `builder-image-index.json` (its sha256 equals the digest).
  - The image was created on 2022-04-20T22:27:57Z. It ships go1.18.1, gcc 10.2.1 and glibc 2.31 (Debian 11).
  - **No 1.18.1-bookworm tag exists:** `docker buildx imagetools inspect docker.io/library/golang:1.18.1-bookworm` returns "not found", because Go 1.18.1 predates bookworm. The glibc variant of the same Go version was used instead. Upstream's alpine3.15 builder would give a musl-dynamic binary, and the brief asked for static or glibc.
  - The image was pulled by digest on 2026-11-07 and removed again after the build.

## Build (one container, bounded)

```
docker run --rm --name issue340-mscgo-build --cpuset-cpus=0-31 --memory=8g --user 1000:1000 \
  -v /home/valentin/build-work-issue340-mscgo:/work -w /work \
  docker.io/library/golang:1.18.1-bullseye@sha256:3b1a72af045ad0fff9fe8e00736baae76d70ff51325ac5bb814fe4754044b972 \
  /work/build-in-container.sh
```

`scripts/build-in-container.sh` is the exact script that ran. In order, it:

1. Sets `HOME`, `GOMODCACHE` and `GOCACHE` to fresh directories under `/work`, and sets `CGO_ENABLED=1`.
2. Runs `go mod download -x`. This was the only step with network access to modules, through the default `GOPROXY=https://proxy.golang.org,direct` and `GOSUMDB=sum.golang.org`.
3. Runs `go mod verify`, which prints `all modules verified`.
4. Sets `GOPROXY=off GOFLAGS=-mod=readonly`, so nothing further can be fetched and go.mod/go.sum cannot change.
5. In `cmd/mailslurper`, runs **`go build -trimpath -o /work/out/mailslurper .`**

**Flags:** the build adds only `-trimpath`, with no `-ldflags` and no tags. This is upstream's Dockerfile build (`go build`) plus `-trimpath`. Upstream's `bin/package.sh`, which made the release zip, adds `-ldflags="-s -w"`. That was not used, so the binary keeps its symbol table.

**`go get` and `go generate`:** upstream's Dockerfile runs both. They were skipped:

- `go mod download` replaces `go get`, as the brief requires.
- `go generate` would `go install github.com/mjibson/esc@latest`, which is unpinned. Its output, `cmd/mailslurper/www/www.go`, is already committed in the pinned source, so the committed file was used unchanged.

**Timing:** 2026-11-07T16:21:06Z to 16:21:42Z (36 s), on 32 CPUs (`--cpuset-cpus=0-31`) with an 8 GiB memory cap.

**Network:** the download log (`build/go-mod-download.log`) shows only `proxy.golang.org` (748 GETs) and `sum.golang.org` (230 GETs, 134 of them `/lookup`).

**Compiler warning:** one `-Wreturn-local-addr` warning in the bundled `sqlite3-binding.c` (go-sqlite3 v2.0.3). It is benign and appears in `build/build.log`.

## Outputs

| File | Content |
|---|---|
| `mailslurper` | **sha256 `eb6cd97d114f881e6a6f7170ddbe1fe914d8f52f025839631e441807f2a458ff`**, 19942072 bytes. ELF x86-64, dynamically linked against glibc (`/lib64/ld-linux-x86-64.so.2`; NEEDED `libdl.so.2`, `libpthread.so.0`, `libc.so.6`), not stripped. It contains no `/work/` paths (trimpath). |
| `build/go-version-m.txt` | `go version -m` of the output. It shows go1.18.1, `CGO_ENABLED=1` and empty `CGO_*FLAGS`. The build info has no `vcs.*` because the tarball has no `.git`. |
| `build/go.mod`, `build/go.sum` | taken from the extracted tree after the build. They are **byte-identical** to the tarball's (checked with `diff`), so nothing was rewritten. |
| `build/go-list-m-all.txt` | `go list -m all` (offline): the main module plus 103 module@version entries |
| `build/modcache-modules.txt` | every `module@version` that has a `.zip` in the module cache: 103 entries, in the cache's case-escaped form (`!knetic` = `Knetic`). It equals the `go list -m all` set. |
| `build/go-mod-download.log` | `go mod download -x` stderr, with every proxy and sumdb URL fetched |
| `build/go-env.txt`, `build/build.log`, `build/build-*.utc` | `go env`, the full container log, and start and end times |
| `build/config.json` | the shipped `cmd/mailslurper/config.json`, used for the launch below |

**Version string:** MailSlurper has no `--version` flag. `./mailslurper --version` prints `flag provided but not defined: -version` and usage, then exits 2 (`launch-cgo/cgo-version-flag.log`). At start-up it logs **`Starting MailSlurper Server v1.14.1`**. The release-1.15.0 source hard-codes `SERVER_VERSION string = "1.14.1"` (`cmd/mailslurper/mailslurper.go:26`, also in `version.json`). That explains why the official binary reports v1.14.1 too.

## Bounded launch of the cgo build: verdict STARTS, LISTENS, ACCEPTS AND STORES MAIL

The launch ran at 2026-11-07T16:22:16Z.

**Setup:**

- **Run directory:** a fresh empty directory under /home holding only the binary and the shipped `config.json`. The binary was run as-is, with no flags.
- **Ports:** the config binds `localhost` on SMTP 2500, admin/www 8080 and service API 8085. Before the launch, `ss` confirmed that none of the three ports was in use. A private network namespace (`unshare -rn`) was attempted first, but user namespaces are not permitted on this host, so the launch used host loopback.
- **Bound:** `timeout 20 ./mailslurper`, stopped with SIGINT after the probe.

**Scripts:** `scripts/launch_probe.sh` (copy) did the launch; `scripts/smtp_send_one.py` (copy, run with `python3 -I`) sent the mail.

**Results** (`launch-cgo/launch-cgo.log`, `launch-cgo/mailslurper.std{out,err}.log`):

- **SQLite:** opened. The log shows `Connecting to database`, then `Creating database tables...`, then `Created tables successfully.`. `mailslurper.db` (20480 bytes) was created in the run directory.
- **Listening:** `ss -ltnp` shows `mailslurper` on `127.0.0.1:2500`, `127.0.0.1:8080` and `127.0.0.1:8085`.
- **One SMTP mail:** `EHLO 250`, `MAIL FROM 250`, `RCPT TO 250`, `DATA 354`, end-of-data **`250 Ok`**, `QUIT 221`.
- **Stored and queryable:** `GET http://localhost:8085/mail` returned 1 item with subject `issue340 cgo probe` and the correct from, to and body. `/mailcount` returned `{"mailCount":1}`. `GET /` on 8080 returned 200.
- **Server log:** `Mail item … written`. It also logs `Problem parsing date`, because the probe message had no `Date:` header. That is harmless; MailSlurper substitutes the receipt time.
- **Shutdown:** SIGINT gives exit 130. The last log line is `level=fatal msg="Error shutting down service listener: context canceled"`, which is MailSlurper's normal shutdown path.
- **Cleanup:** afterwards no process remained and the ports were free. The run directory, the build directory, the module and build caches, and the pulled builder image were all removed. They were created by this supply step, and only they were removed.

## Comparison: the official release binary (for the published result)

Input: `../../2026-11-07-mailslurper/mailslurper-1.15.0-linux.zip`, sha256 `48f2f9ba090f5385301477406b4f6e08f7bfbba0619c635ee591d6e6af3e55f6`. `mailslurper` and `config.json` were unzipped into a fresh run directory. `official-release-run/extracted-files.sha256` records:

- `mailslurper`: `0cd9416b766deb26eed8aa01ad0873d06725a3464fdb9ac3cc82201e2c232627`
- `config.json`: `77d7f97c…`, byte-identical to the source's `cmd/mailslurper/config.json`

The official binary is ELF x86-64, **statically linked** and stripped. Its embedded Go build info (`official-release-run/embedded-buildinfo.txt`, extracted with `strings`; the binary was not run for this) shows:

- built with go1.20.6
- **`CGO_ENABLED=0`**
- `-ldflags="-s -w"`
- `vcs.revision=068acffdcd8960f248ea438c3b7ee51ead601bd2`, `vcs.time=2023-08-03T20:23:20Z`
- `vcs.modified=true`: it was built from a dirty tree at the release commit

**Bounded run:** one run of `timeout 10 ./mailslurper` with the shipped config, at 2026-11-07T16:22:29Z. It **exited 1 within a second**, with nothing on stdout. stderr (`official-release-run/mailslurper.stderr.log`):

```
level=info msg="Starting MailSlurper Server v1.14.1" who=MailSlurper
level=info msg="Using configuration from config.json" who=MailSlurper
level=info msg="Connecting to database" who=MailSlurper
level=info msg="Creating database tables..." who=MailSlurper
level=fatal msg="Error connecting to storage type '1' with a connection string of ./mailslurper.db" error="Error executing query: \n\t\tCREATE TABLE mailitem (...);: Binary was compiled with 'CGO_ENABLED=0', go-sqlite3 requires cgo to work. This is a stub" who=MailSlurper
```

The official binary never opens a listener. Every row's start-up and mail figures for MailSlurper must therefore come from the "built from source" cgo binary, `eb6cd97d…`, or from the Alkemio image's binary on its pinned musl loader (see `../alkemio-image/`).

## Licence

MailSlurper's `LICENSE` in the source tarball is MIT. The cgo binary statically includes SQLite through go-sqlite3. SQLite is public domain, and go-sqlite3 is MIT. At run time it links dynamically against the host's glibc (LGPL-2.1+).
