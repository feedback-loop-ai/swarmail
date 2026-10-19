#!/usr/bin/env bash
# Public entry point, fixed dispatch only (decision 0050 §1): the canonical
# implementation lives in the pinned bundle, bundles/self/scripts/. The verify
# seat runs the pinned copy directly and refuses this file unless its bytes
# match bundles/self/scripts/shims.sha256 — it never selects the algorithm.
cd "$(dirname "${BASH_SOURCE[0]}")/.." || exit 1
exec bash bundles/self/scripts/publish-guard.sh "$@"
