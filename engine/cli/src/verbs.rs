use super::*;

/// `raea mezz [--check] PATH...`: the sources under each PATH (followed through links, folders to four levels)
/// that want an optimized copy, made one at a time (or only listed, with --check). JSON: each source, what it is.
pub fn mezz(args: &[String]) -> R<String> {
    let check = args.iter().any(|a| a == "--check");
    let mut files = Vec::new();
    fn walk(p: &Path, depth: usize, out: &mut Vec<PathBuf>) {
        if p.is_dir() {
            if depth > 4 {
                return;
            }
            let Ok(rd) = std::fs::read_dir(p) else { return };
            let mut es: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| !p.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.'))).collect();
            es.sort();
            for e in es {
                walk(&e, depth + 1, out);
            }
        } else if matches!(p.extension().and_then(|x| x.to_str()).map(str::to_ascii_lowercase).as_deref(), Some("mp4" | "mov" | "m4v")) {
            out.push(p.to_path_buf());
        }
    }
    for a in args.iter().filter(|a| !a.starts_with("--")) {
        walk(Path::new(a), 0, &mut files);
    }
    let mut report = Vec::new();
    for f in files {
        let state = if raea_io_media::mezz::has(&f) {
            "ready".to_string()
        } else if !raea_io_media::mezz::probe(&f).is_some_and(|p| p.wants()) {
            "not needed".to_string()
        } else if check {
            "wanted".to_string()
        } else {
            let t = std::time::Instant::now();
            match raea_io_media::mezz::make(&f) {
                Ok(_) => format!("made in {:.1} s", t.elapsed().as_secs_f64()),
                Err(e) => format!("failed: {e}"),
            }
        };
        report.push(json!({"source": f, "copy": state}));
    }
    // range copies: an edit's frames (a folder's edit.tsx, or a .tsx named) show parts of long-GOP sources that have no
    // whole copy (1080p and under, or too long to copy whole): those parts, merged across gaps of up to 2 s and
    // padded by 1 s, get copies of their own (mezz::make_range)
    for a in args.iter().filter(|a| !a.starts_with("--")) {
        let p = Path::new(a);
        let tsx = if p.is_dir() { p.join("edit.tsx") } else { p.to_path_buf() };
        if tsx.extension().and_then(|x| x.to_str()) != Some("tsx") || !tsx.is_file() {
            continue;
        }
        let times = match raea_runtime_live::Live::open_baked(&tsx) {
            Ok(mut l) => l.video_times(),
            Err(e) => {
                report.push(json!({"edit": tsx, "ranges": format!("failed: {e}")}));
                continue;
            }
        };
        let mut by: std::collections::BTreeMap<PathBuf, Vec<f64>> = Default::default();
        for (file, t) in times {
            by.entry(file).or_default().push(t);
        }
        for (file, mut ts) in by {
            if raea_io_media::mezz::has(&file) || !raea_io_media::mezz::probe(&file).is_some_and(|p| p.long_gop()) {
                continue;
            }
            ts.sort_by(f64::total_cmp);
            let mut spans: Vec<(f64, f64)> = Vec::new();
            for t in ts {
                match spans.last_mut() {
                    Some(s) if t - s.1 <= 2.0 => s.1 = t,
                    _ => spans.push((t, t)),
                }
            }
            for (a, b) in spans {
                let (a, b) = ((a - 1.0).max(0.0), b + 1.0);
                let covered = raea_io_media::mezz::ranges(&file).iter().any(|r| r.start <= a + 1e-6 && r.end >= b - 1e-6);
                let state = if covered {
                    "ready".to_string()
                } else if check {
                    "wanted".to_string()
                } else {
                    let t = std::time::Instant::now();
                    match raea_io_media::mezz::make_range(&file, a, b) {
                        Ok(_) => format!("made in {:.1} s", t.elapsed().as_secs_f64()),
                        Err(e) => format!("failed: {e}"),
                    }
                };
                report.push(json!({"source": file, "range": [(a * 1000.0).round() / 1000.0, (b * 1000.0).round() / 1000.0], "copy": state}));
            }
        }
    }
    Ok(serde_json::to_string_pretty(&report).unwrap_or_default())
}



/// The agent shell's own verbs (engine/cli/src/index.ts) this binary runs; the rest come one by one (docs/agent-surfaces.md)
pub const SHELL: &[&str] = &["init", "cut", "frame", "component", "part", "brand", "library", "show"];

/// One of SHELL's verbs: @raea/cli's handler, run by engine/cli/src/native.ts on raea's JavaScriptCore with Node's core, so the
/// verb's one source is its TypeScript. native.ts answers the host's ops it needs, and runs this binary for the engine's verbs.
pub fn shell(args: &[String]) -> R<String> {
    let script = raea_io_home::home().join("engine/cli/src/native.ts");
    if !script.is_file() {
        return Err(format!("no agent shell at {}: reinstall raea (curl -fsSL https://raea.io/install | sh)", script.display()));
    }
    let me = std::env::current_exe().map_err(|e| e.to_string())?;
    let argv: Vec<String> = std::iter::once(me.display().to_string()).chain(args.iter().cloned()).collect();
    let r = raea_native::js::run_node(&script, &argv, String::new(), std::time::Duration::from_secs(3600));
    if r.exit != 0 {
        return Err(format!("{}{}", r.stdout, r.stderr).trim().to_string());
    }
    eprint!("{}", r.stderr);
    Ok(r.stdout.trim_end().to_string())
}
