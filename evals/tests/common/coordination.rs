//! Blocking coordination for parent test scopes only. Production never waits.
pub fn trial_scope() -> std::fs::File {
    use std::os::unix::fs::OpenOptionsExt;
    let uid = unsafe { libc::geteuid() };
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(format!("/tmp/swarmail-evals-tests-{uid}.lock"))
        .unwrap();
    file.lock().unwrap();
    file
}
