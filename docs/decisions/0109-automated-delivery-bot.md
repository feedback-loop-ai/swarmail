# 0109 — A delivery bot seals and merges, under its own key

- **Status:** accepted (operator, 2026-10-31 — the operator ruled this in
  chat).
- **Date:** 2026-10-31
- **Source:** operator direction, 2026-10-31, given in chat: "let's get the
  bot a key, and alter the policy, this will be fully automated delivery".
- **Amends:** [0056](0056-signing-policy.md) ruling 2 and its signature
  meanings, by dated addendum, from the day the bot is live. Until then the
  operator seals as before.
- **Preserves:** append-only `main`, signed release tags, the operator's sole
  acceptance of decisions, and the dependency allowlist rule.

## Context

0056 made one operator signature the last step before every merge. That
signature needs the operator's forwarded gpg agent, which is unreachable
whenever the operator's machine is. On 2026-10-31 finished, green work waited
hours for it, more than once. 0056 moved the socket out of the runs; it is
still in the way of every merge.

## Decision

1. **The bot seals and merges.** A PR that a delivery run shipped is sealed
   and squash-merged without an operator step. No per-PR operator instruction
   is needed.
2. **The bot has its own key and its own GitHub account.** Its signature names
   the bot. It never signs as the operator and never holds the operator's key.
   0056 refused a server-held key because "anything on the box could then sign
   as the operator"; this key cannot.
3. **The key lives under a separate Unix user.** Seats and the operator's
   assistant run as the operator's user and cannot read the key or the bot's
   GitHub credential. They cannot ask the bot to sign: it polls GitHub, and
   works only from what it fetches there, in its own clone.
4. **What the bot requires, at the exact tree it will merge:**
   - the PR carries the `deliver` label, which the delivery pipeline applies;
   - every house gate passes, run by the bot itself in its own clone (CI does
     not run `evals/` until #201 lands, so CI alone is not enough);
   - every required CI check is green on the sealed head.
5. **What the bot refuses, leaving the PR for the operator.** These are the
   standing rules this automation must not route around:
   - a change to a decision's status to accepted;
   - any new package in `Cargo.lock` or `evals/Cargo.lock`, or a change to
     `deny.toml`;
   - a change to the bot itself, `.github/`, the seal scripts or
     `.brokkr/recipes/self/policy.json`.
6. **What stays the operator's:** accepting decisions, widening dependencies,
   release tags and paid eval trials.

The signature's new meaning, alongside 0056's three kinds: **bot seal** — this
exact branch passed the bot's checks. It is not a person vouching for the code.

## Costs

No person sees a PR before it merges. Code that passes the gates and the
five-member review lands on `main`. The label is applied by the operator's own
user, so anything running as that user can request delivery; item 5 and the
gates are what limit it. The bot runs a full gate pass per PR, serially.

## Enforcement binding

None yet. The bot, its refusal list and their tests are the implementation of
this record, and must land before the addendum to 0056 takes effect. This
record does not change `scripts/seal-branch.sh` or any seat.
