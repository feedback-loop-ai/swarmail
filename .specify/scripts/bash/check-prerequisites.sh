#!/usr/bin/env bash
# Public entry point, fixed dispatch only (decision 0050 §1): the canonical
# implementation lives in the pinned bundle, bundles/self/scripts/speckit/.
# The verify seat refuses this file unless its bytes match
# bundles/self/scripts/shims.sha256 — it never selects the algorithm.
cd "$(dirname "${BASH_SOURCE[0]}")/../../.." || exit 1
exec bash bundles/self/scripts/speckit/check-prerequisites.sh "$@"
