//! `repomap login`: GitHub's device flow (RFC 8628) against the repomap Review
//! App, then one exchange with the review server for a repomap session token
//! (`rms_...`). The GitHub token is used once and discarded; only the session
//! token is stored, in a 0600 file beside the licence file. Headless agents and
//! CI set `REPOMAP_TOKEN` to a session token instead.

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// The repomap Review App's OAuth client id. It is public (the device flow
/// needs no secret). Empty until the App has Device Flow enabled; set
/// `REPOMAP_GITHUB_CLIENT_ID` to use another App meanwhile.
pub const GITHUB_CLIENT_ID: &str = "";
const GITHUB: &str = "https://github.com";
const TIMEOUT: Duration = Duration::from_secs(30);
pub const TOKEN_ENV: &str = "REPOMAP_TOKEN";
const PREFIX: &str = "rms_";

pub struct Endpoints {
    pub github: String,
    pub review: String,
    pub client_id: String,
}

impl Endpoints {
    pub fn from_env() -> Endpoints {
        let id = std::env::var("REPOMAP_GITHUB_CLIENT_ID").ok().filter(|v| !v.is_empty());
        Endpoints {
            github: std::env::var("REPOMAP_GITHUB_URL").ok().filter(|v| !v.is_empty()).unwrap_or_else(|| GITHUB.into()).trim_end_matches('/').to_string(),
            review: review_base(),
            client_id: id.unwrap_or_else(|| GITHUB_CLIENT_ID.to_string()),
        }
    }
}

/// `scheme://host[:port]` of the review server.
pub fn review_base() -> String {
    std::env::var("REPOMAP_REVIEW_URL")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| crate::remote::DEFAULT_BASE.into())
        .trim_end_matches('/')
        .to_string()
}

/// The stored session: `<config dir>/repomap/session`.
pub fn session_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("repomap").join("session"))
}

/// The session token: `REPOMAP_TOKEN` when it is one (`serve` also reads that
/// variable for its own UI token, which never starts with `rms_`), else the
/// stored file.
pub fn token() -> Option<String> {
    token_from(std::env::var(TOKEN_ENV).ok().as_deref(), session_path().as_deref())
}

