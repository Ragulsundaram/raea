use super::*;

thread_local! {
    /// set while run() builds the help, whose render line comes from a run of its own, so that run isn't checked
    pub static IN_HELP: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
/// A verb's usage as `raea help` gives it: each flag it takes with how many values follow it (`--snapshot T FILE`
/// takes two), how many arguments it takes besides its flags (None after a `...`: any number), and its help lines
#[derive(Debug, Default)]
pub struct Usage {
    pub flags: HashMap<String, usize>,
    pub slots: Option<usize>,
    pub lines: Vec<String>,
}

pub fn is_flag(t: &str) -> bool {
    t.len() > 1 && t.starts_with('-') && t[1..].trim_start_matches('-').starts_with(|c: char| c.is_ascii_alphanumeric())
}

/// `verb`'s usage from the help: every form of it (`raea part list | raea part add NAME ...` has two, `sounds [WORDS] |
/// sounds get PACK/INSTRUMENT` too), the flags its lines name (an "also:" in a line's note lists the ones an agent
/// rarely needs). None for a verb the help doesn't list, or one that passes its arguments on (ARGS: media's ffmpeg)
pub fn usage_of(help: &str, verb: &str) -> Option<Usage> {
    let mut u = Usage::default();
    let (mut most, mut any) = (0usize, false);
    for line in help.lines() {
        let line = line.trim().trim_start_matches("usage:").trim();
        let (form, note) = line.split_once("   (").unwrap_or((line, ""));
        let toks: Vec<&str> = form.split_whitespace().collect();
        // the forms on this line: (verb, its tokens)
        let mut forms: Vec<(String, Vec<&str>)> = Vec::new();
        let (mut i, mut depth) = (0, 0i32);
        while i < toks.len() {
            let t = toks[i];
            if t == "raea" && i + 1 < toks.len() {
                forms.push((toks[i + 1].to_string(), Vec::new()));
                i += 2;
                continue;
            }
            if t == "|" && depth == 0 {
                match toks.get(i + 1) {
                    Some(&"raea") => {}
                    Some(n) if is_flag(n) => if let Some(v) = forms.last().map(|f| f.0.clone()) { forms.push((v, Vec::new())) },
                    Some(n) if n.chars().all(|c| c.is_ascii_lowercase() || c == '-') => {
                        forms.push((n.to_string(), Vec::new()));
                        i += 1;
                    }
                    _ => {}
                }
                i += 1;
                continue;
            }
            depth += t.matches('[').count() as i32 - t.matches(']').count() as i32;
            if let Some(f) = forms.last_mut() {
                f.1.push(t);
            }
            i += 1;
        }
        if !forms.iter().any(|f| f.0 == verb) {
            continue;
        }
        if form.contains("ARGS") {
            return None;
        }
        u.lines.push(line.to_string());
        // flags with their values, from every form on the line and its note's "also:"
        let also: Vec<&str> = note.split_once("also:").map(|(_, a)| a.trim_end_matches(')').split_whitespace().collect()).unwrap_or_default();
        for toks in forms.iter().map(|f| &f.1[..]).chain([&also[..]]) {
            let mut k = 0;
            while k < toks.len() {
                let t = toks[k];
                let parts: Vec<&str> = t.trim_start_matches('[').trim_end_matches([']', ',', ';']).split('|').collect();
                if is_flag(parts[0]) {
                    let mut n = 0;
                    let mut closed = t.ends_with(']');
                    while !closed && k + 1 < toks.len() {
                        let v = toks[k + 1];
                        if v == "|" || v == "..." || v.starts_with('[') || is_flag(v.trim_start_matches('[')) {
                            break;
                        }
                        // a value with a flag for its other choice (`[--notes ..|--all]`)
                        for f in v.trim_end_matches(']').split('|').filter(|f| is_flag(f)) {
                            u.flags.entry(f.to_string()).or_insert(0);
                        }
                        n += 1;
                        k += 1;
                        closed = v.ends_with(']');
                    }
                    for f in parts.iter().filter(|f| is_flag(f)) {
                        let e = u.flags.entry(f.to_string()).or_insert(0);
                        *e = (*e).max(if *f == parts[0] { n } else { 0 });
                    }
                } else {
                    for f in parts.iter().filter(|f| is_flag(f)) {
                        u.flags.entry(f.to_string()).or_insert(0);
                    }
                }
                k += 1;
            }
        }
        // the arguments each of the verb's own forms takes besides its flags
        for (_, toks) in forms.iter().filter(|f| f.0 == verb) {
            let (mut n, mut k, mut last_pos) = (0, 0, false);
            while k < toks.len() {
                let t = toks[k];
                let bare = t.trim_start_matches('[');
                if is_flag(bare.split('|').next().unwrap_or("")) {
                    // its values were counted above; skip them
                    let mut closed = t.ends_with(']');
                    while !closed && k + 1 < toks.len() {
                        let v = toks[k + 1];
                        if v == "|" || v == "..." || v.starts_with('[') || is_flag(v.trim_start_matches('[')) {
                            break;
                        }
                        k += 1;
                        closed = v.ends_with(']');
                    }
                    last_pos = false;
                } else if t == "..." {
                    any |= last_pos;
                } else if t != "|" {
                    n += 1;
                    last_pos = true;
                    any |= t.trim_end_matches(']').ends_with("...");
                }
                k += 1;
            }
            most = most.max(n);
        }
    }
    if u.lines.is_empty() {
        return None;
    }
    u.slots = (!any).then_some(most);
    Some(u)
}

/// Why `args` isn't something its verb takes, by its usage in the help: a flag it doesn't name, or an argument past
/// the ones it takes. None when it is, or when the help doesn't give the verb's usage.
pub fn stray(help: &str, args: &[String]) -> Option<String> {
    let verb = args.first()?;
    // physics hands its subverb's arguments to the bake, the goo or the part, which check them; gen to raea-io-gen's own
    if verb == "physics" || verb == "gen" {
        return None;
    }
    let u = usage_of(help, verb)?;
    let refuse = |why: String| Some(format!("{why}\nusage: {}", u.lines.join("\n       ")));
    let (mut taken, mut i) = (0, 1);
    while i < args.len() {
        let a = &args[i];
        if is_flag(a) && a.parse::<f64>().is_err() {
            match u.flags.get(a.as_str()) {
                Some(n) => i += n,
                None => return refuse(format!("raea {verb} doesn't take {a:?}")),
            }
        } else {
            taken += 1;
            if let Some(slots) = u.slots.filter(|s| taken > *s) {
                return refuse(format!("raea {verb} doesn't take {a:?}: it takes {slots} argument{} besides its flags", if slots == 1 { "" } else { "s" }));
            }
        }
        i += 1;
    }
    None
}

/// Every verb the help lists, in its order (each `raea VERB` it names).
pub fn verb_list(help: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in help.lines() {
        // a usage line, not the help's prose ("raea: renders … raea compositions")
        let line = line.trim().trim_start_matches("usage:").trim();
        if !line.starts_with("raea ") {
            continue;
        }
        let toks: Vec<&str> = line.split("   (").next().unwrap_or("").split_whitespace().collect();
        for w in toks.windows(2).filter(|w| w[0] == "raea") {
            let v = w[1].trim_end_matches(':');
            if v.chars().all(|c| c.is_ascii_lowercase() || c == '-') && !v.is_empty() && !out.iter().any(|o| o == v) {
                out.push(v.to_string());
            }
        }
    }
    out
}

/// Where a verb runs (docs/agent-surfaces.md section 4): HEADLESS unless it needs the feed's host.
fn access(verb: &str) -> &'static str {
    match verb {
        // in the raea app's host: a folder an agent adds waits for the person's yes; what's shown is theirs to answer
        "library" => "HOST, then PERSON to add a folder",
        "show" => "HOST (starts the app), then PERSON to answer",
        // take and ls read files; post shows the asks on the person's comments through show
        "notes" => "HOST to show post's asks on the person's comments; take and ls run headless",
        _ => "HEADLESS",
    }
}

