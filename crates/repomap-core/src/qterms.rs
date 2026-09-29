//! What a long natural-language query (an issue, a bug report, a stack trace)
//! says about code: identifiers, file paths and module names, and the title.

/// Extensions that make a dotted token a file name.
const EXTS: &[&str] = &[
    "py", "pyi", "js", "jsx", "ts", "tsx", "mjs", "cjs", "rs", "go", "java", "kt", "rb", "php", "c", "h", "cc", "cpp", "hpp", "cs",
    "swift", "scala", "rst", "md", "txt", "toml", "yaml", "yml", "json", "cfg", "ini", "html", "css", "vue", "sh",
];

#[derive(Debug, Default, Clone)]
pub struct QueryTerms {
    /// Code-like identifiers as written, in order of appearance: `snake_case`,
    /// `camelCase`, `PascalCase`, anything in backticks, and the function
    /// names of stack-trace frames.
    pub idents: Vec<String>,
    /// Lowercased file paths (`django/db/models/query.py`) and module paths
    /// (`django/db/models/query`, from `django.db.models.query`).
    pub paths: Vec<String>,
    /// The first non-empty line, which for an issue is its title.
    pub head: String,
    /// Identifiers and backtick spans, joined: the code the text talks about.
    pub code: String,
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `snake_case` with an inner underscore, or a lowercase letter followed by an
/// uppercase one (`camelCase`, `PascalCase`, `HTTPServer`), of 3+ characters.
fn code_like(w: &str) -> bool {
    if w.len() < 3 || w.len() > 64 {
        return false;
    }
    let b = w.as_bytes();
    if w.trim_matches('_').is_empty() {
        return false;
    }
    if w.trim_matches('_').contains('_') && w.chars().any(|c| c.is_ascii_alphabetic()) {
        return true;
    }
    // A hump: a lowercase letter followed by an uppercase one.
    b.windows(2).any(|p| p[0].is_ascii_lowercase() && p[1].is_ascii_uppercase())
}

fn push_unique(v: &mut Vec<String>, s: String) {
    if !v.contains(&s) {
        v.push(s);
    }
}

fn words_of(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (i, c) in s.char_indices() {
        match (is_ident_char(c), start) {
            (true, None) => start = Some(i),
            (false, Some(st)) => {
                out.push(&s[st..i]);
                start = None;
            }
            _ => {}
        }
    }
    if let Some(st) = start {
        out.push(&s[st..]);
    }
    out.into_iter().filter(|w| w.chars().next().is_some_and(is_ident_start)).collect()
}

fn has_ext(seg: &str) -> bool {
    match seg.rsplit_once('.') {
        Some((stem, ext)) => !stem.is_empty() && EXTS.contains(&ext.to_ascii_lowercase().as_str()),
        None => false,
    }
}

/// A file or module path in one whitespace-delimited token, when it is one.
fn path_of(tok: &str) -> Option<String> {
    let t = tok.trim_matches(|c: char| matches!(c, '.' | ',' | ';' | ':' | '\'' | '"' | '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | '`' | '*'));
    let t = t.rsplit("://").next().unwrap_or(t);
    // `path.py:12` and `path.py#L12`.
    let t = t.split(['#', '?']).next().unwrap_or(t);
    let t = match t.rsplit_once(':') {
        Some((a, n)) if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) => a,
        _ => t,
    };
    if t.len() < 3 || t.len() > 200 {
        return None;
    }
    let norm = t.replace('\\', "/");
    let norm = norm.trim_start_matches("./");
    let last = norm.rsplit('/').next().unwrap_or(norm);
    if !norm.chars().all(|c| is_ident_char(c) || matches!(c, '/' | '.' | '-' | '@' | '+')) {
        return None;
    }
    if has_ext(last) {
        return Some(norm.to_ascii_lowercase());
    }
    // `pkg.sub.module`: every part an identifier, at least two of them.
    if !norm.contains('/') && norm.contains('.') {
        let parts: Vec<&str> = norm.split('.').collect();
        if parts.len() >= 2 && parts.iter().all(|p| !p.is_empty() && p.chars().next().is_some_and(is_ident_start) && p.chars().all(is_ident_char)) {
            return Some(parts.join("/").to_ascii_lowercase());
        }
    }
    None
}

pub fn analyze(q: &str) -> QueryTerms {
    let head = q.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("").chars().take(300).collect();
    let mut out = QueryTerms { head, ..Default::default() };

    // Backtick spans on one line: the author marked them as code.
    let mut spans: Vec<String> = Vec::new();
    let mut plain = String::with_capacity(q.len());
    let mut rest = q;
    while let Some(i) = rest.find('`') {
        plain.push_str(&rest[..i]);
        plain.push(' ');
        let after = &rest[i + 1..];
        // Fenced blocks (```) are code as a whole; their words are treated like plain text.
        match after.find('`') {
            Some(j) if j <= 120 && !after[..j].contains('\n') && j > 0 => {
                spans.push(after[..j].to_string());
                plain.push_str(&after[..j]);
                plain.push(' ');
                rest = &after[j + 1..];
            }
            _ => rest = after,
        }
    }
    plain.push_str(rest);

    // Stack-trace frames: `File "x.py", line 12, in name`.
    let mut frames: Vec<String> = Vec::new();
    for l in plain.lines() {
        if let Some((pre, post)) = l.split_once(", in ") {
            let has_line = pre.rsplit_once("line ").is_some_and(|(_, n)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
            if has_line {
                let name: String = post.trim().chars().take_while(|c| is_ident_char(*c)).collect();
                if name.len() >= 3 && !name.starts_with("_module") {
                    push_unique(&mut frames, name);
                }
            }
        }
    }

    for tok in plain.split_whitespace() {
        if let Some(p) = path_of(tok) {
            push_unique(&mut out.paths, p);
        }
    }
    for s in &spans {
        for w in words_of(s) {
            if w.len() >= 3 {
                push_unique(&mut out.idents, w.to_string());
            }
        }
    }
    for f in frames {
        push_unique(&mut out.idents, f);
    }
    for w in words_of(&plain) {
        if code_like(w) {
            push_unique(&mut out.idents, w.to_string());
        }
    }
    out.idents.truncate(48);
    out.paths.truncate(24);
    let mut code = out.idents.join(" ");
    for s in &spans {
        code.push(' ');
        code.push_str(s);
    }
    out.code = code.chars().take(1500).collect();
    out
}

/// How strongly a file path is one the query names: 1.0 for its full path,
/// 0.5 for a bare file name, 0.8 for a module path (`a.b.c` for `a/b/c.py`).
pub fn path_mention(file: &str, paths: &[String]) -> f32 {
    let f = file.to_ascii_lowercase();
    let stem = match f.rsplit_once('.') {
        Some((s, _)) if !s.ends_with('/') && !s.is_empty() => s,
        _ => f.as_str(),
    };
    let pkg = stem.strip_suffix("/__init__");
    let mut best = 0f32;
    for m in paths {
        let last = m.rsplit('/').next().unwrap_or(m);
        let v = if has_ext(last) {
            if f == *m || (m.contains('/') && f.ends_with(&format!("/{m}"))) {
                1.0
            } else if m.ends_with(&format!("/{f}")) && f.contains('/') {
                1.0
            } else if !m.contains('/') && f.ends_with(&format!("/{m}")) {
                0.5
            } else {
                0.0
            }
        } else if stem == m || stem.ends_with(&format!("/{m}")) {
            0.8
        } else if pkg.is_some_and(|p| p == m || p.ends_with(&format!("/{m}"))) {
            0.6
        } else {
            0.0
        };
        best = best.max(v);
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_identifiers_paths_and_frames() {
        let q = "QuerySet.filter() is slow\n\nIn `django/db/models/query.py` the method get_or_create calls `bulk_create`.\n  File \"/x/django/db/models/sql/compiler.py\", line 12, in execute_sql\nSee django.db.models.query and some plain words.";
        let t = analyze(q);
        assert_eq!(t.head, "QuerySet.filter() is slow");
        for want in ["QuerySet", "get_or_create", "bulk_create", "execute_sql"] {
            assert!(t.idents.contains(&want.to_string()), "{want} in {:?}", t.idents);
        }
        assert!(!t.idents.contains(&"plain".to_string()));
        assert!(t.paths.contains(&"django/db/models/query.py".to_string()), "{:?}", t.paths);
        assert!(t.paths.contains(&"django/db/models/query".to_string()), "{:?}", t.paths);
    }

    #[test]
    fn mentions_match_files() {
        let p = vec!["django/db/models/query.py".to_string(), "django/db/models/sql".to_string(), "utils.py".to_string()];
        assert_eq!(path_mention("django/db/models/query.py", &p), 1.0);
        assert_eq!(path_mention("django/db/models/sql/__init__.py", &p), 0.6);
        assert_eq!(path_mention("django/utils.py", &p), 0.5);
        assert_eq!(path_mention("django/db/models/base.py", &p), 0.0);
    }
}
