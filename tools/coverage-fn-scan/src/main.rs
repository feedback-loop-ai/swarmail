//! The exact coverage gate's parser (decision 0035:
//! docs/decisions/0035-coverage-gate-refuses-by-effect.md). Two subcommands,
//! both queries over artifacts the build itself produced — the inventory is
//! extracted at ingest (build time) and nothing here re-derives one from
//! source text. A real parser for source, because the last four runs prove
//! bash cannot be one; the compiler's own tokens for its own output.
//!
//! `files` — harvest the production units' dep-info from a cargo build dir:
//! the lib via its `.rlib` rule, bins via executable rules whose
//! prerequisites name a bin root. Every entry is normalized against the
//! workspace root; an entry that does not resolve to an existing regular file
//! under the workspace refuses, naming it — never skips. The fn-bearing
//! subset (sorted, unique, workspace-relative) is printed to stdout, and the
//! consumed dep-files are copied to `--capture` when given, so the checks
//! read copies that outlive the build dir the trap removes. A `.rs` entry
//! that fails to parse refuses loudly (rustc parsed it; syn must too); a
//! non-Rust entry that does not parse is data and is exempt, as is a
//! declarations-only file with no function to count.
//!
//! `attrs` — walk a `-Zunpretty=expanded` dump's token tree and refuse any
//! `coverage(off)` / `no_coverage` attribute outside an
//! `#[automatically_derived]` impl block. The walk is over tokens, not a
//! parse: the canonical dump of the real tree cannot be parsed by any syn —
//! `std::pin::pin!` mints the experimental `super let` — while the compiler's
//! printer has already done every canonicalization a source-side recognizer
//! ever died trying to do (comments and whitespace are gone, every macro is
//! expanded, every `cfg_attr` resolved), so on the dump the spelling space is
//! exactly one and the tokens are the canonical form. The attribute is
//! rustc's only per-function exclusion mechanism on this toolchain, so this
//! is exactly the missing-record check, computed where the join is sound; the
//! carve-out is load-bearing, because the compiler's own `derive(Eq)` mints
//! `fn assert_fields_are_eq` and excludes it deliberately. An empty or
//! untotokenizable expansion refuses loudly.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let outcome = match args.split_first() {
        Some((cmd, rest)) if cmd == "files" => cmd_files(rest),
        Some((cmd, rest)) if cmd == "attrs" => cmd_attrs(rest),
        _ => Err(String::from(
            "usage: coverage-fn-scan files --workspace <root> --build-dir <dir> \
             --package <name> [--bin-root <rel>]... [--capture <dir>]\n       \
             coverage-fn-scan attrs <expanded.rs>...",
        )),
    };
    match outcome {
        Ok(code) => code,
        Err(message) => {
            eprintln!("coverage-fn-scan: {message}");
            ExitCode::FAILURE
        }
    }
}

// --- files: the production build's own file set ------------------------------

struct FileArgs {
    workspace: PathBuf,
    build_dir: PathBuf,
    package: String,
    bin_roots: Vec<String>,
    capture: Option<PathBuf>,
}

