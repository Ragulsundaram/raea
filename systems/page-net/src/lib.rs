//! What a page loads (images, stylesheets, fonts): local files, data URLs and http(s), answered at once on the asking
//! thread. A composition's media are files beside it; a URL (an @import of Google Fonts, its faces) is fetched once
//! into raea's cache (raea_io_net::cache, Chrome's user agent for Google's stylesheets so they name
//! WOFF2 faces) and read from there after, so a render is the same frame for frame, and offline once cached.

use blitz_traits::net::{Bytes, NetHandler, NetProvider, Request};

mod host;
pub use host::fetch;

/// Files and data URLs; a path that isn't on disk is looked for under each of `roots` in turn (a server's root: WPT's
/// `/css/...`).
#[derive(Default)]
pub struct FileNet {
    pub roots: Vec<std::path::PathBuf>,
}

/// A file URL's path: `to_file_path`'s, else its decoded path as written, root-absolute, for the roots to find. On
/// Windows `to_file_path` refuses a path with no drive, which is every root-absolute URL a page names (`/media/pong.jpg`,
/// as `file:///media/pong.jpg`), so none of them was looked for under the page's roots, and an app's posters and
/// pictures were missing there alone
fn file_path(url: &url::Url) -> Option<std::path::PathBuf> {
    url.to_file_path().ok().or_else(|| written(url))
}

fn written(url: &url::Url) -> Option<std::path::PathBuf> {
    let p = percent_encoding::percent_decode_str(url.path()).decode_utf8().ok()?;
    p.starts_with('/').then(|| std::path::PathBuf::from(p.as_ref()))
}

#[cfg(test)]
mod tests {
    /// a root-absolute file URL's path as it was written, decoded, and relative to "/" for a root to join
    #[test]
    fn a_root_absolute_url_is_a_path_for_the_roots() {
        let p = super::written(&url::Url::parse("file:///media/a%20b.jpg").unwrap()).unwrap();
        assert_eq!(p, std::path::PathBuf::from("/media/a b.jpg"));
        assert_eq!(p.strip_prefix("/").unwrap(), std::path::Path::new("media/a b.jpg"));
    }

    /// "/artifact.js" from a page's file URL is the same file under a root on every OS: on Windows the URL keeps its
    /// drive, so the path is C:\artifact.js, which a leading "/" alone didn't take for root-absolute (#124)
    #[test]
    fn a_root_absolute_path_is_relative_after_its_root() {
        use std::path::Path;
        assert_eq!(super::root_relative(Path::new("/artifact.js")).unwrap(), Path::new("artifact.js"));
        assert_eq!(super::root_relative(Path::new("/media/a.jpg")).unwrap(), Path::new("media/a.jpg"));
        assert_eq!(super::root_relative(Path::new("media/a.jpg")), None);
        let page = url::Url::parse(if cfg!(windows) { "file:///C:/app/feed/page/index.html" } else { "file:///app/feed/page/index.html" }).unwrap();
        let script = page.join("/artifact.js").unwrap().to_file_path().unwrap();
        assert_eq!(super::root_relative(&script).unwrap(), Path::new("artifact.js"));
    }
}

/// RAEA_PAGE_ROOT's roots, a list as PATH is (an app's Blitz build, then its public/)
pub fn page_roots() -> Vec<std::path::PathBuf> {
    std::env::var_os("RAEA_PAGE_ROOT").map(|v| std::env::split_paths(&v).collect()).unwrap_or_default()
}

