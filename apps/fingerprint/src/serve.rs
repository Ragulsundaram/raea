//! The resident service: the encoders loaded once, on 127.0.0.1 at a port the host file names
//! (~/.raea/fingerprints/.serve.json). A request is one JSON line, {"args": [...], "cwd": "..."}; the answer is
//! progress lines, {"say": "..."}, then {"out": "...", "code": N}. One request at a time, in order.

use crate::{Eval, verbs};
use anyhow::{Context, Result};
use raea_sys_columns::Encoders;
use raea_sys_fingerprint::folder::root;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The service leaves after this long with no request
const IDLE: Duration = Duration::from_secs(30 * 60);
/// How long a verb waits for a service it started to come up (the first run fetches the models)
const UP: Duration = Duration::from_secs(900);

fn host() -> PathBuf {
    root().join(".serve.json")
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// `raea fingerprint serve`: load the encoders, listen, answer until idle or stopped
pub fn serve(eval: Eval) -> Result<()> {
    let started = Instant::now();
    let mut enc = Encoders::load().context("loading the encoders")?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    std::fs::create_dir_all(root())?;
    std::fs::write(host(), json!({"port": port, "pid": std::process::id()}).to_string())?;
    eprintln!("raea fingerprint serve: encoders loaded in {:.1} s, on 127.0.0.1:{port}", started.elapsed().as_secs_f64());
    let last = Arc::new(AtomicU64::new(now()));
    let seen = last.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(60));
            if now().saturating_sub(seen.load(Ordering::Relaxed)) > IDLE.as_secs() {
                let _ = std::fs::remove_file(host());
                eprintln!("raea fingerprint serve: idle for {} min, leaving", IDLE.as_secs() / 60);
                std::process::exit(0);
            }
        }
    });
    for stream in listener.incoming() {
        let Ok(mut s) = stream else { continue };
        last.store(now(), Ordering::Relaxed);
        let mut line = String::new();
        if BufReader::new(&s).read_line(&mut line).is_err() {
            continue;
        }
        let req: Value = serde_json::from_str(&line).unwrap_or_default();
        let args: Vec<String> = serde_json::from_value(req["args"].clone()).unwrap_or_default();
        let cwd = PathBuf::from(req["cwd"].as_str().unwrap_or("."));
        if args.first().is_some_and(|v| v == "stop") {
            let _ = writeln!(s, "{}", json!({"out": "raea fingerprint serve: stopped", "code": 0}));
            let _ = std::fs::remove_file(host());
            return Ok(());
        }
        let mut say = |m: String| {
            let _ = writeln!(s, "{}", json!({ "say": m }));
        };
        let (out, code) = verbs::run(&mut enc, &args, &cwd, eval, &mut say);
        let _ = writeln!(s, "{}", json!({"out": out, "code": code}));
        last.store(now(), Ordering::Relaxed);
    }
    Ok(())
}

/// A verb run in this process, with the encoders loaded for it
pub(crate) fn here(args: &[String], cwd: &Path, eval: Eval) -> (String, i32) {
    match Encoders::load() {
        Ok(mut enc) => verbs::run(&mut enc, args, cwd, eval, &mut |m| eprintln!("{m}")),
        Err(e) => (format!("raea fingerprint: loading the encoders: {e:#}"), 1),
    }
}

fn reach() -> Option<TcpStream> {
    let h: Value = serde_json::from_slice(&std::fs::read(host()).ok()?).ok()?;
    TcpStream::connect(("127.0.0.1", h["port"].as_u64()? as u16)).ok()
}

/// The service started in the background, its output in ~/.raea/fingerprints/.serve.log; its stream once it's up
fn start() -> Result<TcpStream, String> {
    let _ = std::fs::remove_file(host());
    std::fs::create_dir_all(root()).map_err(|e| e.to_string())?;
    let log = root().join(".serve.log");
    let file = std::fs::File::create(&log).map_err(|e| e.to_string())?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut cmd = std::process::Command::new(exe);
    cmd.args(["fingerprint", "serve"]).stdin(std::process::Stdio::null()).stdout(file.try_clone().map_err(|e| e.to_string())?).stderr(file);
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut cmd, 0);
    let mut child = cmd.spawn().map_err(|e| format!("starting the service: {e}"))?;
    eprintln!("raea fingerprint: starting the service (loading the encoders once; its log is {})", log.display());
    let t = Instant::now();
    while t.elapsed() < UP {
        if let Some(s) = reach() {
            return Ok(s);
        }
        if let Ok(Some(status)) = child.try_wait() {
            let tail = std::fs::read_to_string(&log).unwrap_or_default();
            return Err(format!("the service stopped ({status}):\n{}", tail.lines().rev().take(12).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n")));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    let _ = child.kill();
    Err(format!("the service didn't come up in {} s (see {})", UP.as_secs(), log.display()))
}

/// A verb sent to the service, started if it isn't running; its progress printed as it comes
pub(crate) fn call(args: &[String], cwd: &Path) -> (String, i32) {
    let stream = match reach() {
        Some(s) => Ok(s),
        None if args.first().is_some_and(|v| v == "stop") => return ("raea fingerprint: no service running".into(), 0),
        None => start(),
    };
    let mut s = match stream {
        Ok(s) => s,
        Err(e) => return (format!("raea fingerprint: {e}"), 1),
    };
    if writeln!(s, "{}", json!({"args": args, "cwd": cwd})).is_err() {
        return ("raea fingerprint: the service closed the connection".into(), 1);
    }
    for line in BufReader::new(s).lines() {
        let Ok(line) = line else { break };
        let v: Value = serde_json::from_str(&line).unwrap_or_default();
        if let Some(m) = v["say"].as_str() {
            eprintln!("{m}");
        } else if v.get("code").is_some() {
            return (v["out"].as_str().unwrap_or("").to_string(), v["code"].as_i64().unwrap_or(1) as i32);
        }
    }
    ("raea fingerprint: the service ended without an answer (see ~/.raea/fingerprints/.serve.log)".into(), 1)
}
