//! The paid baseline's offline CLI stand-in (proposed 0137), built as a
//! native executable because a pinned CLI must be one.
//!
//! In its own mode a test appends a [`MARKER`] and a JSON configuration to
//! a copy of this image: the configured fake CLI is then one file, so the
//! digest a row records covers every byte that ran. It answers `--version`
//! with its configured banner; any other launch logs its family and model
//! (dsh: its route), holds (bounded) while the hold file names this launch
//! as `<family> <model> <n>`, and exits 0: a measured failure.
//!
//! Without a configuration it is a forwarder that replaces itself with
//! `<path>.hop`, keeping argv and the environment: the launch-boundary
//! proofs use that mode to run their capturing script. A forwarder's pin
//! covers only the forwarder, which is why no CLI-identity proof uses it.
//! This is test equipment: no production path names it.
use std::io::Write as _;
use std::os::unix::process::CommandExt as _;

const MARKER: &[u8] = b"\0PAID-CLI-STANDIN-CONFIG\0";

#[derive(serde::Deserialize)]
struct Config {
    family: String,
    version: String,
    log: std::path::PathBuf,
    hold: std::path::PathBuf,
    canaries: Vec<String>,
}

fn main() {
    let Ok(me) = std::env::current_exe() else {
        std::process::exit(127);
    };
    let Ok(image) = std::fs::read(&me) else {
        std::process::exit(127);
    };
    // A configuration is the last marker followed by JSON. The image's own
    // copy of the marker is followed by other bytes, so a bare image (the
    // forwarder) finds none.
    let config = image
        .windows(MARKER.len())
        .rposition(|w| w == MARKER)
        .and_then(|at| {
            serde_json::Deserializer::from_slice(&image[at + MARKER.len()..])
                .into_iter::<Config>()
                .next()?
                .ok()
        });
    if let Some(config) = config {
        std::process::exit(fake(&config));
    }
    let mut hop = me.into_os_string();
    hop.push(".hop");
    let err = std::process::Command::new(hop)
        .args(std::env::args_os().skip(1))
        .exec();
    eprintln!("paid-cli-standin: {err}");
    std::process::exit(127);
}

fn fake(c: &Config) -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--version") {
        println!("{}", c.version);
        return 0;
    }
    let append = |line: &str| {
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&c.log)
            .expect("launch log");
        writeln!(f, "{line}").expect("launch log");
    };
    if args
        .iter()
        .any(|a| c.canaries.iter().any(|k| a.contains(k.as_str())))
    {
        append("LEAK");
    }
    let mut model = args
        .windows(2)
        .rev()
        .find(|w| w[0] == "--model")
        .map_or("none".to_string(), |w| w[1].clone());
    let set = |k: &str| std::env::var_os(k).is_some_and(|v| !v.is_empty());
    if set("DASHSCOPE_API_KEY") {
        model = "dashscope".into();
    }
    if set("SPARK_API_KEY") {
        model = "spark-glm".into();
    }
    let me = format!("{} {model}", c.family);
    append(&me);
    let n = std::fs::read_to_string(&c.log)
        .expect("launch log")
        .lines()
        .filter(|l| *l == me)
        .count();
    let held = format!("{me} {n}");
    // Bounded: at most 600 polls of 50 ms, as the shell fixture held.
    for _ in 0..600 {
        match std::fs::read_to_string(&c.hold) {
            Ok(s) if s.trim_end_matches('\n') == held => {
                std::thread::sleep(std::time::Duration::from_millis(50))
            }
            _ => break,
        }
    }
    0
}