impl NetProvider for FileNet {
    fn fetch(&self, _doc_id: usize, request: Request, handler: Box<dyn NetHandler>) {
        let url = request.url;
        let bytes = match url.scheme() {
            "file" => file_path(&url).and_then(|p| {
                std::fs::read(&p).ok().or_else(|| {
                    let rel = root_relative(&p)?;
                    self.roots.iter().find_map(|r| std::fs::read(r.join(&rel)).ok())
                })
            }),
            "data" => raea_io_net::data_url(url.as_str()).map(|(_, b)| b),
            // the page's own origin (http://localhost/, as page_fetch has it): the file under the roots, confined, so an
            // @font-face, image or stylesheet the app names by its URL loads as in Chrome (Excalidraw's fonts did not)
            "http" if url.host_str() == Some("localhost") && url.port().is_none() && !self.roots.is_empty() => {
                let path = url::Url::parse(&format!("file://{}", url.path())).ok().and_then(|u| file_path(&u));
                path.and_then(|p| FrameNet { roots: self.roots.clone() }.confined(&p)).filter(|p| p.is_file()).and_then(|p| std::fs::read(p).ok())
            }
            "http" | "https" => match raea_io_net::cache::read(url.as_str()) {
                Ok(b) => Some(b),
                Err(e) => {
                    bevy::log::warn!("raea-page: {e}");
                    None
                }
            },
            _ => None,
        };
        // a load that fails still answers (empty), so the document doesn't wait for it
        if bytes.is_none() && (!matches!(url.scheme(), "http" | "https") || url.host_str() == Some("localhost")) {
            bevy::log::warn!("raea-page: {url} isn't a file, a data URL or a URL that reads");
        }
        if bytes.is_none() {
            // listed once more when the render ends, with the CSS the page dropped (#34)
            blitz_dom::not_applied::note(format!("the file {url} (it didn't load)"));
        }
        handler.bytes(url.to_string(), Bytes::from(bytes.unwrap_or_default()));
    }
}

/// What an <iframe>'s document and its loads come through (blitz-dom's FRAME_NET, set by `config` while
/// iframes are on): FileNet confined to `roots`. A file URL is read only when the file it names, links and `..`
/// resolved, is inside a root, a root-absolute path that isn't there looked for under each root as FileNet does; with
/// no root no file is read. Data URLs and http(s) are FileNet's (docs/blitz-bevy.feature B5c, iframes 6).
pub struct FrameNet {
    pub roots: Vec<std::path::PathBuf>,
}

impl FrameNet {
    /// `path`, or its root-absolute form under a root, if it resolves inside that root
    pub fn confined(&self, path: &std::path::Path) -> Option<std::path::PathBuf> {
        self.roots.iter().filter_map(|r| r.canonicalize().ok()).find_map(|root| {
            let at = |p: std::path::PathBuf| p.canonicalize().ok().filter(|c| c.starts_with(&root));
            at(path.to_path_buf()).or_else(|| at(root.join(root_relative(path)?)))
        })
    }
}

/// A root-absolute path's part after its root: "/artifact.js" is "artifact.js", and so is Windows' "C:\artifact.js",
/// which is what "/artifact.js" against a page at file:///C:/... comes to (a leading "/" alone missed it there, #124)
fn root_relative(path: &std::path::Path) -> Option<std::path::PathBuf> {
    use std::path::Component::{Prefix, RootDir};
    path.has_root().then(|| path.components().skip_while(|c| matches!(c, Prefix(_) | RootDir)).collect())
}

impl NetProvider for FrameNet {
    fn fetch(&self, doc_id: usize, request: Request, handler: Box<dyn NetHandler>) {
        if request.url.scheme() != "file" {
            return FileNet::default().fetch(doc_id, request, handler);
        }
        let url = request.url;
        let bytes = file_path(&url).and_then(|p| self.confined(&p)).and_then(|p| std::fs::read(p).ok());
        if bytes.is_none() {
            bevy::log::warn!("raea-page: a frame's {url} isn't a file inside the page's root");
        }
        handler.bytes(url.to_string(), Bytes::from(bytes.unwrap_or_default()));
    }
}

