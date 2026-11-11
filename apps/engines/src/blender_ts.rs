//! `raea blender run SCRIPT.ts [FILE.blend] [ARGS...]`: a headless Blender with raea's bridge (FILE.blend opened, ARGS
//! its sys.argv after --), the script on raea's engine with each batch posted to it (blender.post), Blender quit after.

use std::path::PathBuf;

use raea_runtime_calls::blender::{BRIDGE, post};
use serde_json::json;

use crate::{R, blender_bridge};

/// What the script's entry does after it: its default export called and awaited, then the session closed
/// (sdk/blender/src/blender.ts's __raea_blender_end), so a failure in Blender the script never awaited fails the run,
/// as an uncaught Python exception would.
const END: &str = "Promise.resolve().then(() => (typeof (M as any).default === \"function\" ? (M as any).default() : undefined)).then(() => (globalThis as any).__raea_blender_end?.()).then(() => call(\"done\", JSON.stringify({ code: 0 })), (e: any) => { call(\"print\", JSON.stringify({ text: \"error: \" + String(e) + (e && e.name === \"BlenderError\" ? (e.pythonTrace ? \"\\n\" + e.pythonTrace : \"\") : e && e.stack ? \"\\n\" + e.stack : \"\") })); call(\"done\", JSON.stringify({ code: 1 })); });";

pub fn run(args: &[String]) -> R<String> {
    let ts = PathBuf::from(&args[2]).canonicalize().map_err(|e| format!("{}: {e}", args[2]))?;
    let (bin, _) = raea_app_tools::blender::find().ok_or("no Blender: raea setup blender finds or downloads one")?;
    let file = args.get(3).filter(|a| a.ends_with(".blend")).map(PathBuf::from);
    // ARGS for Blender's sys.argv, without raea's own --timeout SECS
    let mut rest = args[if file.is_some() { 4 } else { 3 }..].to_vec();
    let mut secs = 3600.0;
    if let Some(i) = rest.iter().position(|a| a == "--timeout") {
        secs = rest.get(i + 1).and_then(|t| t.parse().ok()).unwrap_or(secs);
        rest.drain(i..(i + 2).min(rest.len()));
    }
    let bridge = blender_bridge::start(&bin, file.as_deref(), &rest)?;
    *BRIDGE.lock().map_err(|e| e.to_string())? = Some((bridge.port, bridge.token.clone()));
    let out = std::env::temp_dir().join(format!("raea-blender-{}.js", std::process::id()));
    let ran = raea_runtime_bundle::bundle_module(&ts, &out, END).and_then(|_| raea_runtime_export::run_bundle(&out, secs));
    let _ = std::fs::remove_file(&out);
    let _ = post(&json!({ "sid": "raea", "ops": [{ "o": "quit" }] }));
    *BRIDGE.lock().map_err(|e| e.to_string())? = None;
    bridge.wait();
    let (code, _) = ran?;
    if code != 0 {
        return Err(format!("{} ended with exit {code}", args[2]));
    }
    Ok(String::new())
}