/// `raea help --json`: each verb with its usage lines, its flags (with how many values follow each), how many
/// arguments it takes (null: any number), its access tag, and `own_parser` when its own parser checks them (its flags
/// are then its own help's, not these). tools/skills.sh gen writes the CLI reference from it.
pub fn help_json(help: &str) -> serde_json::Value {
    // a verb whose arguments its own parser checks: stray() lets even a flag no verb has through
    let own = |v: &str| stray(help, &[v.to_string(), "--no-such-flag".to_string()]).is_none();
    let rows: Vec<_> = verb_list(help)
        .into_iter()
        .map(|v| match usage_of(help, &v) {
            Some(u) => {
                let mut flags: Vec<_> = u.flags.into_iter().collect();
                flags.sort();
                json!({"verb": v, "usage": u.lines, "flags": flags.into_iter().map(|(f, n)| json!({"flag": f, "values": n})).collect::<Vec<_>>(), "args": u.slots, "access": access(&v), "own_parser": own(&v)})
            }
            // passes its arguments on (media's ffmpeg): its line, unparsed
            None => json!({"verb": v, "usage": help.lines().map(str::trim).filter(|l| l.contains(&format!("raea {v} "))).collect::<Vec<_>>(), "flags": [], "args": null, "access": access(&v), "own_parser": true}),
        })
        .collect();
    json!({"verbs": rows})
}
