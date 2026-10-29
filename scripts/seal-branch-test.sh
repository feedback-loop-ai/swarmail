#!/usr/bin/env bash
# Public entry point, fixed dispatch only (decision 0050 §1): the canonical
# implementation lives in the pinned bundle, .brokkr/recipes/self/scripts/. The verify
# seat runs the pinned copy directly and refuses this file unless its bytes
# match .brokkr/recipes/self/scripts/shims.sha256 — it never selects the algorithm.
cd "$(dirname "${BASH_SOURCE[0]}")/.." || exit 1
exec bash .brokkr/recipes/self/scripts/seal-branch-test.sh "$@"
