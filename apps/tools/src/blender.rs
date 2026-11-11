//! `raea setup blender` and `raea blender run` (docs/skills.feature SK11). Blender is GPL, so raea never bundles it: it
//! finds the person's Blender, or downloads the current LTS from blender.org into ~/.raea/tools/blender, and runs it as
//! its own program. The setup also installs the MCP for Blender add-on (github.com/ahujasid/mcp-for-blender, MIT) into
//! that Blender, so an agent's `blender` MCP server (raea skills install adds it) can drive a running Blender.
use std::path::{Path, PathBuf};
use std::process::Command;

type R<T> = Result<T, String>;

pub const USAGE: &str = "raea setup blender [--check] [--download]   (find Blender or download the LTS into ~/.raea/tools/blender, and install its MCP add-on)\n       raea blender run SCRIPT.py [ARGS...]   (blender --background --python SCRIPT.py -- ARGS, headless; blender/X.py missing from the project is copied from the library: convert, decimate, repair, split, bake, rig, cloth)\n       raea blender run SCRIPT.ts [FILE.blend] [ARGS...]   (a TypeScript script against Blender's whole Python API through raea/blender: bpy, bmesh, mathutils, typed; a headless Blender, FILE.blend opened, quit when the script ends)";

/// The LTS series raea downloads; the newest patch in blender.org/release/Blender<LTS>/ is taken. Moved when Blender
/// ships a new LTS (blender.org/download/lts).
const LTS: &str = "5.2";

/// Where raea keeps a tool it downloads, ~/.raea/tools/NAME: Blender here, Godot in apps/engines.
pub fn own_dir(name: &str) -> R<PathBuf> {
    Ok(crate::agent_mcp::user_home()?.join(".raea/tools").join(name))
}

/// The Blender binary inside raea's own download, per platform.
fn own_binary() -> R<PathBuf> {
    let d = own_dir("blender")?;
    Ok(if cfg!(target_os = "macos") { d.join("Blender.app/Contents/MacOS/Blender") } else if cfg!(windows) { d.join("blender.exe") } else { d.join("blender") })
}

fn on_path(name: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?).map(|d| d.join(name)).find(|p| p.is_file())
}

/// The Blender to use and where it came from: `git config --global raea.blender`, raea's own download, the usual app
/// folders, then PATH.
pub fn find() -> Option<(PathBuf, &'static str)> {
    let configured = run(Command::new("git").args(["config", "--global", "raea.blender"]), "git config").ok().map(|out| PathBuf::from(out.trim()));
    if let Some(p) = configured.filter(|p| p.is_file()) {
        return Some((p, "raea.blender in git config"));
    }
    if let Some(p) = own_binary().ok().filter(|p| p.is_file()) {
        return Some((p, "raea's download"));
    }
    let mut apps = vec![PathBuf::from("/Applications/Blender.app/Contents/MacOS/Blender")];
    if let Ok(h) = crate::agent_mcp::user_home() {
        apps.push(h.join("Applications/Blender.app/Contents/MacOS/Blender"));
    }
    if let Some(p) = apps.into_iter().find(|p| cfg!(target_os = "macos") && p.is_file()) {
        return Some((p, "Applications"));
    }
    on_path(if cfg!(windows) { "blender.exe" } else { "blender" }).map(|p| (p, "PATH"))
}

/// `blender --version`'s first line ("Blender 5.2.1 LTS").
fn version(bin: &Path) -> R<String> {
    let o = Command::new(bin).arg("--version").output().map_err(|e| format!("{}: {e}", bin.display()))?;
    String::from_utf8_lossy(&o.stdout).lines().find(|l| l.starts_with("Blender ")).map(str::to_string).ok_or_else(|| format!("{} --version printed no version", bin.display()))
}

pub fn run(cmd: &mut Command, what: &str) -> R<String> {
    let o = cmd.output().map_err(|e| format!("{what}: {e}"))?;
    if !o.status.success() {
        return Err(format!("{what} failed: {}{}", String::from_utf8_lossy(&o.stderr).trim(), String::from_utf8_lossy(&o.stdout).trim()));
    }
    Ok(String::from_utf8_lossy(&o.stdout).into_owned())
}

/// This platform's file name in a release folder, for a version.
fn asset(v: &str) -> R<String> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Ok(format!("blender-{v}-macos-arm64.dmg")),
        ("macos", _) => Ok(format!("blender-{v}-macos-x64.dmg")),
        ("linux", "x86_64") => Ok(format!("blender-{v}-linux-x64.tar.xz")),
        ("windows", "aarch64") => Ok(format!("blender-{v}-windows-arm64.zip")),
        ("windows", _) => Ok(format!("blender-{v}-windows-x64.zip")),
        (os, arch) => Err(format!("blender.org has no build for {os} {arch}: install Blender yourself and `git config --global raea.blender PATH`")),
    }
}

