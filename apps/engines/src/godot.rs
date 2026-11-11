//! `raea setup godot`, `raea godot types` and `raea godot open`; `raea godot run` is godot_run.rs. raea's Godot is
//! Godot 4.6.1 with GodotJS (github.com/godotjs/GodotJS, MIT): Godot's whole API from TypeScript, on Godot's own
//! JavaScript engine, JavaScriptCore on the Mac and QuickJS-ng on Windows and Linux (no V8). Godot without GodotJS
//! can't run a script, so raea uses GodotJS's editor build (the editor, headless or not, runs scripts and projects),
//! downloaded from its GitHub release into ~/.raea/tools/godot and checked against the sha256 pinned here.
use std::path::{Path, PathBuf};
use std::process::Command;

use raea_app_tools::blender::own_dir;

type R<T> = Result<T, String>;

pub const USAGE: &str = "raea setup godot [--check]   (download Godot 4.6.1 with GodotJS, its TypeScript bindings (MIT), into ~/.raea/tools/godot: JavaScriptCore on the Mac, QuickJS on Windows and Linux; checked against its pinned sha256)\n       raea godot run SCRIPT.ts [--project DIR] [--window] [--movie FILE [--fps N]] [--size WxH] [--timeout S] [ARGS...]   (a TypeScript script against Godot's whole API through raea/godot: its default export gets the SceneTree and ARGS, headless unless --window; --movie writes every frame Godot draws (NAME.png numbered, or .avi) at a fixed --fps, 30 unless given; the project is DIR, else the nearest project.godot above the script, else an empty one; Godot quits when it returns)\n       raea godot types [DIR]   (the project's own TypeScript types from GodotJS: Godot's API with its scenes, resources and input actions, into DIR/typings)\n       raea godot open [DIR]   (the Godot editor on the project, GodotJS's, to edit its scenes)";

/// The GodotJS release raea downloads (github.com/godotjs/GodotJS/releases); sdk/godot/tools/gen.sh writes
/// raea/godot's types from it, so the two move together.
pub const RELEASE: &str = "v1.1.0.beta1-4.6.1";

/// One platform's download: the asset, its sha256 and size, and the program inside its one top folder.
struct Pin {
    asset: &'static str,
    sha256: &'static str,
    size: u64,
    program: &'static str,
}

fn pin() -> R<Pin> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        // a universal app, ad-hoc signed
        ("macos", _) => Ok(Pin { asset: "macos-editor-app-4.6.1-jsc.zip", sha256: "8a3022cb07c9740b2acc4a0b6bba6be78fb2fe46ba172b13abd51e6829cdca95", size: 143_940_673, program: "Godot.app/Contents/MacOS/Godot" }),
        ("linux", "x86_64") => Ok(Pin { asset: "linux-editor-4.6.1-qjs-ng.zip", sha256: "daa91d6cff5862ed9e0b3776694fbe4ea0ebb474d244f46ffb3aa5af0318989d", size: 78_737_089, program: "godot.linuxbsd.editor.x86_64" }),
        ("windows", "x86_64") => Ok(Pin { asset: "windows-editor-4.6.1-qjs-ng.zip", sha256: "041b761a945b6a34d40f508dbd1accca462d943d2941c3a1a4568f84dd29c1d4", size: 83_915_786, program: "godot.windows.editor.x86_64.exe" }),
        (os, arch) => Err(format!("GodotJS has no build for {os} {arch}: build it (github.com/godotjs/GodotJS) and `git config --global raea.godot PATH`")),
    }
}

/// The release a download in own_dir came from (raea.json beside it), so a moved pin downloads again.
fn own_release() -> Option<String> {
    let text = std::fs::read_to_string(own_dir("godot").ok()?.join("raea.json")).ok()?;
    serde_json::from_str::<serde_json::Value>(&text).ok()?["release"].as_str().map(str::to_string)
}