fn cmd_files(args: &[String]) -> Result<ExitCode, String> {
    let mut parsed = FileArgs {
        workspace: PathBuf::new(),
        build_dir: PathBuf::new(),
        package: String::new(),
        bin_roots: Vec::new(),
        capture: None,
    };
    let mut index = 0;
    while index < args.len() {
        // `--flag=value` and `--flag value` are the same flag; the gate uses
        // the equals form so a jq line can carry a whole argument at once.
        let (flag, inline) = match args[index].split_once('=') {
            Some((flag, value)) => (flag.to_string(), Some(value.to_string())),
            None => (args[index].clone(), None),
        };
        let value = |shift: usize| -> Result<String, String> {
            inline
                .clone()
                .or_else(|| args.get(index + shift).cloned())
                .ok_or_else(|| format!("{flag} needs a value"))
        };
        let skip = |inline: &Option<String>| if inline.is_some() { 1 } else { 2 };
        match flag.as_str() {
            "--workspace" => {
                parsed.workspace = PathBuf::from(value(1)?);
                index += skip(&inline);
            }
            "--build-dir" => {
                parsed.build_dir = PathBuf::from(value(1)?);
                index += skip(&inline);
            }
            "--package" => {
                parsed.package = value(1)?;
                index += skip(&inline);
            }
            "--bin-root" => {
                parsed.bin_roots.push(value(1)?);
                index += skip(&inline);
            }
            "--capture" => {
                parsed.capture = Some(PathBuf::from(value(1)?));
                index += skip(&inline);
            }
            other => return Err(format!("files: unknown argument {other}")),
        }
    }
    if parsed.workspace.as_os_str().is_empty()
        || parsed.build_dir.as_os_str().is_empty()
        || parsed.package.is_empty()
    {
        return Err(String::from(
            "files needs --workspace, --build-dir and --package",
        ));
    }
    harvest(&parsed)
}

/// Lexically normalize a path: drop `.` and resolve `..` without touching the
/// filesystem, so logical workspace paths compare component-wise.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn collect_depfiles(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_depfiles(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "d") {
            out.push(path);
        }
    }
}

/// One make rule of a dep-file: the target and its prerequisite list.
struct Rule {
    target: String,
    prereqs: Vec<String>,
}

fn parse_depfile(path: &Path) -> Result<Vec<Rule>, String> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("cannot read dep-info {}: {err}", path.display()))?;
    let mut logical: Vec<String> = Vec::new();
    let mut pending = String::new();
    for line in text.lines() {
        if let Some(stripped) = line.strip_suffix('\\') {
            pending.push_str(stripped);
            pending.push(' ');
        } else {
            pending.push_str(line);
            logical.push(std::mem::take(&mut pending));
        }
    }
    if !pending.is_empty() {
        logical.push(pending);
    }
    let mut rules = Vec::new();
    for line in logical {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some(split) = unescaped_colon(trimmed) else {
            continue;
        };
        let prereqs: Vec<String> = trimmed[split + 1..]
            .split_whitespace()
            .map(unescape)
            .filter(|prereq| !prereq.is_empty())
            .collect();
        if prereqs.is_empty() {
            continue; // the per-file no-prerequisite rules carry nothing
        }
        rules.push(Rule {
            target: unescape(trimmed[..split].trim_end()),
            prereqs,
        });
    }
    Ok(rules)
}

/// Index of the first `:` not escaped with a backslash.
fn unescaped_colon(line: &str) -> Option<usize> {
    line.char_indices()
        .position(|(index, ch)| ch == ':' && !line[..index].ends_with('\\'))
}

fn unescape(token: &str) -> String {
    token.replace("\\ ", " ").replace("\\#", "#")
}

fn file_name_of(target: &str) -> &str {
    target.rsplit('/').next().unwrap_or(target)
}

/// True when the target names the package's lib rlib: `lib<name>.rlib` or
/// `lib<name>-<hash>.rlib`. A lib's test harness never writes an `.rlib` rule.
fn is_lib_target(target: &str, package: &str) -> bool {
    let name = file_name_of(target);
    let Some(stem) = name.strip_suffix(".rlib") else {
        return false;
    };
    let Some(rest) = stem.strip_prefix("lib") else {
        return false;
    };
    rest == package
        || rest
            .strip_prefix(package)
            .is_some_and(|tail| tail.starts_with('-'))
}

/// True when the target names an unhashed executable for `package` — the final
/// bin artifact at the profile root. Test harnesses are hashed
/// (`<name>-<hash>`) or live under a `deps/` directory; both spellings skip.
fn is_bin_target(target: &str, package: &str) -> bool {
    let name = file_name_of(target);
    let stem = name.strip_suffix(".exe").unwrap_or(name);
    stem == package && !target.contains("/deps/")
}