/// The newest patch of the LTS series in a release folder's listing.
fn newest(listing: &str) -> Option<String> {
    let pre = format!("blender-{LTS}.");
    let mut best: Option<(u32, String)> = None;
    for part in listing.split(&pre).skip(1) {
        let digits: String = part.chars().take_while(char::is_ascii_digit).collect();
        if let Ok(n) = digits.parse::<u32>() {
            if part[digits.len()..].starts_with('-') && best.as_ref().map_or(true, |(b, _)| n > *b) {
                best = Some((n, format!("{LTS}.{n}")));
            }
        }
    }
    best.map(|(_, v)| v)
}

pub fn sha256(file: &Path) -> R<String> {
    let out = if cfg!(windows) {
        run(Command::new("certutil").arg("-hashfile").arg(file).arg("SHA256"), "certutil")?.lines().nth(1).unwrap_or_default().replace(' ', "")
    } else {
        run(Command::new("shasum").args(["-a", "256"]).arg(file), "shasum")?.split_whitespace().next().unwrap_or_default().to_string()
    };
    Ok(out.to_lowercase())
}

/// Downloads the LTS into ~/.raea/tools/blender, checked against blender.org's sha256 list, and returns its binary.
fn download() -> R<(PathBuf, String)> {
    let base = format!("https://download.blender.org/release/Blender{LTS}/");
    let listing = run(Command::new("curl").args(["-fsSL", &base]), "curl")?;
    let v = newest(&listing).ok_or_else(|| format!("no Blender {LTS} release listed at {base}"))?;
    let name = asset(&v)?;
    let dir = own_dir("blender")?;
    let tmp = dir.with_file_name("blender.download");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).map_err(|e| format!("{}: {e}", tmp.display()))?;
    let file = tmp.join(&name);
    run(Command::new("curl").args(["-fL", "--retry", "3", "-o"]).arg(&file).arg(format!("{base}{name}")), "downloading Blender")?;
    let sums = run(Command::new("curl").args(["-fsSL", &format!("{base}blender-{v}.sha256")]), "curl")?;
    let want = sums.lines().find(|l| l.ends_with(&name)).and_then(|l| l.split_whitespace().next()).ok_or_else(|| format!("blender-{v}.sha256 doesn't list {name}"))?.to_lowercase();
    if sha256(&file)? != want {
        return Err(format!("{name} doesn't match blender.org's sha256; nothing installed (it's in {})", tmp.display()));
    }
    let unpacked = tmp.join("unpacked");
    std::fs::create_dir_all(&unpacked).map_err(|e| e.to_string())?;
    if name.ends_with(".dmg") {
        let mnt = tmp.join("mnt");
        run(Command::new("hdiutil").args(["attach", "-nobrowse", "-readonly", "-mountpoint"]).arg(&mnt).arg(&file), "hdiutil attach")?;
        let copied = run(Command::new("ditto").arg(mnt.join("Blender.app")).arg(unpacked.join("Blender.app")), "copying Blender.app");
        let _ = Command::new("hdiutil").args(["detach", "-quiet"]).arg(&mnt).status();
        copied?;
    } else {
        // tar unpacks .tar.xz on Linux and .zip on Windows 10 and 11; both hold one top folder
        run(Command::new("tar").arg("-xf").arg(&file).arg("-C").arg(&unpacked).args(if cfg!(windows) { vec![] } else { vec!["--strip-components=1"] }), "unpacking Blender")?;
        if cfg!(windows) {
            let top = std::fs::read_dir(&unpacked).map_err(|e| e.to_string())?.flatten().find(|e| e.path().is_dir()).ok_or("the zip held no folder")?.path();
            let flat = tmp.join("flat");
            std::fs::rename(&top, &flat).map_err(|e| e.to_string())?;
            std::fs::remove_dir_all(&unpacked).map_err(|e| e.to_string())?;
            std::fs::rename(&flat, &unpacked).map_err(|e| e.to_string())?;
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::rename(&unpacked, &dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let _ = std::fs::remove_dir_all(&tmp);
    Ok((own_binary()?, format!("downloaded Blender {v} LTS from blender.org")))
}

/// Python run inside Blender (background): prints the user add-ons folder, or whether the add-on is enabled.
const ADDONS_DIR: &str = "import bpy; print('RAEA_ADDONS=' + bpy.utils.user_resource('SCRIPTS', path='addons', create=True))";
const ENABLE: &str = "import addon_utils, bpy; addon_utils.enable('blender_mcp', default_set=True, persistent=True); bpy.ops.wm.save_userpref(); print('RAEA_ENABLED=' + str(addon_utils.check('blender_mcp')[1]))";
const CHECK: &str = "import addon_utils; print('RAEA_ENABLED=' + str(addon_utils.check('blender_mcp')[1]))";

fn py(bin: &Path, code: &str, key: &str) -> R<String> {
    let out = run(Command::new(bin).args(["--background", "--factory-startup", "--python-expr", code]), "blender")?;
    out.lines().find_map(|l| l.strip_prefix(key)).map(str::to_string).ok_or_else(|| format!("blender printed no {key}"))
}

/// The add-on: uvx mcp-for-blender's own installer writes it into this Blender's add-ons folder, then Blender enables it.
fn addon(bin: &Path, check: bool) -> R<String> {
    // --factory-startup skips the person's add-ons for speed, so the check reads the saved preferences instead
    let state = |b: &Path| run(Command::new(b).args(["--background", "--python-expr", CHECK]), "blender").map(|o| o.contains("RAEA_ENABLED=True"));
    if state(bin)? {
        return Ok("MCP add-on enabled".into());
    }
    if check {
        return Ok("MCP add-on not installed: raea setup blender installs it".into());
    }
    let uvx = on_path(if cfg!(windows) { "uvx.exe" } else { "uvx" }).ok_or("the MCP add-on comes from uv's uvx: install uv (https://docs.astral.sh/uv/getting-started/installation/), then run raea setup blender again")?;
    let dir = py(bin, ADDONS_DIR, "RAEA_ADDONS=")?;
    run(Command::new(&uvx).args(["mcp-for-blender", "install-addon", "--addons-dir", &dir]), "uvx mcp-for-blender install-addon")?;
    let on = run(Command::new(bin).args(["--background", "--python-expr", ENABLE]), "blender")?;
    if !on.contains("RAEA_ENABLED=True") {
        return Err(format!("the add-on is in {dir} but Blender didn't enable it: enable \"MCP for Blender\" in Preferences > Add-ons"));
    }
    Ok(format!("MCP add-on installed now ({dir})"))
}

/// `raea setup blender [--check] [--download]`.
pub fn setup(args: &[String]) -> R<String> {
    let check = args.iter().any(|a| a == "--check");
    let fresh = args.iter().any(|a| a == "--download");
    let (bin, from) = match find().filter(|_| !fresh) {
        Some((b, f)) => (b, f.to_string()),
        None if check => return Ok(format!("CHECKED  no Blender found (raea.blender, {}, PATH): raea setup blender downloads Blender {LTS} LTS", own_dir("blender")?.display())),
        None => {
            let (b, f) = download()?;
            run(Command::new("git").args(["config", "--global", "raea.blender"]).arg(&b), "git config")?;
            (b, f)
        }
    };
    let v = version(&bin)?;
    let add = addon(&bin, check)?;
    let head = if check { "CHECKED " } else { "APPLIED " };
    Ok(format!("{head} {v} ({from})\n  {}\n  {add}\n  headless: raea blender run SCRIPT.py; live: start Blender and the agent's `blender` MCP server connects (raea skills install adds it)", bin.display()))
}

/// The library's scripts (sdk/guides/craft/bevy/blender: convert, decimate, repair, split, bake, rig, cloth): a fix
/// names `blender/X.py`, and the first run copies the library's X.py into the project's blender/ folder, where it is
/// kept with the project and edited for it (docs/blender.feature, Background). A script already there is never replaced.
fn library(script: &str) -> R<()> {
    let p = Path::new(script);
    let (Some(name), Some(dir)) = (p.file_name(), p.parent()) else { return Ok(()) };
    if p.exists() || dir.file_name().is_none_or(|d| d != "blender") {
        return Ok(());
    }
    let from = raea_io_home::home().join("sdk/guides/craft/bevy/blender").join(name);
    if !from.is_file() {
        return Ok(());
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    std::fs::copy(&from, p).map_err(|e| format!("{}: {e}", p.display()))?;
    eprintln!("copied the library's {} into {} (it is the project's now: edit it there)", name.to_string_lossy(), dir.display());
    Ok(())
}

/// `raea blender run SCRIPT.py [ARGS...]`: the script in a windowless Blender, its args after `--` (sys.argv), its
/// output passed through. Exits as Blender's Python did: --python-exit-code makes an uncaught error fail the run.
pub fn blender(args: &[String]) -> R<String> {
    if args.get(1).map(String::as_str) != Some("run") {
        return Err(format!("usage: {USAGE}"));
    }
    let script = args.get(2).ok_or_else(|| format!("which script: {USAGE}"))?;
    library(script)?;
    if !Path::new(script).is_file() {
        return Err(format!("no script {script}"));
    }
    let (bin, _) = find().ok_or("no Blender: raea setup blender finds or downloads one")?;
    let st = Command::new(&bin).args(["--background", "--factory-startup", "--python-exit-code", "1", "--python", script, "--"]).args(&args[3..]).status().map_err(|e| format!("{}: {e}", bin.display()))?;
    if !st.success() {
        return Err(format!("{script} failed in Blender ({st})"));
    }
    Ok(String::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newest_takes_the_highest_patch_of_the_lts() {
        let listing = format!("<a href=\"blender-{LTS}.0-linux-x64.tar.xz\"> <a href=\"blender-{LTS}.12-macos-arm64.dmg\"> <a href=\"blender-{LTS}.3-windows-x64.zip\"> blender-{LTS}.9.sha256 blender-{LTS}.99beta");
        assert_eq!(newest(&listing), Some(format!("{LTS}.12")));
        assert_eq!(newest("nothing here"), None);
    }

    #[test]
    fn every_supported_platform_names_one_file() {
        assert!(asset("5.2.1").unwrap().starts_with("blender-5.2.1-"));
    }
}