/// The Godot to use and where it came from: `git config --global raea.godot` (a GodotJS build of your own), else
/// raea's download. A Godot without GodotJS isn't looked for: it can't run raea/godot's scripts.
pub fn find() -> Option<(PathBuf, &'static str)> {
    let configured = raea_app_tools::blender::run(Command::new("git").args(["config", "--global", "raea.godot"]), "git config").ok().map(|out| PathBuf::from(out.trim()));
    if let Some(p) = configured.filter(|p| p.is_file()) {
        return Some((p, "raea.godot in git config"));
    }
    let own = own_dir("godot").ok()?.join(pin().ok()?.program);
    own.is_file().then_some((own, "raea's download"))
}

/// The binary, or what to run to get one.
pub fn binary() -> R<PathBuf> {
    find().map(|(p, _)| p).ok_or_else(|| "no Godot with GodotJS: raea setup godot downloads it".to_string())
}

/// `godot --version`'s line ("4.6.1.stable.gh.14d19694e").
fn version(bin: &Path) -> R<String> {
    let o = Command::new(bin).args(["--headless", "--version"]).output().map_err(|e| format!("{}: {e}", bin.display()))?;
    let out = String::from_utf8_lossy(&o.stdout);
    out.lines().map(str::trim).find(|l| l.chars().next().is_some_and(|c| c.is_ascii_digit())).map(str::to_string).ok_or_else(|| format!("{} --version printed no version", bin.display()))
}

/// Downloads the pinned GodotJS editor into ~/.raea/tools/godot and returns its program.
fn download() -> R<(PathBuf, String)> {
    let p = pin()?;
    let dir = own_dir("godot")?;
    let tmp = dir.with_file_name("godot.download");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).map_err(|e| format!("{}: {e}", tmp.display()))?;
    let file = tmp.join(p.asset);
    let url = format!("https://github.com/godotjs/GodotJS/releases/download/{RELEASE}/{}", p.asset);
    raea_app_tools::blender::run(Command::new("curl").args(["-fL", "--retry", "3", "-o"]).arg(&file).arg(&url), "downloading Godot with GodotJS")?;
    let size = std::fs::metadata(&file).map_err(|e| e.to_string())?.len();
    if size != p.size || raea_app_tools::blender::sha256(&file)? != p.sha256 {
        return Err(format!("{} doesn't match the sha256 raea pins ({size} bytes); nothing installed (it's in {})", p.asset, tmp.display()));
    }
    let unpacked = tmp.join("unpacked");
    unzip(&file, &unpacked)?;
    let program = unpacked.join(p.program);
    if !program.is_file() {
        return Err(format!("{} holds no {}", p.asset, p.program));
    }
    // the zips keep no executable bits
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    }
    std::fs::write(unpacked.join("raea.json"), serde_json::json!({ "release": RELEASE, "asset": p.asset, "sha256": p.sha256 }).to_string()).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::rename(&unpacked, &dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let _ = std::fs::remove_dir_all(&tmp);
    Ok((dir.join(p.program), format!("downloaded GodotJS {RELEASE} ({}) from github.com/godotjs/GodotJS", p.asset)))
}

