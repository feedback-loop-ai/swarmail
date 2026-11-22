//! The paid baseline's offline CLI stand-in (proposed 0137), built as a
//! native executable because a pinned CLI must be one: an interpreter script
//! forwards to bytes its digest cannot cover, so the harness refuses it.
//!
//! A test copies this binary to the pinned path and writes the stand-in's
//! behaviour beside it as `<path>.hop`; this binary only replaces itself
//! with that file, keeping argv and the environment. It is test equipment:
//! no production path names it, and the harness never forwards to a hop.
use std::os::unix::process::CommandExt as _;

fn main() {
    let Ok(me) = std::env::current_exe() else {
        std::process::exit(127);
    };
    let mut hop = me.into_os_string();
    hop.push(".hop");
    let err = std::process::Command::new(hop)
        .args(std::env::args_os().skip(1))
        .exec();
    eprintln!("paid-cli-standin: {err}");
    std::process::exit(127);
}
