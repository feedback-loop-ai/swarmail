/// One Prometheus sample: metric name, label pairs, value.
pub type Sample = (String, Vec<(String, String)>, f64);

fn is_metric_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == ':')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':')
}

fn is_label_name(name: &str) -> bool {
    !name.contains(':') && is_metric_name(name)
}

/// A strict reader for the Prometheus text-format subset `/metrics` emits,
/// written from the format's grammar and independent of the server's
/// encoder: `# TYPE <name> counter|gauge` lines; samples of a metric name,
/// an optional `{label="value",…}` set whose values admit only the `\\`,
/// `\"` and `\n` escapes and no raw LF, one space, a numeric value and an
/// LF. Anything else fails the whole scrape, as a strict scraper would.
pub fn parse_prometheus(text: &str) -> Result<Vec<Sample>, String> {
    let mut samples = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        if let Some(comment) = rest.strip_prefix('#') {
            let (line, tail) = comment.split_once('\n').ok_or("a comment without LF")?;
            let words: Vec<&str> = line.split(' ').collect();
            let typed = matches!(
                words.as_slice(),
                ["", "TYPE", name, "counter" | "gauge"] if is_metric_name(name)
            );
            if !typed {
                return Err(format!("a malformed comment line: #{line:?}"));
            }
            rest = tail;
            continue;
        }
        let end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == ':'))
            .unwrap_or(rest.len());
        let (name, tail) = rest.split_at(end);
        if !is_metric_name(name) {
            return Err(format!("a malformed metric name at {rest:?}"));
        }
        rest = tail;
        let mut labels = Vec::new();
        if let Some(tail) = rest.strip_prefix('{') {
            rest = tail;
            loop {
                let (label, tail) = rest.split_once('=').ok_or("a label without '='")?;
                if !is_label_name(label) {
                    return Err(format!("a malformed label name {label:?}"));
                }
                let mut chars = tail
                    .strip_prefix('"')
                    .ok_or("an unquoted label value")?
                    .chars();
                let mut value = String::new();
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some('\\') => value.push('\\'),
                            Some('"') => value.push('"'),
                            Some('n') => value.push('\n'),
                            other => return Err(format!("an illegal escape \\{other:?}")),
                        },
                        Some('\n') => return Err("a raw LF inside a label value".into()),
                        Some(c) => value.push(c),
                        None => return Err("an unterminated label value".into()),
                    }
                }
                rest = chars.as_str();
                labels.push((label.to_string(), value));
                if let Some(tail) = rest.strip_prefix(',') {
                    rest = tail;
                    continue;
                }
                rest = rest.strip_prefix('}').ok_or("a label set without '}'")?;
                break;
            }
        }
        let tail = rest.strip_prefix(' ').ok_or("no space before the value")?;
        let (value, tail) = tail.split_once('\n').ok_or("a sample without LF")?;
        let value: f64 = value
            .parse()
            .map_err(|_| format!("a non-numeric value {value:?}"))?;
        samples.push((name.to_string(), labels, value));
        rest = tail;
    }
    Ok(samples)
}