fn harvest(args: &FileArgs) -> Result<ExitCode, String> {
    // Relative arguments resolve against the process cwd; the gate and the
    // selftest both pass absolute paths.
    let absolutize = |path: &Path| -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_default().join(path)
        }
    };
    let workspace = normalize(&absolutize(&args.workspace));
    let build_dir = normalize(&absolutize(&args.build_dir));
    // Bin roots arrive absolute or relative; the set comparison is on
    // workspace-relative paths either way.
    let bin_roots: Vec<String> = args
        .bin_roots
        .iter()
        .map(|root| {
            let abs = normalize(&workspace.join(root));
            let rel = abs.strip_prefix(&workspace).unwrap_or(&abs);
            rel.to_string_lossy().into_owned()
        })
        .collect();
    let mut depfiles = Vec::new();
    collect_depfiles(&build_dir, &mut depfiles);
    if depfiles.is_empty() {
        return Err(format!(
            "no dep-info (.d) file under {} — the harvest reads the instrumented \
             build the report came from; a missing dep-info refuses (decision 0035)",
            build_dir.display()
        ));
    }
    depfiles.sort();
    // A unit's dep-info may be written more than once in one build dir (the
    // final-artifact spelling and the per-unit spelling carry different rule
    // targets); the file set is the thing that matters, so rules dedup by set
    // and only a genuinely different set refuses.
    let mut lib_rules: Vec<(String, BTreeSet<String>)> = Vec::new();
    let mut bin_rules: BTreeMap<String, (String, BTreeSet<String>)> = BTreeMap::new();
    let mut captured: Vec<PathBuf> = Vec::new();
    for depfile in &depfiles {
        for rule in parse_depfile(depfile)? {
            // Classify on the rule target first: only a harvested unit's
            // entries are resolved and validated, never a build script's or a
            // dependency's (their sources live outside the workspace).
            let is_lib = is_lib_target(&rule.target, &args.package);
            if !is_lib && !is_bin_target(&rule.target, &args.package) {
                continue;
            }
            let set = normalized_set(&rule.prereqs, &workspace, &build_dir)?;
            if is_lib {
                captured.push(depfile.clone());
                lib_rules.push((rule.target.clone(), set));
                continue;
            }
            if !bin_roots.iter().any(|root| set.contains(root)) {
                continue; // bin-shaped but not a harvested bin unit
            }
            captured.push(depfile.clone());
            for root in &bin_roots {
                if set.contains(root) {
                    match bin_rules.get(root) {
                        Some((_, existing)) if *existing != set => {
                            return Err(format!(
                                "ambiguous dep-info for bin root {root}: two units \
                                 claim different file sets — a stale build dir refuses"
                            ));
                        }
                        _ => {
                            bin_rules.insert(root.clone(), (rule.target.clone(), set.clone()));
                        }
                    }
                }
            }
        }
    }
    captured.sort();
    captured.dedup();
    lib_rules.sort_by(|a, b| a.1.cmp(&b.1));
    lib_rules.dedup_by(|a, b| a.1 == b.1);
    if lib_rules.len() > 1 {
        let named: Vec<String> = lib_rules.iter().map(|(target, _)| target.clone()).collect();
        return Err(format!(
            "ambiguous lib dep-info for package {}: {} — the harvest reads one \
             instrumented lib unit; a stale build dir refuses",
            args.package,
            named.join(", ")
        ));
    }
    for root in &bin_roots {
        if !bin_rules.contains_key(root) {
            return Err(format!(
                "no dep-info rule names bin root {root} — the bin's own dep-info \
                 is missing from {} (decision 0035)",
                build_dir.display()
            ));
        }
    }
    let mut entries: BTreeSet<String> = BTreeSet::new();
    if let Some((_, set)) = lib_rules.first() {
        entries.extend(set.iter().cloned());
    }
    for (_, set) in bin_rules.values() {
        entries.extend(set.iter().cloned());
    }
    if entries.is_empty() {
        return Err(format!(
            "the production units of package {} have no file set — the harvest \
             found no lib rule and no bin root",
            args.package
        ));
    }
    let mut production: BTreeSet<String> = BTreeSet::new();
    for entry in &entries {
        let resolved = workspace.join(entry);
        if !fs::metadata(&resolved)
            .map(|m| m.is_file())
            .unwrap_or(false)
        {
            return Err(format!(
                "dep-info entry {entry:?} is no longer an existing regular file \
                 under {} — refusing, never skipping",
                workspace.display()
            ));
        }
        if declares_function(&resolved)? {
            production.insert(entry.clone());
        }
    }
    if let Some(capture) = &args.capture {
        fs::create_dir_all(capture)
            .map_err(|err| format!("cannot create capture dir {}: {err}", capture.display()))?;
        for depfile in &captured {
            let name = depfile.file_name().unwrap_or_default();
            fs::copy(depfile, capture.join(name))
                .map_err(|err| format!("cannot capture {}: {err}", depfile.display()))?;
        }
        eprintln!(
            "coverage-fn-scan: captured {} dep-info file(s) to {}",
            captured.len(),
            capture.display()
        );
    }
    for file in &production {
        println!("{file}");
    }
    Ok(ExitCode::SUCCESS)
}

