//! HTML named character references, semicolon-terminated, exact-case
//! (decision 0040 §5) — the data the bounded reference decoder looks a
//! name up in, never a second decoder.
//!
//! **Provenance — interim.** Decision 0040 §7 requires the complete set of
//! 2,125 semicolon-ended mappings from a pinned snapshot of the WHATWG
//! `entities.json` (<https://html.spec.whatwg.org/entities.json>), with its
//! SHA-256 digest, the WHATWG CC-BY-4.0 attribution and the generation
//! command recorded here. The run-17 implement seat had no network and no
//! readable local snapshot, so this table is a hand-transcribed subset:
//! the five XML names, their HTML5 uppercase aliases, the names the spec's
//! acceptance criteria pin (`Lt`, `fjlig`, `NotEqualTilde`, `ThickSpace`,
//! `nGt`, `nLt`, ...) and the common typographic and punctuation names.
//! Every entry is a WHATWG mapping; none is invented. Replacing the table
//! with the full generated one is outstanding work.
//!
//! **Regeneration.** `scripts/gen-entities.py` documents the steps: fetch
//! and digest a snapshot, emit the `NAMED` block from it, replace the block
//! below and update this provenance note. The script keeps only the
//! `;`-terminated keys, sorts them by bytes, emits every code point of
//! each mapping and refuses a count other than 2,125. Generation is
//! maintenance work, never a build or network step.

/// `(name without '&' and ';', replacement)`, sorted by the name's bytes so
/// lookup is a binary search over a fixed table: O(log n) bounded-length
/// comparisons, no allocation, no runtime initialization.
static NAMED: &[(&str, &str)] = &[
    ("AMP", "&"),
    ("COPY", "\u{a9}"),
    ("GT", ">"),
    ("Gt", "\u{226b}"),
    ("LT", "<"),
    ("Lt", "\u{226a}"),
    ("NewLine", "\n"),
    ("NoBreak", "\u{2060}"),
    ("NonBreakingSpace", "\u{a0}"),
    ("NotEqualTilde", "\u{2242}\u{338}"),
    ("QUOT", "\""),
    ("REG", "\u{ae}"),
    ("Tab", "\t"),
    ("ThickSpace", "\u{205f}\u{200a}"),
    ("ThinSpace", "\u{2009}"),
    ("ZeroWidthSpace", "\u{200b}"),
    ("amp", "&"),
    ("apos", "'"),
    ("ast", "*"),
    ("bsol", "\\"),
    ("bull", "\u{2022}"),
    ("colon", ":"),
    ("comma", ","),
    ("commat", "@"),
    ("copy", "\u{a9}"),
    ("dollar", "$"),
    ("emsp", "\u{2003}"),
    ("ensp", "\u{2002}"),
    ("equals", "="),
    ("euro", "\u{20ac}"),
    ("excl", "!"),
    ("fjlig", "fj"),
    ("grave", "`"),
    ("gt", ">"),
    ("hellip", "\u{2026}"),
    ("laquo", "\u{ab}"),
    ("lcub", "{"),
    ("ldquo", "\u{201c}"),
    ("lowbar", "_"),
    ("lpar", "("),
    ("lsqb", "["),
    ("lsquo", "\u{2018}"),
    ("lt", "<"),
    ("mdash", "\u{2014}"),
    ("middot", "\u{b7}"),
    ("nGt", "\u{226b}\u{20d2}"),
    ("nLt", "\u{226a}\u{20d2}"),
    ("nbsp", "\u{a0}"),
    ("ndash", "\u{2013}"),
    ("num", "#"),
    ("percnt", "%"),
    ("period", "."),
    ("plus", "+"),
    ("quest", "?"),
    ("quot", "\""),
    ("raquo", "\u{bb}"),
    ("rcub", "}"),
    ("rdquo", "\u{201d}"),
    ("reg", "\u{ae}"),
    ("rpar", ")"),
    ("rsqb", "]"),
    ("rsquo", "\u{2019}"),
    ("semi", ";"),
    ("shy", "\u{ad}"),
    ("sol", "/"),
    ("thinsp", "\u{2009}"),
    ("times", "\u{d7}"),
    ("trade", "\u{2122}"),
    ("verbar", "|"),
    ("zwj", "\u{200d}"),
    ("zwnj", "\u{200c}"),
];

/// The replacement for `name` (the token between '&' and ';'), exact case.
pub(super) fn lookup(name: &str) -> Option<&'static str> {
    NAMED
        .binary_search_by(|&(key, _)| key.cmp(name))
        .ok()
        .map(|index| NAMED[index].1)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Binary search is only correct over a strictly sorted table: a
    // misplaced entry would silently stop decoding.
    #[test]
    fn the_table_is_strictly_sorted_and_every_name_resolves() {
        assert!(NAMED.windows(2).all(|pair| pair[0].0 < pair[1].0));
        for &(name, value) in NAMED {
            assert_eq!(lookup(name), Some(value), "{name}");
        }
        assert_eq!(lookup("nosuch"), None);
        assert_eq!(lookup("Amp"), None, "lookup is exact-case");
    }

    // The decoder hands a name lookup only the token inside its
    // 40-character window, so every `name;` must fit there; and the
    // decision 0040 §6 growth bound is checked against the table itself:
    // `&name;` of n + 2 bytes yields at most 6/5 of that (`&nGt;`, five
    // bytes to six).
    #[test]
    fn every_name_fits_the_window_and_expands_at_most_six_fifths() {
        for &(name, value) in NAMED {
            assert!(name.chars().count() < 40, "{name}");
            assert!(5 * value.len() <= 6 * (name.len() + 2), "{name}");
        }
        assert_eq!(lookup("nGt").map(str::len), Some(6));
    }
}