/// A zip's files into `to`, without its one top folder (every GodotJS zip has one).
fn unzip(file: &Path, to: &Path) -> R<()> {
    let f = std::fs::File::open(file).map_err(|e| format!("{}: {e}", file.display()))?;
    let mut z = zip::ZipArchive::new(f).map_err(|e| format!("{}: {e}", file.display()))?;
    for i in 0..z.len() {
        let mut e = z.by_index(i).map_err(|e| e.to_string())?;
        let Some(path) = e.enclosed_name() else { return Err(format!("{} has an entry outside its folder: {}", file.display(), String::from_utf8_lossy(e.name_raw()))) };
        let rel: PathBuf = path.components().skip(1).collect();
        if rel.as_os_str().is_empty() {
            continue;
        }
        let out = to.join(rel);
        if e.is_dir() {
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(d) = out.parent() {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        let mut w = std::fs::File::create(&out).map_err(|e| format!("{}: {e}", out.display()))?;
        std::io::copy(&mut e, &mut w).map_err(|e| format!("{}: {e}", out.display()))?;
    }
    Ok(())
}

/// `raea setup godot [--check]`.
pub fn setup(args: &[String]) -> R<String> {
    let check = args.iter().any(|a| a == "--check");
    let current = own_release().as_deref() == Some(RELEASE);
    let (bin, from) = match find() {
        Some((p, from)) if from != "raea's download" || current => (p, from.to_string()),
        _ if check => return Err(format!("no Godot with GodotJS {RELEASE}: raea setup godot downloads it")),
        _ => download()?,
    };
    let v = version(&bin)?;
    Ok(format!("APPLIED  Godot {v} with GodotJS at {} ({from}); raea godot run SCRIPT.ts runs a script against it", bin.display()))
}

/// The project folder: DIR, else the current folder; it needs its project.godot.
fn project(arg: Option<&String>) -> R<PathBuf> {
    let dir = arg.map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let dir = dir.canonicalize().map_err(|e| format!("{}: {e}", dir.display()))?;
    if !dir.join("project.godot").is_file() {
        return Err(format!("{} has no project.godot", dir.display()));
    }
    Ok(dir)
}

/// `raea godot types [DIR]`: GodotJS's codegen on the project, headless (--generate-types), into DIR/typings.
fn types(args: &[String]) -> R<String> {
    let dir = project(args.get(2))?;
    let o = Command::new(binary()?).args(["--headless", "--editor", "--path"]).arg(&dir).arg("--generate-types").output().map_err(|e| e.to_string())?;
    let log = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    if !o.status.success() || !log.contains("Type generation complete") {
        let tail: Vec<&str> = log.lines().rev().take(12).collect::<Vec<_>>().into_iter().rev().collect();
        return Err(format!("GodotJS's codegen failed on {}:\n{}", dir.display(), tail.join("\n")));
    }
    let mut n = 0;
    for e in std::fs::read_dir(dir.join("typings")).into_iter().flatten().flatten() {
        n += e.file_name().to_string_lossy().ends_with(".d.ts") as usize;
    }
    Ok(format!("APPLIED  {n} typings files in {} (Godot's API with the project's scenes, resources and input actions); a tsconfig that includes them types `import ... from \"godot\"`", dir.join("typings").display()))
}

/// `raea godot open [DIR]`: the editor on the project, left running.
fn open(args: &[String]) -> R<String> {
    let dir = project(args.get(2))?;
    Command::new(binary()?).args(["--editor", "--path"]).arg(&dir).spawn().map_err(|e| e.to_string())?;
    Ok(format!("APPLIED  the Godot editor (GodotJS) is opening {}", dir.display()))
}

/// `raea godot types|open ...` (run is godot_run::run).
pub fn godot(args: &[String]) -> R<String> {
    match args.get(1).map(String::as_str) {
        Some("types") => types(args),
        Some("open") => open(args),
        Some("run") => crate::godot_run::run(args),
        _ => Err(format!("usage: {USAGE}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn this_platform_has_a_pin_with_a_full_digest() {
        if let Ok(p) = pin() {
            assert_eq!(p.sha256.len(), 64);
            assert!(p.sha256.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
            assert!(p.asset.ends_with(".zip") && p.size > 50_000_000);
        }
    }

    #[test]
    fn unzip_drops_the_top_folder() {
        let base = std::env::temp_dir().join(format!("raea-godot-unzip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let zf = base.join("a.zip");
        let mut w = zip::ZipWriter::new(std::fs::File::create(&zf).unwrap());
        let o = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        w.add_directory("top/", o).unwrap();
        w.start_file("top/Godot.app/Contents/MacOS/Godot", o).unwrap();
        std::io::Write::write_all(&mut w, b"binary").unwrap();
        w.finish().unwrap();
        unzip(&zf, &base.join("out")).unwrap();
        assert_eq!(std::fs::read(base.join("out/Godot.app/Contents/MacOS/Godot")).unwrap(), b"binary");
        let _ = std::fs::remove_dir_all(&base);
    }
}