/// Resolve a dep-info entry against the workspace root (this toolchain's
/// relative spellings) or the build dir (older layouts), and normalize it to a
/// workspace-relative path. An entry that does not resolve to an existing
/// regular file under the workspace refuses, naming it — never skips.
fn normalized_entry(entry: &str, workspace: &Path, build_dir: &Path) -> Result<String, String> {
    let candidate = Path::new(entry);
    let resolved = if candidate.is_absolute() {
        normalize(candidate)
    } else {
        let under_workspace = normalize(&workspace.join(candidate));
        if fs::metadata(&under_workspace).is_ok() {
            under_workspace
        } else {
            normalize(&build_dir.join(candidate))
        }
    };
    match fs::metadata(&resolved) {
        Ok(meta) if meta.is_file() => {}
        _ => {
            return Err(format!(
                "dep-info entry {entry:?} does not resolve to an existing regular \
                 file under {} — a hostile entry refuses, never skips",
                workspace.display()
            ))
        }
    }
    let Ok(relative) = resolved.strip_prefix(workspace) else {
        return Err(format!(
            "dep-info entry {entry:?} resolves outside the workspace {} — \
             refusing, never skipping",
            workspace.display()
        ));
    };
    Ok(relative.to_string_lossy().into_owned())
}

fn normalized_set(
    entries: &[String],
    workspace: &Path,
    build_dir: &Path,
) -> Result<BTreeSet<String>, String> {
    entries
        .iter()
        .map(|entry| normalized_entry(entry, workspace, build_dir))
        .collect()
}

/// Does the file declare at least one function (any nesting depth)? A `.rs`
/// entry that fails to parse is a loud refusal; a non-Rust entry that fails is
/// data (`include_str!` assets) and is exempt, and so is a file with no
/// functions (`src/model.rs` — nothing any dimension could count).
fn declares_function(path: &Path) -> Result<bool, String> {
    let is_rust = path.extension().is_some_and(|ext| ext == "rs");
    let bytes = fs::read(path).map_err(|err| format!("cannot read {}: {err}", path.display()))?;
    let text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) if !is_rust => return Ok(false),
        Err(_) => {
            return Err(format!(
                "cannot read {} as UTF-8 — rustc compiled it, so syn must read it",
                path.display()
            ))
        }
    };
    match syn::parse_file(&text) {
        Ok(file) => Ok(items_declare_fn(&file.items)),
        Err(_err) if !is_rust => Ok(false),
        Err(err) => Err(format!(
            "syn cannot parse {} ({err}) — rustc parsed it, so syn must too; the \
             remedy is a helper lock bump",
            path.display()
        )),
    }
}

