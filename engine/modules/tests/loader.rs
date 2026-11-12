use std::path::PathBuf;

use raea_modules::{Loader, ModuleId, Prebuilt, RefusalKind};

fn space(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let d = std::env::temp_dir().join(format!("raea-modules-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    for (p, s) in files {
        let f = d.join(p);
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(f, s).unwrap();
    }
    d
}

fn libs() -> Vec<Prebuilt> {
    vec![
        Prebuilt { specifier: "react".into(), exports: vec!["default".into(), "useState".into()] },
        Prebuilt { specifier: "react/jsx-runtime".into(), exports: vec!["jsx".into(), "jsxs".into(), "Fragment".into()] },
        Prebuilt { specifier: "raea".into(), exports: vec!["Sequence".into(), "useCurrentFrame".into()] },
    ]
}

#[test]
fn tsx_uses_reacts_automatic_runtime_and_drops_types() {
    let d = space("tsx", &[("components/card.tsx", "type P = { big: string };\nexport default function Card({ big }: P) { return <div className=\"p-4\">{big}</div>; }\n")]);
    let mut l = Loader::new(&d, libs()).unwrap();
    let id = l.resolve("./components/card", &d.join("edit.tsx").display().to_string()).unwrap();
    let m = l.load(&id).unwrap();
    assert!(m.source.contains("react/jsx-runtime"), "{}", m.source);
    assert!(!m.source.contains("type P"), "types are gone: {}", m.source);
    assert!(m.source_map.is_some());
}

#[test]
fn imports_resolve_inside_the_space_and_refuse_outside() {
    let d = space("resolve", &[("edit.tsx", ""), ("parts/index.ts", "export const x = 1;"), ("a/b.ts", "")]);
    let l = Loader::new(&d, libs()).unwrap();
    let entry = d.join("edit.tsx").display().to_string();
    assert_eq!(l.resolve("./parts", &entry).unwrap(), ModuleId::File(d.canonicalize().unwrap().join("parts/index.ts")));
    let b = d.canonicalize().unwrap().join("a/b.ts").display().to_string();
    assert!(matches!(l.resolve("../edit", &b).unwrap(), ModuleId::File(_)));
    let out = l.resolve("../../etc/passwd", &entry).unwrap_err().message;
    assert!(out.contains("outside the space"), "{out}");
    let missing = l.resolve("./nope", &entry).unwrap_err().message;
    assert!(missing.contains("isn't there"), "{missing}");
    let r = l.resolve("lodash", &entry).unwrap_err();
    assert_eq!((r.kind, r.specifier.as_deref(), r.referrer.as_deref()), (RefusalKind::UnknownLibrary, Some("lodash"), Some(entry.as_str())));
    let bare = r.message;
    assert!(bare.contains("raea, react, react/jsx-runtime"), "names what it has: {bare}");
}

#[test]
fn prebuilt_libraries_reexport_from_the_engines_table() {
    let d = space("prebuilt", &[]);
    let mut l = Loader::new(&d, libs()).unwrap();
    let m = l.load(&l.resolve("react", "file:///x.tsx").unwrap()).unwrap();
    assert!(m.source.contains("globalThis.__raea_modules[\"react\"]") && m.source.contains("export default m.default") && m.source.contains("export const { useState } = m"), "{}", m.source);
}

#[test]
fn media_imports_name_their_file() {
    let d = space("media", &[("media/take01.mov", "x")]);
    let mut l = Loader::new(&d, libs()).unwrap();
    let id = l.resolve("./media/take01.mov", &d.join("edit.tsx").display().to_string()).unwrap();
    assert!(l.load(&id).unwrap().source.starts_with("export default \"") );
}

#[test]
fn an_edit_retranspiles_only_what_changed() {
    let d = space("cache", &[("a.tsx", "export const A = () => <b>a</b>;"), ("b.tsx", "export const B = () => <i>b</i>;")]);
    let mut l = Loader::new(&d, libs()).unwrap();
    let r = d.join("edit.tsx").display().to_string();
    let (a, b) = (l.resolve("./a", &r).unwrap(), l.resolve("./b", &r).unwrap());
    l.load(&a).unwrap();
    l.load(&b).unwrap();
    assert_eq!(l.transpiled, 2);
    l.load(&a).unwrap();
    std::fs::write(d.join("b.tsx"), "export const B = () => <i>changed</i>;").unwrap();
    let m = l.load(&b).unwrap();
    assert_eq!(l.transpiled, 3, "a came from the cache, b was re-transpiled");
    assert!(m.source.contains("changed"));
}

#[test]
fn a_parse_error_says_where() {
    let d = space("bad", &[("bad.tsx", "export const X = () => <div>;\n")]);
    let mut l = Loader::new(&d, libs()).unwrap();
    let id = l.resolve("./bad", &d.join("edit.tsx").display().to_string()).unwrap();
    let e = l.load(&id).unwrap_err().message;
    assert!(e.contains("bad.tsx") && e.contains("can't be parsed"), "{e}");
}


#[test]
fn shared_imports_resolve_to_the_workspaces_shared_folder() {
    // docs/layout.feature LY2: a project imports its workspace's shared/ by @shared/, and a shared file its neighbours
    let w = space("shared", &[
        (".raea/workspace", "[workspace]\n\tname = W\n"),
        ("shared/components/desk.tsx", "import { leg } from \"./leg\";\nexport default () => leg;"),
        ("shared/components/leg.ts", "export const leg = 1;"),
        ("shared/components/props/index.ts", "export const p = 1;"),
        ("projects/a/compositions/main.tsx", ""),
    ]);
    let root = w.canonicalize().unwrap();
    let l = Loader::new(&w.join("projects/a"), libs()).unwrap();
    let entry = root.join("projects/a/compositions/main.tsx").display().to_string();
    assert_eq!(l.resolve("@shared/components/desk", &entry).unwrap(), ModuleId::File(root.join("shared/components/desk.tsx")));
    assert_eq!(l.resolve("@shared/components/props", &entry).unwrap(), ModuleId::File(root.join("shared/components/props/index.ts")));
    let desk = root.join("shared/components/desk.tsx").display().to_string();
    assert_eq!(l.resolve("./leg", &desk).unwrap(), ModuleId::File(root.join("shared/components/leg.ts")));
    // a relative climb to another project is still outside, and says @shared/
    let other = l.resolve("../../b/x", &entry).unwrap_err().message;
    assert!(other.contains("outside the space") && other.contains("@shared/"), "{other}");
    assert!(l.resolve("@shared/../projects/b/x", &entry).unwrap_err().message.contains("climbs out"));
    assert!(l.resolve("@shared/components/nope", &entry).unwrap_err().message.contains("isn't there"));
    // with no workspace above, @shared/ says so
    let loose = space("shared-loose", &[("edit.tsx", "")]);
    let l = Loader::new(&loose, libs()).unwrap();
    let e = loose.join("edit.tsx").display().to_string();
    let r = l.resolve("@shared/components/desk", &e).unwrap_err();
    assert!(r.kind == RefusalKind::UnknownLibrary && r.message.contains("in no workspace"), "{}", r.message);
}
