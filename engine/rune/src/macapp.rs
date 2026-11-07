//! A TSX app as a Mac app (R7b): `raea-rune-host bundle app.js [-o Name.app]`.
//!
//! The .app holds the app's script (Contents/Resources/app.js) and this host (Contents/MacOS/<name>), which, started
//! with no arguments from inside an .app, opens that script in a window (`bundled`). The host signs it ad hoc, so it
//! opens on the Mac that made it; `raea app bundle` then signs it as its developer asks (apps/targets sign.rs:
//! `--identity`, `--notarize`, docs/targets.feature T1d), as it does a web app. The app's fonts go with it (T3d): each
//! face the host gathered at the start, as TrueType in Resources/faces, and Resources/fonts.css naming them, which the
//! .app gathers when it starts. Media the app loads by a relative path stays where it was (it's reported if missing).

use std::path::{Path, PathBuf};
use std::process::Command;
/// The bundle inside the .app this host runs from, if it does.
pub fn bundled() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let js = exe.parent()?.parent()?.join("Resources/app.js");
    (exe.parent()?.ends_with("Contents/MacOS") && js.is_file()).then_some(js)
}

fn run(cmd: &mut Command) -> Result<String, String> {
    let o = cmd.output().map_err(|e| format!("{cmd:?}: {e}"))?;
    let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    if !o.status.success() {
        return Err(format!("{:?} failed: {}", cmd.get_program(), text.trim()));
    }
    Ok(text)
}

/// `src` (the app's script) as `out`, an app named `name` with window size `size` and the faces `fonts`, signed ad hoc.
/// What codesign said.
pub fn make(src: &str, name: &str, size: (u32, u32), out: &Path, fonts: &[raea_rune_media::fonts::Face]) -> Result<Vec<String>, String> {
    let _ = std::fs::remove_dir_all(out);
    let contents = out.join("Contents");
    let (macos, res) = (contents.join("MacOS"), contents.join("Resources"));
    for d in [&macos, &res] {
        std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    std::fs::write(res.join("app.js"), src).map_err(|e| e.to_string())?;
    std::fs::write(res.join("size"), format!("{}x{}", size.0, size.1)).map_err(|e| e.to_string())?;
    if !fonts.is_empty() {
        let faces = res.join("faces");
        std::fs::create_dir_all(&faces).map_err(|e| format!("{}: {e}", faces.display()))?;
        let mut css = String::new();
        for (i, f) in fonts.iter().enumerate() {
            std::fs::write(faces.join(format!("{i}.ttf")), &f.bytes).map_err(|e| e.to_string())?;
            // one rule a face, under the name the app gave it (the face's own name comes from the file)
            let n = f.names.first().map_or("", String::as_str);
            css.push_str(&format!("@font-face {{ font-family: \"{n}\"; src: url(\"faces/{i}.ttf\"); }}\n"));
        }
        std::fs::write(res.join("fonts.css"), css).map_err(|e| e.to_string())?;
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    std::fs::copy(&exe, macos.join(name)).map_err(|e| format!("{}: {e}", exe.display()))?;
    let id: String = name.chars().filter(char::is_ascii_alphanumeric).collect::<String>().to_lowercase();
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>{name}</string>
<key>CFBundleIdentifier</key><string>io.raea.rune.{id}</string>
<key>CFBundleName</key><string>{name}</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>0.1.0</string>
<key>CFBundleVersion</key><string>1</string>
<key>LSMinimumSystemVersion</key><string>13.0</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
"#
    );
    std::fs::write(contents.join("Info.plist"), plist).map_err(|e| e.to_string())?;

    // off the Mac the same folder runs as is (`bundled` finds its script), and codesign is the Mac's, as for a web app
    if !cfg!(target_os = "macos") {
        return Ok(vec!["not signed: codesign is the Mac's".into()]);
    }
    run(Command::new("codesign").args(["--force", "--deep", "--sign", "-"]).arg(out))?;
    Ok(vec!["signed ad hoc: it opens on the Mac that made it".into()])
}