/// A media element's local source as a path: a file URL's path, and a root-absolute path that isn't on disk looked for
/// under RAEA_PAGE_ROOT, as FileNet looks for a page's other loads (an app's public/, served at "/").
pub fn page_path(src: &str) -> String {
    let path = src.strip_prefix("file://").unwrap_or(src);
    if let Some(rel) = path.strip_prefix('/')
        && !std::path::Path::new(path).exists()
        && let Some(under) = page_roots().into_iter().map(|r| r.join(rel)).find(|u| u.exists())
    {
        return under.to_string_lossy().into_owned();
    }
    path.to_string()
}

/// Blitz's image decoder (blitz-dom's IMAGE_DECODER): every format raea-runtime-js's page decodes, as it does. A data URL's
/// format is its MIME type's, a file's its extension's; SVG and what Bevy doesn't read fall to Blitz's own.
pub fn install_decoder() {
    let _ = blitz_dom::net::IMAGE_DECODER.set(|url, bytes| {
        let mime = match url.strip_prefix("data:") {
            Some(rest) => rest.split([';', ',']).next()?.to_string(),
            None => format!("image/{}", url.split(['?', '#']).next()?.rsplit_once('.')?.1.to_ascii_lowercase()),
        };
        if mime == "image/svg+xml" || mime == "image/svg" {
            return None;
        }
        let img = raea_sys_image::decode::decode(bytes, &mime)?;
        let size = img.size();
        Some((size.x, size.y, img.data?))
    });
}

/// A page's fetch of its own origin (sdk/ui's platform.ts sends fetch as the host call "net.fetch", and the page is
/// http://localhost/): the file under the page's roots (RAEA_PAGE_ROOT, FrameNet's confinement), as the server that
/// serves the page answers it, 404 when there's none; the app's `fetch("/media/x.mp3")` as in Chrome. None for any
/// other URL, or with no roots, which go out as raea-runtime-js's fetch does.
pub fn page_fetch(args: &serde_json::Value) -> Option<serde_json::Value> {
    use base64::Engine as _;
    let url = args["url"].as_str()?;
    let u = url::Url::parse(url).ok()?;
    let roots = page_roots();
    if u.scheme() != "http" || u.host_str() != Some("localhost") || u.port().is_some() || roots.is_empty() {
        return None;
    }
    // its path decoded, as a file URL's
    let path = file_path(&url::Url::parse(&format!("file://{}", u.path())).ok()?)?;
    // /media/x from the root's media/, then its assets/ (Bevy's assets folder: docs/sdk.feature S5a), confined either way
    let assets = path.strip_prefix("/media").ok().map(|rest| std::path::Path::new("/assets").join(rest));
    let net = FrameNet { roots };
    let found = if matches!(args["method"].as_str(), None | Some("GET" | "HEAD")) {
        net.confined(&path).filter(|p| p.is_file()).or_else(|| assets.and_then(|a| net.confined(&a)).filter(|p| p.is_file()))
    } else {
        None
    };
    Some(match found.and_then(|p| std::fs::read(&p).ok().map(|b| (p, b))) {
        Some((p, bytes)) => serde_json::json!({
            "status": 200, "statusText": "OK", "url": url, "redirected": false,
            "headers": [["content-type", mime(&p)], ["content-length", bytes.len().to_string()]],
            "body": base64::engine::general_purpose::STANDARD.encode(&bytes),
        }),
        None => serde_json::json!({ "status": 404, "statusText": "Not Found", "url": url, "redirected": false, "headers": [], "body": "" }),
    })
}

/// A file's content type by its extension, as a static server sends it.
fn mime(p: &std::path::Path) -> &'static str {
    match p.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref() {
        Some("html" | "htm") => "text/html",
        Some("css") => "text/css",
        Some("js" | "mjs") => "text/javascript",
        Some("json") => "application/json",
        Some("txt") => "text/plain",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("ogg" | "oga") => "audio/ogg",
        Some("flac") => "audio/flac",
        Some("m4a") => "audio/mp4",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        _ => "application/octet-stream",
    }
}
