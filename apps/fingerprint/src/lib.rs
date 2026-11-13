//! `raea fingerprint` (docs/fingerprint.feature FP3a): a person's fingerprint made, bootstrapped, taught and used to
//! score a piece. The encoders take seconds to load, so the verbs run in a resident service (`raea fingerprint
//! serve`, serve.rs) that keeps them loaded: the first verb starts it, on localhost, and later verbs reach it in
//! milliseconds (earshot's CLI reloads its models on every call, 20-40 s). It leaves after 30 idle minutes, or on
//! `raea fingerprint stop`. RAEA_FINGERPRINT_HERE=1 runs a verb in this process instead.

mod approve;
mod bootstrap;
mod pick;
mod serve;
mod verbs;

use std::path::Path;

pub use serve::serve;

pub const USAGE: &str = "raea fingerprint new NAME [--who W] [--about TEXT] [--extends FP] | raea fingerprint bootstrap FILE... [--from LIST] [--per N] | raea fingerprint teach FILE --axis A --at T [--bad|--good] [--span S] [--box x0,y0,x1,y1] [--words W] [--project P] | raea fingerprint approve FILE... [--project P] | raea fingerprint score FILE [--json] [--sheet out.png] [--timeline] | raea fingerprint report [--json] | raea fingerprint which [PATH] [--json] | raea fingerprint use NAME [--in project|workspace] [PATH] | raea fingerprint diff A B [--json] | raea fingerprint share NAME [--in project|workspace] [PATH] | raea fingerprint stop [--fp NAME]   (a person's sense of good work as axes with small heads over on-device encoders, docs/fingerprint.md: new makes ~/.raea/fingerprints/NAME with raea's base taxonomy in fingerprint.ts (or FP's axes with --extends) and what it's for and every axis scoring from its words; bootstrap puts known defects into windows of real work and trains the mechanical axes, held out by source; teach adds a label at a moment; approve makes approved work clean on every axis, away from its notes; score prints the moments it expects a note on first, then every axis per window with --json, a sheet PNG with --sheet, or the piece second by second in words with --timeline; report says each head's held-out score, labels, cheats, gate and what it can't judge; which says the one that judges a piece (its project's, its workspace's, then yours) and why, use pins one there, diff lists the axes two fingerprints rank the same evidence most differently on, and share copies one's taxonomy and evidence (no heads, no media) into a project or workspace for its team)";

/// Evaluates a .ts file's default export to its JSON (the CLI's spec host)
pub type Eval = fn(&Path) -> Result<String, String>;

/// The flags each verb takes, and whether each takes a value
pub(crate) const FLAGS: [(&str, bool); 19] = [
    ("--fp", true),
    ("--who", true),
    ("--about", true),
    ("--extends", true),
    ("--from", true),
    ("--per", true),
    ("--axis", true),
    ("--at", true),
    ("--span", true),
    ("--box", true),
    ("--words", true),
    ("--project", true),
    ("--sheet", true),
    ("--in", true),
    ("--bad", false),
    ("--good", false),
    ("--json", false),
    ("--timeline", false),
    ("--here", false),
];

/// A flag's value
pub(crate) fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).map(String::as_str)
}

pub(crate) fn has(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

/// The words that aren't flags or their values; an unknown flag is an error
pub(crate) fn words(args: &[String]) -> Result<Vec<&str>, String> {
    let mut out = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match FLAGS.iter().find(|(f, _)| f == a) {
            Some((_, true)) => {
                it.next().ok_or_else(|| format!("{a} takes a value"))?;
            }
            Some(_) => {}
            None if a.starts_with("--") => return Err(format!("no flag {a}")),
            None => out.push(a.as_str()),
        }
    }
    Ok(out)
}

/// `raea fingerprint VERB ...`, printed; its exit code (1: the verb failed, 2: the command was wrong)
pub fn cli(args: &[String], eval: Eval) -> i32 {
    let Some(verb) = args.first().map(String::as_str).filter(|v| !matches!(*v, "--help" | "-h" | "help")) else {
        println!("{USAGE}");
        return 0;
    };
    if let Err(e) = words(&args[1..]) {
        eprintln!("raea fingerprint: {e}\n{USAGE}");
        return 2;
    }
    match verb {
        "serve" => match serve(eval) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("raea fingerprint serve: {e:#}");
                1
            }
        },
        "report" => verbs::report_here(args),
        "which" => pick::which_here(&args[1..], &std::env::current_dir().unwrap_or_default()),
        "diff" => pick::diff_here(&args[1..]),
        "share" => pick::share_here(&args[1..], &std::env::current_dir().unwrap_or_default()),
        "use" => pick::use_here(&args[1..], &std::env::current_dir().unwrap_or_default()),
        "new" | "bootstrap" | "teach" | "approve" | "score" | "stop" => {
            let cwd = std::env::current_dir().unwrap_or_default();
            let here = has(args, "--here") || std::env::var_os("RAEA_FINGERPRINT_HERE").is_some();
            let (out, code) = if here { serve::here(args, &cwd, eval) } else { serve::call(args, &cwd) };
            if !out.is_empty() {
                println!("{}", out.trim_end());
            }
            code
        }
        v => {
            eprintln!("raea fingerprint: no verb {v}\n{USAGE}");
            2
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_usage_names_every_flag() {
        let missing: Vec<_> = super::FLAGS.iter().filter(|(f, _)| *f != "--here" && !super::USAGE.contains(f)).collect();
        assert!(missing.is_empty(), "raea fingerprint takes flags its usage doesn't name: {missing:?}");
    }
}