fn items_declare_fn(items: &[syn::Item]) -> bool {
    items.iter().any(item_declares_fn)
}

fn item_declares_fn(item: &syn::Item) -> bool {
    match item {
        syn::Item::Fn(_) => true,
        syn::Item::Mod(module) => module
            .content
            .as_ref()
            .is_some_and(|(_, items)| items_declare_fn(items)),
        syn::Item::Impl(imp) => imp
            .items
            .iter()
            .any(|ii| matches!(ii, syn::ImplItem::Fn(_))),
        syn::Item::Trait(trait_) => trait_
            .items
            .iter()
            .any(|ti| matches!(ti, syn::TraitItem::Fn(f) if f.default.is_some())),
        // A macro item can generate functions when compiled (a
        // `macro_rules!` body, an invocation whose definition lives
        // elsewhere), and a const or static item holds an expression that
        // can name a function pointer or a closure — executable code the
        // report must account for. Declaring them "declarations-only" would
        // discard a production harness-named file carrying exactly that
        // shape, and its missing SF record would then never refuse (the
        // review's HIGH on the inventory). When in doubt the file stays in
        // the inventory: over-inclusion is loud, under-inclusion is silent.
        syn::Item::Macro(_) => true,
        syn::Item::Const(_) => true,
        syn::Item::Static(_) => true,
        _ => false,
    }
}

// --- attrs: the canonical expansion's exclusion attributes --------------------
//
// The scan is a walk over the expansion's own token tree, not a parse: the
// canonical dump of the real tree cannot be parsed by any syn —
// `std::pin::pin!` (src/api.rs:521) mints the experimental `super let`
// (verified: syn 2.0.119 through 3.0.9 refuse the same token), while the
// compiler's printer has already done every canonicalization a source-side
// recognizer ever died trying to do. On the dump the spelling space is
// exactly one — comments and whitespace are gone before the tokens exist,
// every macro is expanded and every `cfg_attr` resolved — so the tokens ARE
// the canonical form, and a token walk over them is a query over the
// compiler's own output, not a re-derivation from source. `no_coverage` is
// the deprecated precursor; `coverage(off)` is rustc's only per-function
// exclusion mechanism on this toolchain.

fn cmd_attrs(args: &[String]) -> Result<ExitCode, String> {
    if args.is_empty() {
        return Err(String::from("attrs needs at least one expansion file"));
    }
    let mut failures: Vec<String> = Vec::new();
    for file in args {
        let text = fs::read_to_string(file)
            .map_err(|err| format!("cannot read expansion {file}: {err}"))?;
        if text.trim().is_empty() {
            return Err(format!(
                "expansion {file} is empty — it must resolve the same cfg set as \
                 the instrumented build"
            ));
        }
        let stream: proc_macro2::TokenStream = text.parse().map_err(|err| {
            format!(
                "cannot tokenize expansion {file} ({err}) — the dump must be the \
                 compiler's own output; a cfg mismatch refuses loudly, never \
                 silently"
            )
        })?;
        let mut scan = Scan {
            file: file.clone(),
            offenders: Vec::new(),
        };
        let tokens: Vec<proc_macro2::TokenTree> = stream.into_iter().collect();
        scan_tokens(&tokens, false, &mut scan);
        failures.extend(scan.offenders);
    }
    if failures.is_empty() {
        Ok(ExitCode::SUCCESS)
    } else {
        for offender in &failures {
            eprintln!("{offender}");
        }
        Ok(ExitCode::FAILURE)
    }
}

struct Scan {
    file: String,
    offenders: Vec<String>,
}

