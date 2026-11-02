# Publishing

[← README](../../README.md) · [Guide index](README.md)

The crate is **not published on crates.io** — no version has ever been
uploaded, and nothing in CI uploads one. What ships is the container image
(`ghcr.io/feedback-loop-ai/swarmail`) and the OCI Helm chart; from source,
install with `cargo install --locked --path .` from a checkout. The
package is prepared for a first upload should the operator choose to make
one: the packaged file set is curated in `Cargo.toml` (`include`) — the
sources, the embedded static assets, the real-protocol test suites, the
criterion bench, the WHATWG entity snapshot, both license texts and the
README, not the delivery constitution, the brokkr seats or the CI
workflow.

```bash
cargo publish --dry-run --locked   # proves the package builds; uploads nothing
```

An upload would be the operator's act (`cargo login <token>` with a
never-committed token, then `cargo publish --locked`, which publishes
exactly the dependency graph `Cargo.lock` pins); until one happens, no
document may claim the crate is on crates.io.

**Version policy:** a version bump is the operator's release act — bump
`version` in `Cargo.toml`, refresh `Cargo.lock` (`cargo check` keeps it in
lockstep, so `--locked` stays publishable), set `charts/swarmail/Chart.yaml`'s
`version`, `appVersion` and Artifact Hub image to the same version (a joint
image-and-chart tag needs all four equal, decision 0051), keep the client
packages under `clients/` on Cargo's version (a test checks it), append
`docs/releases/vX.Y.Z.md`, and sign the tag (see *Releases* in
[CONTRIBUTING](../../CONTRIBUTING.md)). Shipped release notes are
**append-only, with dated errata**: what a release shipped and measured is
never rewritten; a later correction is added as a dated *Correction* beside
the claim (or in the notes' "Errata and provenance" section) naming its
issue and commit, and work landed after a tag is recorded in dated
"Unreleased" sections until the next cut. `docs/releases/v0.1.0.md` has
been extended this way since its tag; its errata section lists each edit
and the batch that made it.
