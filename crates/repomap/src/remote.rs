//! Shared maps over HTTP: a `https://review.repomap.sylphx.com/m/{owner}/{repo}`
//! link is read as a map the server built from source it already holds. This
//! module fetches it (conditional requests, a disk cache) and maps the server's
//! answers to what an agent can act on. It never sees source code: the
//! document is the source-free `SharedMap`.

use crate::login;
use repomap_core::shared::{self, Link, SharedMap};
use repomap_core::Index;
use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// The default review host; `REPOMAP_REVIEW_URL` overrides it.
pub const DEFAULT_BASE: &str = "https://review.repomap.sylphx.com";
/// How long a map is trusted before the server is asked again.
const FRESH: Duration = Duration::from_secs(60);
const TIMEOUT: Duration = Duration::from_secs(60);
const MAX_BODY: u64 = 512 * 1024 * 1024;

/// Where links point and how to authenticate. Built once per call from the
/// environment; tests pass their own.
#[derive(Clone)]
pub struct Config {
    /// `scheme://host[:port]` of the review server. Links to any other host
    /// are refused, so a session token is only ever sent here.
    pub base: String,
    pub token: Option<String>,
    pub cache_dir: PathBuf,
    pub fresh: Duration,
}

impl Config {
    pub fn from_env() -> Config {
        Config {
            base: login::review_base(),
            token: login::token(),
            cache_dir: repomap_core::index::cache_root().join("shared"),
            fresh: FRESH,
        }
    }

    /// The link, when it names this config's review host.
    pub fn link(&self, s: &str) -> Result<Link, String> {
        let link = shared::parse_link(s)
            .ok_or_else(|| format!("`{s}` is not a shared-map link; expected {}/m/{{owner}}/{{repo}}", self.base))?;
        if link.base != self.base {
            return Err(format!("`{s}` is not on the repomap review server {}; a shared map link looks like {}/m/{{owner}}/{{repo}}", self.base, self.base));
        }
        Ok(link)
    }
}

/// Why a map could not be read, in the terms the agent is told.
#[derive(Debug, Clone, PartialEq)]
pub enum Fail {
    /// 401: not signed in, or the session expired.
    Unauthorized,
    /// 402: the publisher's Team is not active.
    TeamInactive,
    /// 404: no such repository, or one the user cannot read (the same answer).
    NotFound,
    /// 202: the server is building the map.
    Building(u64),
    Other(String),
}

impl Fail {
    pub fn message(&self) -> String {
        match self {
            Fail::Unauthorized => "not signed in to repomap (or the session expired): run `repomap login`".into(),
            Fail::TeamInactive => "the repomap Team plan behind this map is not active".into(),
            Fail::NotFound => "no map you can read at this link".into(),
            Fail::Building(s) => format!("the map is building, retry in {s} s"),
            Fail::Other(m) => m.clone(),
        }
    }
}

struct Held {
    index: Arc<Index>,
    etag: Option<String>,
    checked: Instant,
}

fn held() -> &'static Mutex<HashMap<String, Held>> {
    static HELD: OnceLock<Mutex<HashMap<String, Held>>> = OnceLock::new();
    HELD.get_or_init(Default::default)
}

#[cfg(test)]
pub(crate) fn forget_memory() {
    held().lock().unwrap().clear();
}

fn slug(base: &str) -> String {
    base.split("://").nth(1).unwrap_or(base).replace([':', '/'], "_")
}