/// The bracket group's exclusion spelling, if any: `coverage(off)` (any
/// argument list whose first token is `off`) or the bare `no_coverage`.
fn exclusion_kind(attr: &proc_macro2::TokenTree) -> Option<&'static str> {
    let group = match attr {
        proc_macro2::TokenTree::Group(group)
            if group.delimiter() == proc_macro2::Delimiter::Bracket =>
        {
            group
        }
        _ => return None,
    };
    let mut inner = group.stream().into_iter();
    match inner.next()? {
        proc_macro2::TokenTree::Ident(ident) if ident == "no_coverage" => Some("no_coverage"),
        proc_macro2::TokenTree::Ident(ident) if ident == "coverage" => match inner.next()? {
            proc_macro2::TokenTree::Group(args)
                if args.delimiter() == proc_macro2::Delimiter::Parenthesis =>
            {
                let mut arg = args.stream().into_iter();
                match arg.next()? {
                    proc_macro2::TokenTree::Ident(ident) if ident == "off" => Some("coverage(off)"),
                    _ => None,
                }
            }
            _ => None,
        },
        _ => None,
    }
}

fn is_automatically_derived(attr: &proc_macro2::TokenTree) -> bool {
    match attr {
        proc_macro2::TokenTree::Group(group)
            if group.delimiter() == proc_macro2::Delimiter::Bracket =>
        {
            let mut inner = group.stream().into_iter();
            matches!(
                inner.next(),
                Some(proc_macro2::TokenTree::Ident(ident)) if ident == "automatically_derived"
            ) && inner.next().is_none()
        }
        _ => false,
    }
}

/// `fn <name>` when the exclusion sits on a function, so a refusal names it.
fn following_fn_name(rest: &[proc_macro2::TokenTree]) -> String {
    match rest {
        [proc_macro2::TokenTree::Ident(fun), proc_macro2::TokenTree::Ident(name), ..]
            if fun == "fn" =>
        {
            format!(" on fn {name}")
        }
        _ => String::new(),
    }
}

fn scan_tokens(tokens: &[proc_macro2::TokenTree], derived: bool, scan: &mut Scan) {
    let mut index = 0;
    while index < tokens.len() {
        match &tokens[index] {
            proc_macro2::TokenTree::Group(group) => {
                let inner: Vec<_> = group.stream().into_iter().collect();
                scan_tokens(&inner, derived, scan);
                index += 1;
            }
            proc_macro2::TokenTree::Punct(punct) if punct.as_char() == '#' => {
                // An inner attribute `#![...]` (legal inside a fn body, and
                // the canonical expansion preserves it) carries a `!` between
                // the `#` and the bracket: skip it, or the exclusion behind
                // the bang is never seen (the review's HIGH on inner
                // attributes).
                let bang = matches!(
                    tokens.get(index + 1),
                    Some(proc_macro2::TokenTree::Punct(p)) if p.as_char() == '!'
                );
                let attr_index = if bang { index + 2 } else { index + 1 };
                let attr = match tokens.get(attr_index) {
                    Some(attr) => attr,
                    None => return,
                };
                if let Some(kind) = exclusion_kind(attr) {
                    if !derived {
                        let line = attr.span().start().line;
                        scan.offenders.push(format!(
                            "{}:{line}: {kind}{} — a function compiled but excluded \
                             from instrumentation",
                            scan.file,
                            following_fn_name(&tokens[attr_index + 1..])
                        ));
                    }
                    index = attr_index + 1;
                    continue;
                }
                if is_automatically_derived(attr) {
                    // The minted impl's body: the first brace group after the
                    // attribute. Everything carved stays carved only inside it
                    // — a later item in the same stream is checked on its own.
                    if let Some(offset) = (index + 2..).find(|&i| {
                        matches!(
                            tokens.get(i),
                            Some(proc_macro2::TokenTree::Group(group))
                                if group.delimiter() == proc_macro2::Delimiter::Brace
                        )
                    }) {
                        if let Some(proc_macro2::TokenTree::Group(body)) = tokens.get(offset) {
                            let inner: Vec<_> = body.stream().into_iter().collect();
                            scan_tokens(&inner, true, scan);
                        }
                        index = offset + 1;
                        continue;
                    }
                }
                index += 2;
            }
            _ => index += 1,
        }
    }
}
