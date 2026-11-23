# 0138 — Bind nested driver usage to a committed paid attempt

- **Status:** proposed
- **Date:** 2026-11-23
- **Scope:** Part of #404, #391 and #164. [Specification](../../specs/nested-driver-usage/spec.md)

## Proposed evidence contract

0119 and 0136 at 64f4763e already authorize the nested driver as the token source. Accepted 0137 at 33be71de settles the profiles and the unit of score commitment. A separately written trial-id lookup could associate a bill from an interrupted attempt with its replacement.

Publish a versioned closed companion in the existing retained trial leaf. Its owner is the harness. Bind its digest in the same metadata commit that validates the score. Verify trial, slot/attempt, profile and source on replay. Expose only the six typed session fields specified in the linked design. Source-specific absence and ambiguity propagate to the reader. Retain independently valid fields. Keep schema-2 result rows and existing grade semantics.

This adds one small artifact and one digest per nested trial. Interruption can leave a paid execution unmeasured. Recovery follows 0137's cessation rule. A committed score consumes its slot even when usage is unavailable. The proposal adds no authority to spend, parse a direct CLI stream, or change mail behavior. Only the operator accepts this record.
