#!/usr/bin/env python3
"""Emit src/extract/entities.rs's NAMED table from a WHATWG entities.json.

Maintenance tooling (decision 0040 §7), never a build or test step:

    curl -o entities.json https://html.spec.whatwg.org/entities.json
    sha256sum entities.json            # record the digest in entities.rs
    python3 -I scripts/gen-entities.py entities.json > named.rs.txt

then replace the `static NAMED` block in src/extract/entities.rs with
named.rs.txt and update the module doc's provenance (snapshot date,
digest, the WHATWG CC-BY-4.0 attribution). Only `;`-terminated keys are
kept (the 106 legacy no-semicolon aliases are deliberately not syntax),
every code point of a multi-scalar mapping is emitted, and a count other
than 2,125 is refused.
"""

import json
import sys

EXPECTED = 2125


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


def main(path):
    with open(path, encoding="utf-8") as handle:
        table = json.load(handle)
    named = []
    for key, entry in table.items():
        if not key.startswith("&") or not key.endswith(";"):
            continue
        value = "".join(chr(code) for code in entry["codepoints"])
        if value != entry["characters"]:
            sys.exit("codepoints and characters disagree for " + key)
        named.append((key[1:-1].encode("ascii"), value))
    if len(named) != EXPECTED:
        sys.exit("expected %d semicolon-terminated names, found %d" % (EXPECTED, len(named)))
    named.sort(key=lambda pair: pair[0])
    print("static NAMED: &[(&str, &str)] = &[")
    for name, value in named:
        print('    ("%s", "%s"),' % (name.decode("ascii"), rust_escape(value)))
    print("];")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit("usage: gen-entities.py entities.json")
    main(sys.argv[1])
