#!/usr/bin/env python3
"""Emit src/extract/entities.rs's NAMED table from the pinned WHATWG snapshot.

Maintenance tooling (decisions 0040 §7, 0049 §2), never a build or test step:

    python3 -I scripts/gen-entities.py scripts/data/html-entities.json > named.rs.txt

then replace the `static NAMED` block in src/extract/entities.rs with
named.rs.txt and run `cargo fmt`. The input is the unmodified
https://html.spec.whatwg.org/entities.json, fetched 2026-10-18 (WHATWG,
CC-BY-4.0). Its raw bytes must hash to PINNED_SHA256 before anything is
parsed or emitted; a deliberate snapshot update changes the data file, this
pin and the provenance note in entities.rs together.

Only `;`-terminated keys are kept (the 106 legacy no-semicolon aliases are
deliberately not syntax, and each must equal its `;` counterpart), names
must be ASCII and unique, every code point of a multi-scalar mapping is
emitted, and any count other than 2,231 keys / 2,125 names is refused.
All checks run before the first line of output, so a refused input leaves
no partial table.
"""

import hashlib
import json
import sys

PINNED_SHA256 = "d741d877ac77c4194c4ad526b5b4a19aef8dfe411ab840a466891cdbb9f362e6"
EXPECTED_KEYS = 2231
EXPECTED = 2125
EXPECTED_LEGACY = 106


def rust_escape(value):
    out = []
    for ch in value:
        code = ord(ch)
        if ch in "\\\"":
            out.append("\\" + ch)
        elif 0x20 <= code < 0x7F:
            out.append(ch)
        else:
            out.append("\\u{%x}" % code)
    return "".join(out)


def unique_object(pairs):
    table = {}
    for key, value in pairs:
        if key in table:
            sys.exit("duplicate key " + key)
        table[key] = value
    return table


def main(path):
    with open(path, "rb") as handle:
        raw = handle.read()
    digest = hashlib.sha256(raw).hexdigest()
    if digest != PINNED_SHA256:
        sys.exit("snapshot SHA-256 %s is not the pinned %s" % (digest, PINNED_SHA256))
    table = json.loads(raw.decode("utf-8"), object_pairs_hook=unique_object)
    if len(table) != EXPECTED_KEYS:
        sys.exit("expected %d keys, found %d" % (EXPECTED_KEYS, len(table)))
    named = {}
    legacy = 0
    for key, entry in table.items():
        if not key.startswith("&") or not key[1:].rstrip(";").isascii():
            sys.exit("not an ASCII '&' name: " + key)
        value = "".join(chr(code) for code in entry["codepoints"])
        if value != entry["characters"]:
            sys.exit("codepoints and characters disagree for " + key)
        if not key.endswith(";"):
            legacy += 1
            if table.get(key + ";") != entry:
                sys.exit("legacy " + key + " differs from its ';' counterpart")
            continue
        name = key[1:-1].encode("ascii")
        if not name or name in named:
            sys.exit("empty or duplicate name for " + key)
        named[name] = value
    if len(named) != EXPECTED or legacy != EXPECTED_LEGACY:
        sys.exit(
            "expected %d names and %d legacy aliases, found %d and %d"
            % (EXPECTED, EXPECTED_LEGACY, len(named), legacy)
        )
    lines = ["static NAMED: &[(&str, &str)] = &["]
    for name in sorted(named):
        lines.append('    ("%s", "%s"),' % (name.decode("ascii"), rust_escape(named[name])))
    lines.append("];")
    sys.stdout.write("\n".join(lines) + "\n")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit("usage: gen-entities.py scripts/data/html-entities.json")
    main(sys.argv[1])