fn token_from(env: Option<&str>, path: Option<&Path>) -> Option<String> {
    let ok = |s: &str| s.starts_with(PREFIX).then(|| s.to_string());
    env.and_then(|v| ok(v.trim())).or_else(|| std::fs::read_to_string(path?).ok().and_then(|s| ok(s.trim())))
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .http_status_as_error(false)
        .max_redirects(0)
        .user_agent(concat!("repomap/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

/// POST `body` as JSON, answer `(status, parsed JSON)`.
fn post(url: &str, body: &Value) -> Result<(u16, Value)> {
    let mut res = agent()
        .post(url)
        .header("content-type", "application/json")
        .header("accept", "application/json")
        .send(body.to_string())
        .with_context(|| format!("cannot reach {url}"))?;
    let status = res.status().as_u16();
    let text = res.body_mut().read_to_string().unwrap_or_default();
    Ok((status, serde_json::from_str(&text).unwrap_or(Value::Null)))
}

/// Run the device flow and store the session at `path`. Returns the GitHub login.
pub fn login(ep: &Endpoints, path: &Path, out: &mut dyn Write) -> Result<String> {
    if ep.client_id.is_empty() {
        bail!("this repomap build has no GitHub client id for sign-in yet; set REPOMAP_GITHUB_CLIENT_ID, or use REPOMAP_TOKEN with a session token");
    }
    let (status, dev) = post(&format!("{}/login/device/code", ep.github), &json!({"client_id": ep.client_id}))?;
    let field = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).map(String::from);
    let (Some(device), Some(user_code), Some(uri)) = (field(&dev, "device_code"), field(&dev, "user_code"), field(&dev, "verification_uri")) else {
        bail!("GitHub did not start the sign-in ({status}): {}", field(&dev, "error_description").or_else(|| field(&dev, "error")).unwrap_or_default());
    };
    writeln!(out, "Open {uri} and enter the code {user_code}")?;
    out.flush()?;
    let mut interval = dev.get("interval").and_then(Value::as_u64).unwrap_or(5);
    let deadline = Instant::now() + Duration::from_secs(dev.get("expires_in").and_then(Value::as_u64).unwrap_or(900));
    let github_token = loop {
        std::thread::sleep(Duration::from_secs(interval));
        if Instant::now() > deadline {
            bail!("the sign-in code expired; run `repomap login` again");
        }
        let (_, v) = post(
            &format!("{}/login/oauth/access_token", ep.github),
            &json!({"client_id": ep.client_id, "device_code": device, "grant_type": "urn:ietf:params:oauth:grant-type:device_code"}),
        )?;
        if let Some(t) = field(&v, "access_token") {
            break t;
        }
        match field(&v, "error").as_deref() {
            Some("authorization_pending") => {}
            Some("slow_down") => interval += 5,
            Some("expired_token") => bail!("the sign-in code expired; run `repomap login` again"),
            Some("access_denied") => bail!("sign-in was cancelled on GitHub"),
            other => bail!("GitHub refused the sign-in: {}", other.unwrap_or("no access token")),
        }
    };
    // One exchange: the server reads who we are, revokes the GitHub token and
    // answers with the session token. Nothing of GitHub's is kept here.
    let (status, s) = post(&format!("{}/v1/sessions", ep.review), &json!({"github_token": github_token}))?;
    let Some(session) = field(&s, "token").filter(|t| t.starts_with(PREFIX)) else {
        bail!(
            "the repomap review server did not start a session ({status}): {}",
            s.pointer("/error/message").and_then(Value::as_str).unwrap_or("no session token")
        );
    };
    write_session(path, &session)?;
    Ok(s.pointer("/user/login").and_then(Value::as_str).unwrap_or("you").to_string())
}

/// Write the token to `path` readable by the user only.
pub fn write_session(path: &Path, token: &str) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(&tmp).with_context(|| format!("cannot write {}", tmp.display()))?;
    f.write_all(token.as_bytes())?;
    f.flush()?;
    drop(f);
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// End the session on the server (best effort) and delete the stored file.
/// Returns whether a stored session was removed.
pub fn logout(ep: &Endpoints, path: &Path) -> Result<bool> {
    let stored = std::fs::read_to_string(path).ok().map(|s| s.trim().to_string()).filter(|s| s.starts_with(PREFIX));
    if let Some(t) = &stored {
        let _ = agent().delete(format!("{}/v1/sessions/current", ep.review)).header("authorization", &format!("Bearer {t}")).call();
    }
    match std::fs::remove_file(path) {
        Ok(()) => Ok(stored.is_some()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}

pub fn run_login() -> Result<()> {
    let path = session_path().context("no config directory on this machine; use REPOMAP_TOKEN")?;
    let who = login(&Endpoints::from_env(), &path, &mut std::io::stdout())?;
    println!("Signed in as {who}. Session stored in {}", path.display());
    Ok(())
}

pub fn run_logout() -> Result<()> {
    let path = session_path().context("no config directory on this machine")?;
    if logout(&Endpoints::from_env(), &path)? {
        println!("Signed out. Removed {}", path.display());
    } else {
        println!("No stored session.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::fake::{reply, serve};

    #[test]
    fn device_flow_stores_only_the_session_token() {
        let polls = std::sync::Mutex::new(0);
        let fake = serve(move |r| match (r.method.as_str(), r.url.as_str()) {
            ("POST", "/login/device/code") => reply(200, r#"{"device_code":"dc","user_code":"ABCD-1234","verification_uri":"https://github.com/login/device","interval":0,"expires_in":60}"#),
            ("POST", "/login/oauth/access_token") => {
                let mut n = polls.lock().unwrap();
                *n += 1;
                match *n {
                    1 => reply(200, r#"{"error":"authorization_pending"}"#),
                    _ => reply(200, r#"{"access_token":"gho_githubusertoken"}"#),
                }
            }
            ("POST", "/v1/sessions") => reply(201, r#"{"object":"session","token":"rms_abc123","user":{"id":7,"login":"octocat"},"expires_at":1}"#),
            _ => reply(404, "{}"),
        });
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("repomap/session");
        let ep = Endpoints { github: fake.base.clone(), review: fake.base.clone(), client_id: "cid".into() };
        let mut out = Vec::new();
        let who = login(&ep, &path, &mut out).unwrap();
        assert_eq!(who, "octocat");
        let shown = String::from_utf8(out).unwrap();
        assert!(shown.contains("ABCD-1234") && shown.contains("https://github.com/login/device"), "{shown}");
        // The session token is stored and the GitHub token is not.
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "rms_abc123");
        assert!(!std::fs::read_to_string(&path).unwrap().contains("gho_"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        }
        // The server saw the GitHub token exactly once, in the exchange.
        let seen = fake.seen.lock().unwrap();
        let exchange = seen.iter().filter(|r| r.url == "/v1/sessions").collect::<Vec<_>>();
        assert_eq!(exchange.len(), 1);
        assert!(exchange[0].body.contains("gho_githubusertoken"));
        drop(seen);
        assert_eq!(token_from(None, Some(&path)).as_deref(), Some("rms_abc123"));
        // Logout ends the session and removes the file.
        assert!(logout(&ep, &path).unwrap());
        assert!(!path.exists());
        assert!(fake.seen.lock().unwrap().iter().any(|r| r.method == "DELETE" && r.url == "/v1/sessions/current" && r.header("authorization") == Some("Bearer rms_abc123")));
    }

    #[test]
    fn denied_expired_and_unconfigured_sign_ins_say_so() {
        let fake = serve(|r| match r.url.as_str() {
            "/login/device/code" => reply(200, r#"{"device_code":"dc","user_code":"X","verification_uri":"u","interval":0,"expires_in":60}"#),
            _ => reply(200, r#"{"error":"access_denied"}"#),
        });
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("session");
        let ep = Endpoints { github: fake.base.clone(), review: fake.base.clone(), client_id: "cid".into() };
        let e = login(&ep, &path, &mut Vec::new()).unwrap_err().to_string();
        assert!(e.contains("cancelled"), "{e}");
        assert!(!path.exists());
        let none = Endpoints { client_id: String::new(), ..ep };
        assert!(login(&none, &path, &mut Vec::new()).unwrap_err().to_string().contains("client id"));
    }

    #[test]
    fn only_session_tokens_count() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("s");
        std::fs::write(&p, "not-a-session\n").unwrap();
        // `serve` also reads REPOMAP_TOKEN for its own UI token; that is not a session.
        assert_eq!(token_from(None, Some(&p)), None);
        assert_eq!(token_from(Some("a-serve-ui-token"), Some(&p)), None);
        std::fs::write(&p, "rms_x\n").unwrap();
        assert_eq!(token_from(None, Some(&p)).as_deref(), Some("rms_x"));
        // The environment wins over the stored file.
        assert_eq!(token_from(Some("rms_env"), Some(&p)).as_deref(), Some("rms_env"));
    }
}