/// The map behind `link`, from memory, the disk cache or the server.
pub fn load(cfg: &Config, link: &Link) -> Result<Arc<Index>, Fail> {
    let key = format!("{}/{}/{}", cfg.base, link.owner, link.repo);
    let dir = cfg.cache_dir.join(slug(&cfg.base)).join(&link.owner).join(&link.repo);
    // The token is part of the identity: another user must not read a map
    // cached for this one.
    let who = cfg.token.clone().unwrap_or_default();
    let key = format!("{key}\u{0}{who}");
    let mut etag = None;
    let mut in_memory = None;
    if let Some(h) = held().lock().unwrap().get(&key) {
        if h.checked.elapsed() < cfg.fresh {
            return Ok(h.index.clone());
        }
        etag = h.etag.clone();
        in_memory = Some(h.index.clone());
    }
    let disk_map = dir.join("map.json");
    let disk_tag = dir.join("etag");
    if etag.is_none() {
        etag = std::fs::read_to_string(&disk_tag).ok().filter(|_| disk_map.is_file());
    }
    let cached = |etag: Option<String>| -> Option<Arc<Index>> {
        let m: SharedMap = serde_json::from_slice(&std::fs::read(&disk_map).ok()?).ok()?;
        let index = Arc::new(Index::from_shared(m, &link.owner, &link.repo).ok()?);
        held().lock().unwrap().insert(key.clone(), Held { index: index.clone(), etag, checked: Instant::now() });
        Some(index)
    };
    let url = format!("{}/v1/repos/{}/{}/map", cfg.base, link.owner, link.repo);
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .http_status_as_error(false)
        .max_redirects(0)
        .user_agent(concat!("repomap/", env!("CARGO_PKG_VERSION")))
        .build()
        .into();
    let mut req = agent.get(&url).header("accept", shared::MEDIA_TYPE);
    if let Some(t) = &cfg.token {
        req = req.header("authorization", &format!("Bearer {t}"));
    }
    if let Some(e) = &etag {
        req = req.header("if-none-match", e);
    }
    let mut res = match req.call() {
        Ok(r) => r,
        // Offline or the server is down: a map already on disk still answers.
        Err(e) => return in_memory.or_else(|| cached(etag)).ok_or_else(|| Fail::Other(format!("cannot reach {}: {e}", cfg.base))),
    };
    let status = res.status().as_u16();
    let header = |name: &str| res.headers().get(name).and_then(|v| v.to_str().ok()).map(String::from);
    match status {
        200 => {
            let new_tag = header("etag");
            let mut body = Vec::new();
            res.body_mut()
                .with_config()
                .limit(MAX_BODY)
                .reader()
                .read_to_end(&mut body)
                .map_err(|e| Fail::Other(format!("reading the map failed: {e}")))?;
            let m: SharedMap = serde_json::from_slice(&body).map_err(|e| Fail::Other(format!("the server sent a map this repomap cannot read: {e}")))?;
            let index = Arc::new(Index::from_shared(m, &link.owner, &link.repo).map_err(Fail::Other)?);
            if std::fs::create_dir_all(&dir).is_ok() {
                // Written whole: a map is replaced, never half-written.
                let tmp = dir.join("map.json.tmp");
                if std::fs::write(&tmp, &body).is_ok() && std::fs::rename(&tmp, &disk_map).is_ok() {
                    match &new_tag {
                        Some(t) => {
                            let _ = std::fs::write(&disk_tag, t);
                        }
                        None => {
                            let _ = std::fs::remove_file(&disk_tag);
                        }
                    }
                }
            }
            held().lock().unwrap().insert(key, Held { index: index.clone(), etag: new_tag, checked: Instant::now() });
            Ok(index)
        }
        304 if in_memory.is_some() => {
            let index = in_memory.unwrap();
            held().lock().unwrap().insert(key, Held { index: index.clone(), etag, checked: Instant::now() });
            Ok(index)
        }
        304 => cached(etag.clone()).ok_or_else(|| Fail::Other("the server said the cached map is current, but there is none; try again".into())),
        202 => Err(Fail::Building(header("retry-after").and_then(|v| v.trim().parse().ok()).unwrap_or(10))),
        401 => Err(Fail::Unauthorized),
        402 => Err(Fail::TeamInactive),
        404 => {
            // Access lost or the repo is gone: do not keep serving its map.
            held().lock().unwrap().remove(&key);
            let _ = std::fs::remove_dir_all(&dir);
            Err(Fail::NotFound)
        }
        s => Err(Fail::Other(format!("the repomap review server answered {s}"))),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::fake::*;
    use super::*;

    /// A map of a small TypeScript repo, as the server would send it.
    pub(crate) fn sample_map(files: &[(&str, &str)], commit: &str) -> String {
        let d = tempfile::tempdir().unwrap();
        for (p, body) in files {
            let path = d.path().join(p);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, body).unwrap();
        }
        let idx = std::sync::Arc::new(Index::build(d.path(), &repomap_core::BuildOptions { use_cache: false, ..Default::default() }).unwrap());
        let mut map = idx.to_shared("test", repomap_core::workspace_graph::shared_packages(&idx));
        map.commit = Some(commit.into());
        map.web = Some("https://github.com/acme/api".into());
        serde_json::to_string(&map).unwrap()
    }

    const FILES: &[(&str, &str)] = &[("src/a.ts", "export function alpha() {\n  return beta();\n}\nexport function beta() {\n  return 1;\n}\n")];

    fn cfg(base: &str, dir: &std::path::Path, token: Option<&str>) -> Config {
        Config { base: base.into(), token: token.map(String::from), cache_dir: dir.to_path_buf(), fresh: Duration::ZERO }
    }

    fn link(c: &Config) -> Link {
        c.link(&format!("{}/m/acme/api", c.base)).unwrap()
    }

    #[test]
    fn etag_revalidation_reuses_the_cached_map() {
        let body = sample_map(FILES, "c1");
        let fake = serve(move |r| match r.header("if-none-match") {
            Some("\"c1.1\"") => Reply { status: 304, headers: vec![], body: String::new() },
            _ => Reply { status: 200, headers: vec![("etag", "\"c1.1\"".into()), ("content-type", "application/vnd.repomap.map+json".into())], body: body.clone() },
        });
        let d = tempfile::tempdir().unwrap();
        let c = cfg(&fake.base, d.path(), Some("rms_tok"));
        let first = load(&c, &link(&c)).unwrap();
        assert!(first.symbols.iter().any(|s| s.name == "alpha"));
        assert_eq!(first.shared.as_ref().unwrap().commit.as_deref(), Some("c1"));
        let second = load(&c, &link(&c)).unwrap();
        // 304: the same index, nothing re-read.
        assert!(Arc::ptr_eq(&first, &second));
        let seen = fake.seen.lock().unwrap();
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[0].url, "/v1/repos/acme/api/map");
        assert_eq!(seen[0].header("authorization"), Some("Bearer rms_tok"));
        assert_eq!(seen[0].header("if-none-match"), None);
        assert_eq!(seen[1].header("if-none-match"), Some("\"c1.1\""));
        drop(seen);
        // A new process (empty memory) revalidates from the disk cache.
        held().lock().unwrap().clear();
        let third = load(&c, &link(&c)).unwrap();
        assert!(third.symbols.iter().any(|s| s.name == "beta"));
        assert_eq!(fake.seen.lock().unwrap()[2].header("if-none-match"), Some("\"c1.1\""));
    }

    #[test]
    fn a_fresh_map_is_not_asked_for_again() {
        let body = sample_map(FILES, "c1");
        let fake = serve(move |_| Reply { status: 200, headers: vec![("etag", "\"c1.1\"".into())], body: body.clone() });
        let d = tempfile::tempdir().unwrap();
        let c = Config { fresh: Duration::from_secs(60), ..cfg(&fake.base, d.path(), None) };
        load(&c, &link(&c)).unwrap();
        load(&c, &link(&c)).unwrap();
        assert_eq!(fake.seen.lock().unwrap().len(), 1);
    }

    #[test]
    fn each_status_has_its_own_answer() {
        for (status, retry, want) in [
            (401, None, Fail::Unauthorized),
            (402, None, Fail::TeamInactive),
            (404, None, Fail::NotFound),
            (202, Some("7"), Fail::Building(7)),
        ] {
            let fake = serve(move |_| {
                let mut r = reply(status, r#"{"error":{"type":"x","code":"y","message":"z"}}"#);
                if let Some(s) = retry {
                    r.headers.push(("retry-after", s.into()));
                }
                r
            });
            let d = tempfile::tempdir().unwrap();
            let c = cfg(&fake.base, d.path(), None);
            assert_eq!(load(&c, &link(&c)).err(), Some(want.clone()), "{status}");
        }
        assert!(Fail::Unauthorized.message().contains("repomap login"));
        assert!(Fail::NotFound.message().contains("no map you can read"));
        assert!(Fail::Building(7).message().contains("retry in 7 s"));
        let fake = serve(|_| reply(500, "{}"));
        let d = tempfile::tempdir().unwrap();
        let c = cfg(&fake.base, d.path(), None);
        assert!(matches!(load(&c, &link(&c)), Err(Fail::Other(m)) if m.contains("500")));
    }

    #[test]
    fn losing_access_drops_the_cached_map() {
        let body = sample_map(FILES, "c1");
        let gone = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let g2 = gone.clone();
        let fake = serve(move |_| {
            if g2.load(std::sync::atomic::Ordering::SeqCst) {
                reply(404, "{}")
            } else {
                Reply { status: 200, headers: vec![("etag", "\"c1.1\"".into())], body: body.clone() }
            }
        });
        let d = tempfile::tempdir().unwrap();
        let c = cfg(&fake.base, d.path(), Some("rms_t"));
        load(&c, &link(&c)).unwrap();
        gone.store(true, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(load(&c, &link(&c)).err(), Some(Fail::NotFound));
        assert!(!d.path().join(slug(&c.base)).join("acme/api/map.json").exists());
    }

    #[test]
    fn an_unreachable_server_serves_the_cached_map() {
        let body = sample_map(FILES, "c1");
        let fake = serve(move |_| Reply { status: 200, headers: vec![("etag", "\"c1.1\"".into())], body: body.clone() });
        let d = tempfile::tempdir().unwrap();
        let c = cfg(&fake.base, d.path(), None);
        load(&c, &link(&c)).unwrap();
        held().lock().unwrap().clear();
        drop(fake);
        assert!(load(&c, &link(&c)).is_ok());
    }

    #[test]
    fn only_the_review_host_is_trusted_with_a_token() {
        let c = cfg("https://review.repomap.sylphx.com", std::path::Path::new("/nonexistent"), Some("rms_t"));
        assert!(c.link("https://review.repomap.sylphx.com/m/acme/api").is_ok());
        let e = c.link("https://evil.example/m/acme/api").unwrap_err();
        assert!(e.contains("not on the repomap review server"), "{e}");
        assert!(c.link("https://review.repomap.sylphx.com/other").unwrap_err().contains("not a shared-map link"));
        assert!(c.link("../api").is_err());
    }
}

#[cfg(test)]
pub(crate) mod fake {
    //! A scripted HTTP server on a loopback port, for the remote read path
    //! and `repomap login`.

    use std::sync::{Arc, Mutex};

    pub struct Req {
        pub method: String,
        pub url: String,
        pub headers: Vec<(String, String)>,
        pub body: String,
    }

    impl Req {
        pub fn header(&self, name: &str) -> Option<&str> {
            self.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
        }
    }

    pub struct Reply {
        pub status: u16,
        pub headers: Vec<(&'static str, String)>,
        pub body: String,
    }

    pub fn reply(status: u16, body: &str) -> Reply {
        Reply { status, headers: vec![("content-type", "application/json".into())], body: body.into() }
    }

    pub struct Fake {
        pub base: String,
        pub seen: Arc<Mutex<Vec<Req>>>,
        server: Arc<tiny_http::Server>,
    }

    impl Drop for Fake {
        fn drop(&mut self) {
            self.server.unblock();
        }
    }

    /// `handler` answers each request; the server stops when dropped.
    pub fn serve(handler: impl Fn(&Req) -> Reply + Send + 'static) -> Fake {
        let server = Arc::new(tiny_http::Server::http("127.0.0.1:0").unwrap());
        let base = format!("http://{}", server.server_addr().to_ip().unwrap());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let (s2, seen2) = (server.clone(), seen.clone());
        std::thread::spawn(move || {
            for mut rq in s2.incoming_requests() {
                let mut body = String::new();
                let _ = rq.as_reader().read_to_string(&mut body);
                let req = Req {
                    method: rq.method().to_string(),
                    url: rq.url().to_string(),
                    headers: rq.headers().iter().map(|h| (h.field.to_string(), h.value.to_string())).collect(),
                    body,
                };
                let r = handler(&req);
                seen2.lock().unwrap().push(req);
                let mut resp = tiny_http::Response::from_string(r.body).with_status_code(r.status);
                for (k, v) in r.headers {
                    resp.add_header(tiny_http::Header::from_bytes(k.as_bytes(), v.as_bytes()).unwrap());
                }
                let _ = rq.respond(resp);
            }
        });
        Fake { base, seen, server }
    }
}
