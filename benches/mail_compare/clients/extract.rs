//! The frozen oracle extractors (accepted decision 0132 §6.1): the eval
//! oracle's `six_digit_code` and `labelled_url`, moved here unchanged so the
//! API clients run exactly the function the oracle runs. The eval crate
//! compiles this file by `#[path]` and re-exports both from `oracle`, so its
//! existing callers and their tests are untouched.

/// The URL on the text line that starts with `label`.
pub fn labelled_url(text: &str, label: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let rest = line.trim().strip_prefix(label)?;
        rest.split_whitespace()
            .find(|w| w.starts_with("http://"))
            .map(str::to_string)
    })
}

/// The first standalone six-digit run in `text`.
pub fn six_digit_code(text: &str) -> Option<String> {
    text.split(|c: char| !c.is_ascii_digit())
        .find(|w| w.len() == 6)
        .map(str::to_string)
}
