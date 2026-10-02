//! Database map: tables, columns, foreign keys and indexes, from a live
//! database (introspected by the binary) or from schema sources in the repo:
//! SQL migrations, Prisma, Drizzle, SQLAlchemy, Diesel and Django models.
//! Tables are linked to the code that queries them.

use crate::index::Index;
use crate::lang::Lang;
use regex::Regex;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write;

#[derive(Debug, Clone, Default, Serialize)]
pub struct Column {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub primary_key: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ForeignKey {
    pub columns: Vec<String>,
    pub ref_table: String,
    pub ref_columns: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct IndexDef {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct CodeRef {
    pub file: String,
    pub line: u32,
    pub symbol: Option<String>,
    pub via: &'static str,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Table {
    /// Non-default schema (`public`/`main`/database default are omitted).
    pub schema: Option<String>,
    pub name: String,
    pub kind: String,
    pub columns: Vec<Column>,
    pub foreign_keys: Vec<ForeignKey>,
    pub indexes: Vec<IndexDef>,
    /// Where the definition came from: `file:line`, or the live database kind.
    pub source: String,
    /// ORM identifiers that stand for this table in code (model, variable, module).
    pub aliases: Vec<String>,
    pub used_by: Vec<CodeRef>,
}

impl Table {
    pub fn key(&self) -> String {
        match &self.schema {
            Some(s) => format!("{s}.{}", self.name),
            None => self.name.clone(),
        }
    }
    fn col_mut(&mut self, name: &str) -> Option<&mut Column> {
        self.columns.iter_mut().find(|c| c.name.eq_ignore_ascii_case(name))
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DbSchema {
    /// `postgres`, `mysql`, `sqlite`, or `repo` for static sources.
    pub origin: String,
    pub sources: Vec<String>,
    pub tables: Vec<Table>,
}

impl DbSchema {
    pub fn table(&self, name: &str) -> Option<&Table> {
        let n = name.trim().trim_matches('"');
        self.tables
            .iter()
            .find(|t| t.key().eq_ignore_ascii_case(n))
            .or_else(|| self.tables.iter().find(|t| t.name.eq_ignore_ascii_case(n)))
            .or_else(|| self.tables.iter().find(|t| t.aliases.iter().any(|a| a.eq_ignore_ascii_case(n))))
    }

    fn upsert(&mut self, t: Table) -> usize {
        if let Some(i) = self.tables.iter().position(|x| x.key().eq_ignore_ascii_case(&t.key())) {
            let old = &mut self.tables[i];
            // Prefer the ORM definition as the source: code links resolve through it.
            if !t.source.to_ascii_lowercase().contains(".sql:") && old.source.to_ascii_lowercase().contains(".sql:") {
                old.source = t.source.clone();
            }
            for c in t.columns {
                if old.col_mut(&c.name).is_none() {
                    old.columns.push(c);
                }
            }
            old.foreign_keys.extend(t.foreign_keys);
            old.indexes.extend(t.indexes);
            for a in t.aliases {
                if !old.aliases.contains(&a) {
                    old.aliases.push(a);
                }
            }
            i
        } else {
            self.tables.push(t);
            self.tables.len() - 1
        }
    }

    fn find_mut(&mut self, name: &str) -> Option<&mut Table> {
        let (schema, n) = split_name(name);
        self.tables
            .iter_mut()
            .find(|t| t.name.eq_ignore_ascii_case(&n) && (schema.is_none() || t.schema.as_deref().map(|s| s.eq_ignore_ascii_case(schema.as_deref().unwrap())).unwrap_or(false)))
    }
}

// ---------------------------------------------------------------- identifiers

fn unquote(s: &str) -> String {
    s.trim().trim_matches(|c| c == '"' || c == '`' || c == '[' || c == ']' || c == '\'').to_string()
}

/// `"public"."users"` -> (None, users); `auth.users` -> (Some(auth), users).
pub fn split_name(raw: &str) -> (Option<String>, String) {
    let parts: Vec<String> = raw.split('.').map(unquote).filter(|p| !p.is_empty()).collect();
    match parts.len() {
        0 => (None, String::new()),
        1 => (None, parts[0].clone()),
        _ => {
            let schema = parts[parts.len() - 2].clone();
            let name = parts[parts.len() - 1].clone();
            if matches!(schema.to_ascii_lowercase().as_str(), "public" | "main" | "dbo") {
                (None, name)
            } else {
                (Some(schema), name)
            }
        }
    }
}

// ---------------------------------------------------------------- SQL DDL

fn strip_sql_comments(src: &str) -> String {
    let b = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    let mut quote: Option<u8> = None;
    while i < b.len() {
        let c = b[i];
        if let Some(q) = quote {
            out.push(c as char);
            if c == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        if c == b'\'' || c == b'"' || c == b'`' {
            quote = Some(c);
            out.push(c as char);
            i += 1;
        } else if c == b'-' && i + 1 < b.len() && b[i + 1] == b'-' {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else if c == b'/' && i + 1 < b.len() && b[i + 1] == b'*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                i += 1;
            }
            i += 2;
        } else {
            out.push(c as char);
            i += 1;
        }
    }
    out
}

/// Split on `sep` at paren depth 0, outside quotes and `$$` bodies.
fn split_top(s: &str, sep: char) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut dollar = false;
    let mut start = 0;
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let mut k = 0;
    while k < chars.len() {
        let (i, c) = chars[k];
        if dollar {
            if c == '$' && k + 1 < chars.len() && chars[k + 1].1 == '$' {
                dollar = false;
                k += 2;
                continue;
            }
            k += 1;
            continue;
        }
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            k += 1;
            continue;
        }
        match c {
            '\'' | '"' | '`' => quote = Some(c),
            '$' if k + 1 < chars.len() && chars[k + 1].1 == '$' => {
                dollar = true;
                k += 2;
                continue;
            }
            '(' => depth += 1,
            ')' => depth -= 1,
            _ if c == sep && depth == 0 => {
                out.push((start, s[start..i].to_string()));
                start = i + c.len_utf8();
            }
            _ => {}
        }
        k += 1;
    }
    if start < s.len() {
        out.push((start, s[start..].to_string()));
    }
    out
}

/// Words, quoted identifiers and parenthesised groups as tokens.
fn sql_tokens(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() || c == ',' {
            i += 1;
        } else if c == '(' {
            let mut depth = 0;
            let start = i;
            while i < chars.len() {
                if chars[i] == '(' {
                    depth += 1;
                } else if chars[i] == ')' {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        break;
                    }
                }
                i += 1;
            }
            out.push(chars[start..i].iter().collect());
        } else if c == '"' || c == '`' || c == '[' || c == '\'' {
            let close = if c == '[' { ']' } else { c };
            let start = i;
            i += 1;
            while i < chars.len() && chars[i] != close {
                i += 1;
            }
            i += 1;
            let mut tok: String = chars[start..i.min(chars.len())].iter().collect();
            // Glue a following `.name` (schema-qualified identifiers).
            while i < chars.len() && chars[i] == '.' {
                let s2 = i;
                i += 1;
                while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '(' && chars[i] != ',' {
                    i += 1;
                }
                tok.push_str(&chars[s2..i].iter().collect::<String>());
            }
            out.push(tok);
        } else {
            let start = i;
            while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '(' && chars[i] != ',' {
                if chars[i] == '"' || chars[i] == '`' {
                    // "schema"."table" style continues through quotes.
                    let q = chars[i];
                    i += 1;
                    while i < chars.len() && chars[i] != q {
                        i += 1;
                    }
                }
                i += 1;
            }
            out.push(chars[start..i.min(chars.len())].iter().collect());
        }
    }
    out
}

fn paren_list(tok: &str) -> Vec<String> {
    let inner = tok.trim().trim_start_matches('(').trim_end_matches(')');
    split_top(inner, ',')
        .into_iter()
        .map(|(_, s)| {
            let s = s.trim();
            // `col ASC`, `lower(col)`, `col(10)` -> the identifier part.
            let first = s.split_whitespace().next().unwrap_or("");
            unquote(first.split('(').next().unwrap_or(first))
        })
        .filter(|s| !s.is_empty())
        .collect()
}

fn upper(t: &str) -> String {
    t.to_ascii_uppercase()
}

const COL_STOP: &[&str] = &[
    "NOT", "NULL", "PRIMARY", "UNIQUE", "REFERENCES", "DEFAULT", "CONSTRAINT", "CHECK", "GENERATED", "COLLATE", "AUTO_INCREMENT", "AUTOINCREMENT", "COMMENT", "ON", "IDENTITY", "AS",
];

fn parse_column(def: &str) -> Option<(Column, Option<ForeignKey>, bool)> {
    let toks = sql_tokens(def);
    let name = unquote(toks.first()?);
    if name.is_empty() {
        return None;
    }
    let mut ty = String::new();
    let mut i = 1;
    while i < toks.len() && !COL_STOP.contains(&upper(&toks[i]).as_str()) {
        if !ty.is_empty() && !toks[i].starts_with('(') {
            ty.push(' ');
        }
        ty.push_str(&toks[i]);
        i += 1;
    }
    let mut col = Column { name, data_type: ty.to_ascii_lowercase(), nullable: true, primary_key: false };
    let mut fk = None;
    let mut unique = false;
    while i < toks.len() {
        match upper(&toks[i]).as_str() {
            "NOT" if i + 1 < toks.len() && upper(&toks[i + 1]) == "NULL" => {
                col.nullable = false;
                i += 1;
            }
            "PRIMARY" => {
                col.primary_key = true;
                col.nullable = false;
            }
            "UNIQUE" => unique = true,
            "REFERENCES" if i + 1 < toks.len() => {
                let (s, t) = split_name(&toks[i + 1]);
                let ref_table = match s {
                    Some(s) => format!("{s}.{t}"),
                    None => t,
                };
                let ref_columns = if i + 2 < toks.len() && toks[i + 2].starts_with('(') { paren_list(&toks[i + 2]) } else { Vec::new() };
                fk = Some(ForeignKey { columns: vec![col.name.clone()], ref_table, ref_columns });
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }
    Some((col, fk, unique))
}

/// Table-level constraint or column definition inside CREATE TABLE / ALTER TABLE ADD.
fn apply_table_item(t: &mut Table, item: &str) {
    let toks = sql_tokens(item);
    if toks.is_empty() {
        return;
    }
    let mut k = 0;
    if upper(&toks[0]) == "CONSTRAINT" {
        k = 2;
    }
    if k >= toks.len() {
        return;
    }
    let head = upper(&toks[k]);
    match head.as_str() {
        "PRIMARY" => {
            if let Some(list) = toks.iter().skip(k).find(|x| x.starts_with('(')) {
                for c in paren_list(list) {
                    if let Some(col) = t.col_mut(&c) {
                        col.primary_key = true;
                        col.nullable = false;
                    }
                }
            }
        }
        "FOREIGN" => {
            let lists: Vec<&String> = toks.iter().skip(k).filter(|x| x.starts_with('(')).collect();
            if let Some(r) = toks.iter().position(|x| upper(x) == "REFERENCES") {
                if r + 1 < toks.len() && !lists.is_empty() {
                    let (s, n) = split_name(&toks[r + 1]);
                    t.foreign_keys.push(ForeignKey {
                        columns: paren_list(lists[0]),
                        ref_table: match s {
                            Some(s) => format!("{s}.{n}"),
                            None => n,
                        },
                        ref_columns: if r + 2 < toks.len() && toks[r + 2].starts_with('(') { paren_list(&toks[r + 2]) } else { Vec::new() },
                    });
                }
            }
        }
        "UNIQUE" | "KEY" | "INDEX" => {
            if let Some(list) = toks.iter().skip(k).find(|x| x.starts_with('(')) {
                let name = if k + 1 < toks.len() && !toks[k + 1].starts_with('(') && upper(&toks[k + 1]) != "KEY" { unquote(&toks[k + 1]) } else { String::new() };
                t.indexes.push(IndexDef { name, columns: paren_list(list), unique: head == "UNIQUE" });
            }
        }
        "CHECK" | "EXCLUDE" | "FULLTEXT" | "SPATIAL" => {}
        _ => {
            if let Some((col, fk, unique)) = parse_column(item) {
                if unique {
                    t.indexes.push(IndexDef { name: String::new(), columns: vec![col.name.clone()], unique: true });
                }
                if let Some(fk) = fk {
                    t.foreign_keys.push(fk);
                }
                if let Some(existing) = t.col_mut(&col.name) {
                    *existing = col;
                } else {
                    t.columns.push(col);
                }
            }
        }
    }
}

fn line_at(src: &str, byte: usize) -> u32 {
    src[..byte.min(src.len())].bytes().filter(|b| *b == b'\n').count() as u32 + 1
}

pub fn parse_sql(schema: &mut DbSchema, path: &str, src: &str) {
    let cleaned = strip_sql_comments(src);
    let re_create = Regex::new(r"(?is)^\s*create\s+(?:or\s+replace\s+)?(?:(?:global\s+|local\s+)?(?:temp|temporary)\s+|unlogged\s+|virtual\s+)?(table|view|materialized\s+view)\s+(?:if\s+not\s+exists\s+)?([^\s(]+)").unwrap();
    let re_index = Regex::new(r"(?is)^\s*create\s+(unique\s+)?index\s+(?:concurrently\s+)?(?:if\s+not\s+exists\s+)?([^\s(]*)\s*on\s+(?:only\s+)?([^\s(]+)(?:\s+using\s+\w+)?\s*(\(.*\))").unwrap();
    let re_alter = Regex::new(r"(?is)^\s*alter\s+table\s+(?:if\s+exists\s+)?(?:only\s+)?([^\s]+)\s+(.*)$").unwrap();
    let re_drop = Regex::new(r"(?is)^\s*drop\s+(?:table|view)\s+(?:if\s+exists\s+)?([^;]+?)(?:\s+cascade|\s+restrict)?\s*$").unwrap();
    for (off, stmt) in split_top(&cleaned, ';') {
        let line = line_at(&cleaned, off + stmt.len() - stmt.trim_start().len());
        let st = stmt.trim();
        if st.is_empty() {
            continue;
        }
        if let Some(c) = re_create.captures(st) {
            let kind = if c[1].to_ascii_lowercase().contains("view") { "view" } else { "table" };
            let (s, n) = split_name(&c[2]);
            let mut t = Table { schema: s, name: n, kind: kind.into(), source: format!("{path}:{line}"), ..Default::default() };
            if kind == "table" {
                let rest = &st[c.get(0).unwrap().end()..];
                if let Some(open) = rest.find('(') {
                    // Body = the balanced group after the name.
                    let body_tok = sql_tokens(&rest[open..]).into_iter().next().unwrap_or_default();
                    let body = body_tok.trim_start_matches('(').trim_end_matches(')');
                    for (_, item) in split_top(body, ',') {
                        apply_table_item(&mut t, item.trim());
                    }
                }
            }
            if let Some(existing) = schema.find_mut(&c[2]) {
                // CREATE TABLE after a DROP in a later migration: replace.
                *existing = t;
            } else {
                schema.tables.push(t);
            }
        } else if let Some(c) = re_index.captures(st) {
            let unique = c.get(1).is_some();
            let name = unquote(&c[2]);
            let cols = paren_list(sql_tokens(&c[4]).first().map(|s| s.as_str()).unwrap_or(""));
            if let Some(t) = schema.find_mut(&c[3]) {
                t.indexes.push(IndexDef { name, columns: cols, unique });
            }
        } else if let Some(c) = re_alter.captures(st) {
            let target = c[1].to_string();
            let actions = c[2].to_string();
            for (_, act) in split_top(&actions, ',') {
                let a = act.trim();
                let toks = sql_tokens(a);
                if toks.is_empty() {
                    continue;
                }
                let verb = upper(&toks[0]);
                let Some(t) = schema.find_mut(&target) else { break };
                match verb.as_str() {
                    "ADD" => {
                        let mut rest = a[3..].trim_start();
                        let lower = rest.to_ascii_lowercase();
                        if lower.starts_with("column ") {
                            rest = rest[7..].trim_start();
                        }
                        let lower = rest.to_ascii_lowercase();
                        if lower.starts_with("if not exists ") {
                            rest = rest[14..].trim_start();
                        }
                        apply_table_item(t, rest);
                    }
                    "DROP" => {
                        let mut k = 1;
                        if k < toks.len() && upper(&toks[k]) == "COLUMN" {
                            k += 1;
                        }
                        if k + 1 < toks.len() && upper(&toks[k]) == "IF" {
                            k += 2;
                        }
                        if k < toks.len() && upper(&toks[k]) != "CONSTRAINT" {
                            let col = unquote(&toks[k]);
                            t.columns.retain(|c| !c.name.eq_ignore_ascii_case(&col));
                            t.foreign_keys.retain(|f| !f.columns.iter().any(|c| c.eq_ignore_ascii_case(&col)));
                        }
                    }
                    "RENAME" => {
                        if toks.len() >= 3 && upper(&toks[1]) == "TO" {
                            let (_, n) = split_name(&toks[2]);
                            t.name = n;
                        } else {
                            let k = if upper(&toks.get(1).map(|s| s.as_str()).unwrap_or("")) == "COLUMN" { 2 } else { 1 };
                            if toks.len() > k + 2 && upper(&toks[k + 1]) == "TO" {
                                let (from, to) = (unquote(&toks[k]), unquote(&toks[k + 2]));
                                if let Some(col) = t.col_mut(&from) {
                                    col.name = to;
                                }
                            }
                        }
                    }
                    "ALTER" | "MODIFY" => {
                        let k = if toks.len() > 1 && upper(&toks[1]) == "COLUMN" { 2 } else { 1 };
                        if let Some(colname) = toks.get(k).map(|s| unquote(s)) {
                            let rest: Vec<String> = toks.iter().skip(k + 1).map(|s| upper(s)).collect();
                            if let Some(col) = t.col_mut(&colname) {
                                if rest.windows(2).any(|w| w[0] == "SET" && w[1] == "NOT") {
                                    col.nullable = false;
                                } else if rest.windows(2).any(|w| w[0] == "DROP" && w[1] == "NOT") {
                                    col.nullable = true;
                                } else if let Some(p) = rest.iter().position(|x| x == "TYPE") {
                                    if let Some(ty) = toks.get(k + 1 + p + 1) {
                                        col.data_type = ty.to_ascii_lowercase();
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        } else if let Some(c) = re_drop.captures(st) {
            for (_, name) in split_top(&c[1], ',') {
                let (s, n) = split_name(name.trim());
                schema.tables.retain(|t| !(t.name.eq_ignore_ascii_case(&n) && (s.is_none() || t.schema == s)));
            }
        }
    }
}

// ---------------------------------------------------------------- Prisma

fn to_snake(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_ascii_lowercase().to_string() + c.as_str(),
        None => String::new(),
    }
}

fn bracket_list(s: &str) -> Vec<String> {
    s.trim().trim_start_matches('[').trim_end_matches(']').split(',').map(|x| x.split('(').next().unwrap_or(x).trim().trim_matches('"').to_string()).filter(|x| !x.is_empty()).collect()
}

pub fn parse_prisma(schema: &mut DbSchema, path: &str, src: &str) {
    let re_block = Regex::new(r"(?m)^\s*(model|view)\s+(\w+)\s*\{").unwrap();
    let re_map = Regex::new(r#"@@map\(\s*(?:name:\s*)?"([^"]+)"\s*\)"#).unwrap();
    let re_fmap = Regex::new(r#"@map\(\s*(?:name:\s*)?"([^"]+)"\s*\)"#).unwrap();
    let re_rel = Regex::new(r"@relation\(([^)]*)\)").unwrap();
    let re_fields = Regex::new(r"fields:\s*(\[[^\]]*\])").unwrap();
    let re_refs = Regex::new(r"references:\s*(\[[^\]]*\])").unwrap();
    let re_attr_list = Regex::new(r"@@(index|unique|id)\(\s*(?:fields:\s*)?(\[[^\]]*\])").unwrap();
    let models: HashSet<String> = re_block.captures_iter(src).map(|c| c[2].to_string()).collect();
    // model name -> table name (after @@map), resolved in a first pass.
    let mut table_of: HashMap<String, String> = HashMap::new();
    let mut blocks = Vec::new();
    for c in re_block.captures_iter(src) {
        let start = c.get(0).unwrap().end();
        let end = src[start..].find("\n}").map(|e| start + e).unwrap_or(src.len());
        let body = &src[start..end];
        let table = re_map.captures(body).map(|m| m[1].to_string()).unwrap_or_else(|| c[2].to_string());
        table_of.insert(c[2].to_string(), table.clone());
        blocks.push((c[1].to_string(), c[2].to_string(), table, body.to_string(), line_at(src, c.get(0).unwrap().start())));
    }
    for (kind, model, table, body, line) in blocks {
        let mut t = Table { name: table, kind: if kind == "view" { "view".into() } else { "table".into() }, source: format!("{path}:{line}"), aliases: vec![model.clone(), lower_first(&model)], ..Default::default() };
        let mut field_col: HashMap<String, String> = HashMap::new();
        for raw in body.lines() {
            let l = raw.trim();
            if l.is_empty() || l.starts_with("//") {
                continue;
            }
            if l.starts_with("@@") {
                if let Some(c) = re_attr_list.captures(l) {
                    let cols: Vec<String> = bracket_list(&c[2]).into_iter().map(|f| field_col.get(&f).cloned().unwrap_or(f)).collect();
                    match &c[1] {
                        "id" => {
                            for f in &cols {
                                if let Some(col) = t.col_mut(f) {
                                    col.primary_key = true;
                                }
                            }
                        }
                        k => t.indexes.push(IndexDef { name: String::new(), columns: cols, unique: k == "unique" }),
                    }
                }
                continue;
            }
            let mut parts = l.split_whitespace();
            let (Some(field), Some(ty)) = (parts.next(), parts.next()) else { continue };
            let base = ty.trim_end_matches('?').trim_end_matches("[]");
            if let Some(rel) = re_rel.captures(l) {
                if let (Some(f), Some(r)) = (re_fields.captures(&rel[1]), re_refs.captures(&rel[1])) {
                    let cols: Vec<String> = bracket_list(&f[1]).into_iter().map(|x| field_col.get(&x).cloned().unwrap_or(x)).collect();
                    t.foreign_keys.push(ForeignKey { columns: cols, ref_table: table_of.get(base).cloned().unwrap_or_else(|| base.to_string()), ref_columns: bracket_list(&r[1]) });
                }
                continue;
            }
            if models.contains(base) || ty.ends_with("[]") && models.contains(base) {
                continue; // relation field without a column
            }
            let colname = re_fmap.captures(l).map(|m| m[1].to_string()).unwrap_or_else(|| field.to_string());
            field_col.insert(field.to_string(), colname.clone());
            t.columns.push(Column { name: colname.clone(), data_type: base.to_ascii_lowercase(), nullable: ty.ends_with('?'), primary_key: l.contains("@id") });
            if l.contains("@unique") {
                t.indexes.push(IndexDef { name: String::new(), columns: vec![colname], unique: true });
            }
        }
        schema.upsert(t);
    }
}

// ---------------------------------------------------------------- Drizzle

fn balanced(src: &str, open_at: usize) -> Option<(usize, &str)> {
    let b = src.as_bytes();
    let (open, close) = match b.get(open_at)? {
        b'{' => (b'{', b'}'),
        b'(' => (b'(', b')'),
        b'[' => (b'[', b']'),
        _ => return None,
    };
    let mut depth = 0;
    let mut quote: Option<u8> = None;
    for i in open_at..b.len() {
        let c = b[i];
        if let Some(q) = quote {
            if c == q && b[i - 1] != b'\\' {
                quote = None;
            }
            continue;
        }
        match c {
            b'"' | b'\'' | b'`' => quote = Some(c),
            _ if c == open => depth += 1,
            _ if c == close => {
                depth -= 1;
                if depth == 0 {
                    return Some((i, &src[open_at + 1..i]));
                }
            }
            _ => {}
        }
    }
    None
}

pub fn parse_drizzle(schema: &mut DbSchema, path: &str, src: &str) {
    let re_table = Regex::new(r#"(?:export\s+)?const\s+(\w+)\s*=\s*(?:\w+\.)?(pgTable|mysqlTable|sqliteTable|table)\s*\(\s*["'`]([^"'`]+)["'`]\s*,\s*"#).unwrap();
    let re_type = Regex::new(r#"^\s*(\w+)\s*\(\s*(?:["'`]([^"'`]+)["'`])?"#).unwrap();
    let re_ref = Regex::new(r"\.references\(\s*\(\)\s*(?::\s*\w+\s*)?=>\s*(\w+)\.(\w+)").unwrap();
    let mut var_table: HashMap<String, String> = HashMap::new();
    for c in re_table.captures_iter(src) {
        var_table.insert(c[1].to_string(), c[3].to_string());
    }
    for c in re_table.captures_iter(src) {
        let m = c.get(0).unwrap();
        let mut at = m.end();
        // `pgTable("x", (t) => ({ ... }))` form: skip to the object literal.
        while at < src.len() && !matches!(src.as_bytes()[at], b'{') {
            if src.as_bytes()[at] == b')' {
                break;
            }
            at += 1;
        }
        let Some((_, body)) = balanced(src, at) else { continue };
        let mut t = Table { name: c[3].to_string(), kind: "table".into(), source: format!("{path}:{}", line_at(src, m.start())), aliases: vec![c[1].to_string()], ..Default::default() };
        for (_, prop) in split_top(body, ',') {
            let prop = prop.trim();
            let Some((key, val)) = prop.split_once(':') else { continue };
            let key = key.trim().trim_matches(|x| x == '"' || x == '\'');
            if key.is_empty() || key.contains(' ') {
                continue;
            }
            let Some(tc) = re_type.captures(val) else { continue };
            let colname = tc.get(2).map(|m| m.as_str().to_string()).unwrap_or_else(|| key.to_string());
            let col = Column { name: colname.clone(), data_type: tc[1].to_ascii_lowercase(), nullable: !val.contains(".notNull()") && !val.contains(".primaryKey()"), primary_key: val.contains(".primaryKey()") };
            if val.contains(".unique()") {
                t.indexes.push(IndexDef { name: String::new(), columns: vec![colname.clone()], unique: true });
            }
            if let Some(r) = re_ref.captures(val) {
                let ref_table = var_table.get(&r[1]).cloned().unwrap_or_else(|| r[1].to_string());
                t.foreign_keys.push(ForeignKey { columns: vec![colname.clone()], ref_table, ref_columns: vec![to_snake(&r[2])] });
            }
            t.columns.push(col);
        }
        schema.upsert(t);
    }
}

// ---------------------------------------------------------------- SQLAlchemy

pub fn parse_sqlalchemy(schema: &mut DbSchema, path: &str, src: &str) {
    let re_class = Regex::new(r"(?m)^([ \t]*)class\s+(\w+)\s*(?:\([^)]*\))?\s*:").unwrap();
    let re_tn = Regex::new(r#"__tablename__\s*=\s*["']([^"']+)["']"#).unwrap();
    let re_col = Regex::new(r"^(\w+)\s*(?::\s*([^=]+))?=\s*(?:\w+\.)?(Column|mapped_column)\s*\((.*)$").unwrap();
    let re_fk = Regex::new(r#"ForeignKey\(\s*["']([^"'.]+(?:\.[^"'.]+)?)\.([^"'.]+)["']"#).unwrap();
    let re_str = Regex::new(r#"^\s*["']([^"']+)["']"#).unwrap();
    let re_fk_attr = Regex::new(r"ForeignKey\(\s*([A-Za-z_]\w*)\.(\w+)\s*[,)]").unwrap();
    let re_type = Regex::new(r"(?:^|,)\s*(?:sa\.|db\.)?([A-Z]\w*)").unwrap();
    let re_mapped = Regex::new(r"Mapped\[\s*(?:Optional\[)?\s*([\w.]+)").unwrap();
    let lines: Vec<&str> = src.lines().collect();
    for c in re_class.captures_iter(src) {
        let indent = c[1].len();
        let start_line = line_at(src, c.get(0).unwrap().start()) as usize;
        let mut body: Vec<&str> = Vec::new();
        for l in lines.iter().skip(start_line) {
            if !l.trim().is_empty() && l.len() - l.trim_start().len() <= indent {
                break;
            }
            body.push(l);
        }
        let text = body.join("\n");
        // Flask-SQLAlchemy / declarative models without __tablename__ use the snake-cased class name.
        let header = c.get(0).unwrap().as_str();
        let name = match re_tn.captures(&text) {
            Some(tn) => tn[1].to_string(),
            None if (header.contains("Model") || header.contains("Base")) && (text.contains("Column(") || text.contains("mapped_column(")) => to_snake(&c[2]),
            None => continue,
        };
        let mut t = Table { name, kind: "table".into(), source: format!("{path}:{start_line}"), aliases: vec![c[2].to_string()], ..Default::default() };
        // Join continuation lines of multi-line Column(...) calls.
        let mut stmts: Vec<String> = Vec::new();
        let mut depth = 0i32;
        for l in &body {
            let trimmed = l.trim();
            if depth > 0 {
                if let Some(last) = stmts.last_mut() {
                    last.push(' ');
                    last.push_str(trimmed);
                }
            } else {
                stmts.push(trimmed.to_string());
            }
            depth += trimmed.matches('(').count() as i32 - trimmed.matches(')').count() as i32;
            depth = depth.max(0);
        }
        for s in stmts {
            let Some(m) = re_col.captures(&s) else { continue };
            let attr = m[1].to_string();
            let args = &m[4];
            let name = re_str.captures(args).map(|x| x[1].to_string()).unwrap_or(attr);
            let ty = m.get(2).and_then(|a| re_mapped.captures(a.as_str()).map(|x| x[1].to_string())).or_else(|| re_type.captures(args).map(|x| x[1].to_string())).unwrap_or_default();
            let pk = args.contains("primary_key=True");
            let nullable = !(args.contains("nullable=False") || pk) && !m.get(2).map_or(false, |a| !a.as_str().contains("Optional") && !a.as_str().contains("None"));
            if let Some(fk) = re_fk.captures(args) {
                t.foreign_keys.push(ForeignKey { columns: vec![name.clone()], ref_table: fk[1].to_string(), ref_columns: vec![fk[2].to_string()] });
            } else if let Some(fk) = re_fk_attr.captures(args) {
                // ForeignKey(User.id): the class name resolves through table aliases.
                t.foreign_keys.push(ForeignKey { columns: vec![name.clone()], ref_table: fk[1].to_string(), ref_columns: vec![fk[2].to_string()] });
            }
            if args.contains("unique=True") {
                t.indexes.push(IndexDef { name: String::new(), columns: vec![name.clone()], unique: true });
            }
            t.columns.push(Column { name, data_type: ty.to_ascii_lowercase(), nullable, primary_key: pk });
        }
        schema.upsert(t);
    }
}

// ---------------------------------------------------------------- Django

/// True for `models.py` and for modules inside a `models/` package.
fn is_django_models_file(lower: &str) -> bool {
    let name = lower.rsplit('/').next().unwrap_or(lower);
    name == "models.py" || lower.contains("/models/")
}

/// The app label of a models file: the Django app directory it sits in.
fn django_app_label(path: &str) -> String {
    let segs: Vec<&str> = path.split('/').collect();
    match segs.iter().rposition(|s| *s == "models.py" || *s == "models") {
        Some(i) if i > 0 => segs[i - 1].to_string(),
        _ => String::new(),
    }
}

#[derive(Debug, Default, Clone)]
struct DjangoField {
    /// Attribute name (`author`).
    name: String,
    /// Column name: `db_column=`, else the attribute name (`author_id` for a relation).
    column: String,
    /// Field class as written: `CharField`, `ForeignKey`, …
    type_name: String,
    primary_key: bool,
    nullable: bool,
    unique: bool,
    db_index: bool,
    /// Relation target as written: `"auth.User"`, `self`, `settings.AUTH_USER_MODEL` or a class name.
    target: Option<String>,
    to_field: Option<String>,
    /// `ManyToManyField`: this model has no column; Django owns a join table.
    many_to_many: bool,
    /// `through=` model: the join table is a model of its own, so nothing is implied.
    through: Option<String>,
    /// `db_table=` on the join table.
    join_table: Option<String>,
}

/// One `class Meta` body. Every attribute is optional, so a
/// `class Meta(Base.Meta)` chain resolves the way Django's attribute lookup
/// does: the nearest declaration wins.
#[derive(Debug, Default, Clone)]
struct DjangoMeta {
    app_label: Option<String>,
    db_table: Option<String>,
    indexes: Option<Vec<IndexDef>>,
    /// `Meta.constraints`, resolved separately: a model that declares
    /// constraints still inherits its base's `indexes`.
    constraints: Option<Vec<IndexDef>>,
    /// `indexes = [*Base.Meta.indexes, …]` / `indexes.extend(Base.Meta.indexes)`:
    /// the base names spliced in.
    index_splats: Vec<String>,
    constraint_splats: Vec<String>,
    unique_together: Option<Vec<Vec<String>>>,
    index_together: Option<Vec<Vec<String>>>,
}

#[derive(Debug, Default, Clone)]
struct DjangoClass {
    name: String,
    bases: Vec<String>,
    line: u32,
    fields: Vec<DjangoField>,
    /// `class Meta`: `abstract` and `proxy` are never inherited (Django resets
    /// `abstract` before installing an abstract base's Meta).
    has_meta: bool,
    is_abstract: bool,
    proxy: bool,
    /// `class Meta(Base.Meta)`: the `Base` names whose Meta is extended.
    meta_bases: Vec<String>,
    meta: DjangoMeta,
}

/// One Django models file: its path, app label and classes.
struct DjangoFile {
    path: String,
    app: String,
    classes: Vec<DjangoClass>,
}

fn py_text<'a>(src: &'a str, node: tree_sitter::Node) -> &'a str {
    node.utf8_text(src.as_bytes()).unwrap_or("")
}

/// A Python string without its quotes; any other node verbatim.
fn py_value(src: &str, node: tree_sitter::Node) -> String {
    let t = py_text(src, node).trim();
    if node.kind() == "string" {
        t.trim_matches(|c| c == '"' || c == '\'').to_string()
    } else {
        t.to_string()
    }
}

fn py_true(src: &str, node: tree_sitter::Node) -> bool {
    matches!(py_text(src, node).trim(), "True" | "true")
}

/// Last segment of a (possibly dotted) Python name: `models.CharField` -> `CharField`.
fn py_name(text: &str) -> &str {
    text.trim().rsplit('.').next().unwrap_or(text).trim()
}

/// Positional and keyword arguments of a call.
fn py_args<'a>(src: &str, call: tree_sitter::Node<'a>) -> (Vec<tree_sitter::Node<'a>>, HashMap<String, tree_sitter::Node<'a>>) {
    let mut positional = Vec::new();
    let mut kw = HashMap::new();
    if let Some(args) = call.child_by_field_name("arguments") {
        let mut cur = args.walk();
        for a in args.named_children(&mut cur) {
            if a.kind() == "keyword_argument" {
                if let (Some(n), Some(v)) = (a.child_by_field_name("name"), a.child_by_field_name("value")) {
                    kw.insert(py_text(src, n).to_string(), v);
                }
            } else {
                positional.push(a);
            }
        }
    }
    (positional, kw)
}

/// String entries of a list/tuple.
fn py_string_list(src: &str, node: tree_sitter::Node) -> Vec<String> {
    if node.kind() == "string" {
        return vec![py_value(src, node)];
    }
    if !matches!(node.kind(), "list" | "tuple" | "set") {
        return Vec::new();
    }
    let mut cur = node.walk();
    node.named_children(&mut cur).filter(|e| e.kind() == "string").map(|e| py_value(src, e)).collect()
}

/// `unique_together` / `index_together`: one group of column names per constraint.
fn py_string_groups(src: &str, node: tree_sitter::Node) -> Vec<Vec<String>> {
    if !matches!(node.kind(), "list" | "tuple" | "set") {
        return Vec::new();
    }
    let mut cur = node.walk();
    let kids: Vec<tree_sitter::Node> = node.named_children(&mut cur).collect();
    if kids.iter().all(|k| k.kind() == "string") {
        return vec![kids.iter().map(|k| py_value(src, *k)).collect()];
    }
    kids.iter().map(|k| py_string_list(src, *k)).filter(|g| !g.is_empty()).collect()
}

/// `Meta.indexes` / `Meta.constraints`: `models.Index(...)` and `models.UniqueConstraint(...)`.
fn py_indexes(src: &str, node: tree_sitter::Node) -> Vec<IndexDef> {
    let mut out = Vec::new();
    if !matches!(node.kind(), "list" | "tuple" | "set") {
        return out;
    }
    let mut cur = node.walk();
    for e in node.named_children(&mut cur) {
        if e.kind() != "call" {
            continue;
        }
        let Some(callee) = e.child_by_field_name("function") else { continue };
        let ty = py_name(py_text(src, callee));
        // `Index`, `UniqueConstraint` and the backend index classes that end in
        // `Index` (`GinIndex`, `BTreeIndex`, `SpGistIndex`, …).
        if ty != "UniqueConstraint" && !ty.ends_with("Index") {
            continue;
        }
        let (pos, kw) = py_args(src, e);
        // `Index(fields=["a", "-b"], name=…)` or `Index("a", name=…)`.
        let columns: Vec<String> = match kw.get("fields") {
            Some(f) => py_string_list(src, *f),
            None => pos.iter().filter(|n| n.kind() == "string").map(|n| py_value(src, *n)).collect(),
        };
        let columns: Vec<String> = columns.into_iter().map(|c| c.trim_start_matches('-').to_string()).filter(|c| !c.is_empty()).collect();
        if columns.is_empty() {
            continue;
        }
        out.push(IndexDef { name: kw.get("name").map(|n| py_value(src, *n)).unwrap_or_default(), columns, unique: ty == "UniqueConstraint" });
    }
    out
}

/// One assignment in a class body, as (left, right).
fn py_assignment<'a>(stmt: tree_sitter::Node<'a>) -> Option<(tree_sitter::Node<'a>, tree_sitter::Node<'a>)> {
    let inner = if stmt.kind() == "expression_statement" { stmt.named_child(0)? } else { return None };
    if inner.kind() != "assignment" {
        return None;
    }
    Some((inner.child_by_field_name("left")?, inner.child_by_field_name("right")?))
}

/// `indexes.extend(Base.Meta.indexes)` inside a Meta body, as
/// (attribute, base class name).
fn py_meta_extends(src: &str, stmt: tree_sitter::Node) -> Option<(String, String)> {
    if stmt.kind() != "expression_statement" {
        return None;
    }
    let call = stmt.named_child(0)?;
    if call.kind() != "call" {
        return None;
    }
    let target = py_text(src, call.child_by_field_name("function")?).trim();
    let (attr, _) = target.rsplit_once(".extend")?;
    if !attr.ends_with("indexes") && !attr.ends_with("constraints") {
        return None;
    }
    let args = call.child_by_field_name("arguments")?;
    let mut cur = args.walk();
    let text = py_text(src, args.named_children(&mut cur).next()?).trim();
    for suffix in [".Meta.indexes", ".Meta.constraints"] {
        if let Some(name) = text.strip_suffix(suffix) {
            return Some((attr.to_string(), name.trim().to_string()));
        }
    }
    None
}

/// A field declaration: `name = models.CharField(...)`.
fn django_field(src: &str, attr: &str, call: tree_sitter::Node) -> Option<DjangoField> {
    let ty = py_name(py_text(src, call.child_by_field_name("function")?));
    let relation = matches!(ty, "ForeignKey" | "OneToOneField" | "ManyToManyField");
    // Fields end in `Field`, relations in `Key`/`Relation`; `Manager()`, `Index()`,
    // `Q()` and the virtual `GenericForeignKey` are none of them.
    if !relation && !ty.ends_with("Field") {
        return None;
    }
    let (pos, kw) = py_args(src, call);
    let mut f = DjangoField { name: attr.to_string(), type_name: ty.to_string(), ..Default::default() };
    f.primary_key = kw.get("primary_key").map(|n| py_true(src, *n)).unwrap_or(false);
    f.nullable = kw.get("null").map(|n| py_true(src, *n)).unwrap_or(false) && !f.primary_key;
    f.unique = kw.get("unique").map(|n| py_true(src, *n)).unwrap_or(false) || ty == "OneToOneField";
    // Django indexes relation columns unless told otherwise.
    f.db_index = match kw.get("db_index") {
        Some(n) => py_true(src, *n),
        None => relation && ty != "ManyToManyField",
    };
    let column = kw.get("db_column").map(|n| py_value(src, *n)).filter(|s| !s.is_empty());
    f.column = column.unwrap_or_else(|| if relation && ty != "ManyToManyField" { format!("{attr}_id") } else { attr.to_string() });
    if ty == "ManyToManyField" {
        f.many_to_many = true;
        f.through = kw.get("through").map(|n| py_value(src, *n)).filter(|s| !s.is_empty());
        f.join_table = kw.get("db_table").map(|n| py_value(src, *n)).filter(|s| !s.is_empty());
    }
    if relation {
        f.target = kw.get("to").map(|n| py_value(src, *n)).or_else(|| pos.first().map(|n| py_value(src, *n))).filter(|s| !s.is_empty());
        f.to_field = kw.get("to_field").map(|n| py_value(src, *n)).filter(|s| !s.is_empty());
    }
    Some(f)
}

fn django_meta(src: &str, class: tree_sitter::Node, c: &mut DjangoClass) {
    c.has_meta = true;
    if let Some(sup) = class.child_by_field_name("superclasses") {
        let mut cur = sup.walk();
        for b in sup.named_children(&mut cur) {
            // `class Meta(Base.Meta)` extends the base's Meta.
            if let Some(name) = py_text(src, b).trim().strip_suffix(".Meta") {
                if !name.is_empty() {
                    c.meta_bases.push(name.to_string());
                }
            }
        }
    }
    let Some(body) = class.child_by_field_name("body") else { return };
    let mut cur = body.walk();
    for stmt in body.named_children(&mut cur) {
        let Some((left, right)) = py_assignment(stmt) else {
            if let Some((attr, name)) = py_meta_extends(src, stmt) {
                if attr.ends_with("constraints") {
                    c.meta.constraint_splats.push(name);
                } else {
                    c.meta.index_splats.push(name);
                }
            }
            continue;
        };
        if left.kind() != "identifier" {
            continue;
        }
        match py_text(src, left) {
            "abstract" => c.is_abstract = py_true(src, right),
            "proxy" => c.proxy = py_true(src, right),
            "db_table" => c.meta.db_table = Some(py_value(src, right)),
            "app_label" => c.meta.app_label = Some(py_value(src, right)),
            "indexes" | "constraints" => {
                let constraints = py_text(src, left) == "constraints";
                // `[*Base.Meta.indexes, Index(…)]` splices the base's list in.
                let mut cur = right.walk();
                for e in right.named_children(&mut cur) {
                    if e.kind() != "list_splat" {
                        continue;
                    }
                    let text = py_text(src, e).trim().trim_start_matches('*');
                    for suffix in [".Meta.indexes", ".Meta.constraints"] {
                        if let Some(name) = text.strip_suffix(suffix) {
                            let name = name.trim().to_string();
                            let is_constraint = suffix.ends_with("constraints");
                            if is_constraint {
                                c.meta.constraint_splats.push(name);
                            } else {
                                c.meta.index_splats.push(name);
                            }
                        }
                    }
                }
                let parsed = py_indexes(src, right);
                let slot = if constraints { &mut c.meta.constraints } else { &mut c.meta.indexes };
                let mut v = slot.take().unwrap_or_default();
                v.extend(parsed);
                *slot = Some(v);
            }
            "unique_together" => {
                let mut v = c.meta.unique_together.take().unwrap_or_default();
                v.extend(py_string_groups(src, right));
                c.meta.unique_together = Some(v);
            }
            "index_together" => {
                let mut v = c.meta.index_together.take().unwrap_or_default();
                v.extend(py_string_groups(src, right));
                c.meta.index_together = Some(v);
            }
            _ => {}
        }
    }
}

/// One top-level `class X(...)` in a models file.
fn django_class(src: &str, node: tree_sitter::Node) -> Option<DjangoClass> {
    let name = py_text(src, node.child_by_field_name("name")?).to_string();
    if name.is_empty() {
        return None;
    }
    let mut c = DjangoClass { name, line: line_at(src, node.start_byte()), ..Default::default() };
    if let Some(sup) = node.child_by_field_name("superclasses") {
        let mut cur = sup.walk();
        for b in sup.named_children(&mut cur) {
            let t = py_text(src, b).trim();
            if !t.is_empty() {
                c.bases.push(t.to_string());
            }
        }
    }
    let Some(body) = node.child_by_field_name("body") else { return Some(c) };
    let mut cur = body.walk();
    for stmt in body.named_children(&mut cur) {
        if stmt.kind() == "class_definition" {
            let inner = stmt.child_by_field_name("name").map(|n| py_text(src, n));
            if inner == Some("Meta") {
                django_meta(src, stmt, &mut c);
            }
            continue;
        }
        let Some((left, right)) = py_assignment(stmt) else { continue };
        if left.kind() != "identifier" || right.kind() != "call" {
            continue;
        }
        if let Some(f) = django_field(src, py_text(src, left), right) {
            c.fields.push(f);
        }
    }
    Some(c)
}

/// Every top-level class in one models file, parsed with the tree-sitter Python grammar.
fn django_classes(src: &str) -> Vec<DjangoClass> {
    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&Lang::Python.grammar()).is_err() {
        return Vec::new();
    }
    let Some(tree) = parser.parse(src, None) else { return Vec::new() };
    let root = tree.root_node();
    let mut cur = root.walk();
    root.named_children(&mut cur).filter(|n| n.kind() == "class_definition").filter_map(|n| django_class(src, n)).collect()
}

/// A class is a Django model when a base is `Model` (`models.Model`,
/// `django.db.models.Model`) or another model class in the repository.
fn django_is_model(name: &str, by_name: &HashMap<&str, &DjangoClass>, memo: &mut HashMap<String, bool>, depth: u32) -> bool {
    if depth > 16 {
        return false;
    }
    if let Some(v) = memo.get(name) {
        return *v;
    }
    let Some(c) = by_name.get(name) else { return false };
    memo.insert(name.to_string(), false); // break inheritance cycles
    let ok = c.bases.iter().any(|b| {
        let last = py_name(b);
        // `models.Model`, an abstract base in the repository, or one that comes
        // from a library (`AbstractUser`, `TimeStampedModel`, `BaseModel`).
        last == "Model"
            || last.starts_with("Abstract")
            || last.ends_with("Model")
            || django_is_model(last, by_name, memo, depth + 1)
    });
    memo.insert(name.to_string(), ok);
    ok
}

/// `Meta.db_table`, or Django's default `<app_label>_<modelname lower>`.
/// `%(app_label)s` and `%(class)s` are filled in, as Django does.
fn django_table_name(app: &str, name: &str, db_table: Option<&str>) -> String {
    let lower = name.to_lowercase();
    let table = db_table.filter(|s| !s.is_empty()).map(str::to_string).unwrap_or_else(|| {
        if app.is_empty() {
            lower.clone()
        } else {
            format!("{app}_{lower}")
        }
    });
    if table.contains('%') {
        table.replace("%(app_label)s", app).replace("%(class)s", &lower)
    } else {
        table
    }
}

/// Resolution context: every models file in the repository at once, so an
/// abstract base in another app, a class-valued `ForeignKey` and a
/// `"app.Model"` string all land on the right table.
struct DjangoCtx<'a> {
    by_name: HashMap<&'a str, &'a DjangoClass>,
    /// Class name -> is a concrete model (a table).
    concrete: HashMap<String, bool>,
    /// `"Model"` and `"app.Model"` -> table name.
    table_of: HashMap<String, String>,
    auth_user_model: Option<&'a str>,
}

impl<'a> DjangoCtx<'a> {
    fn new(files: &'a [DjangoFile], auth_user_model: Option<&'a str>) -> DjangoCtx<'a> {
        let mut by_name: HashMap<&str, &DjangoClass> = HashMap::new();
        for f in files {
            for c in &f.classes {
                by_name.entry(c.name.as_str()).or_insert(c);
            }
        }
        let mut memo = HashMap::new();
        let mut concrete = HashMap::new();
        for f in files {
            for c in &f.classes {
                let is_model = django_is_model(&c.name, &by_name, &mut memo, 0);
                concrete.insert(c.name.clone(), is_model && !c.is_abstract && !c.proxy);
            }
        }
        let mut table_of: HashMap<String, String> = HashMap::new();
        for f in files {
            for c in &f.classes {
                if !concrete.get(&c.name).copied().unwrap_or(false) {
                    continue;
                }
                let app = c.meta.app_label.clone().unwrap_or_else(|| f.app.clone());
                let table = django_table_name(&app, &c.name, c.meta.db_table.as_deref());
                table_of.entry(format!("{app}.{}", c.name)).or_insert(table.clone());
                table_of.entry(c.name.clone()).or_insert(table);
            }
        }
        DjangoCtx { by_name, concrete, table_of, auth_user_model }
    }

    /// The `class Meta` a model uses: its own, or the nearest base that
    /// declares one (Django's `getattr(cls, "Meta")`).
    fn nearest_meta(&self, c: &'a DjangoClass, depth: u32) -> Option<&'a DjangoClass> {
        if c.has_meta {
            return Some(c);
        }
        if depth > 16 {
            return None;
        }
        for b in &c.bases {
            if let Some(base) = self.by_name.get(py_name(b)) {
                if let Some(m) = self.nearest_meta(base, depth + 1) {
                    return Some(m);
                }
            }
        }
        None
    }

    /// Meta declarations in lookup order: the model's own (or the nearest
    /// base's), then the ones it extends with `class Meta(Base.Meta)`.
    fn meta_chain(&self, c: &'a DjangoClass, out: &mut Vec<&'a DjangoClass>, depth: u32) {
        if depth > 16 {
            return;
        }
        let Some(owner) = self.nearest_meta(c, 0) else { return };
        out.push(owner);
        for b in &owner.meta_bases {
            if let Some(base) = self.by_name.get(py_name(b)) {
                self.meta_chain(base, out, depth + 1);
            }
        }
    }

    /// The Meta attributes a model ends up with: the first declaration of each
    /// wins, like Python attribute lookup on the Meta class.
    fn meta_of(&self, c: &'a DjangoClass) -> DjangoMeta {
        self.meta_at(c, 0)
    }

    fn meta_at(&self, c: &'a DjangoClass, depth: u32) -> DjangoMeta {
        let mut chain: Vec<&DjangoClass> = Vec::new();
        self.meta_chain(c, &mut chain, 0);
        let mut m = DjangoMeta::default();
        for owner in chain {
            let o = &owner.meta;
            if m.app_label.is_none() {
                m.app_label.clone_from(&o.app_label);
            }
            if m.db_table.is_none() {
                m.db_table.clone_from(&o.db_table);
            }
            if m.indexes.is_none() && (o.indexes.is_some() || !o.index_splats.is_empty()) {
                let mut v: Vec<IndexDef> = Vec::new();
                for s in &o.index_splats {
                    if depth > 16 {
                        break;
                    }
                    if let Some(base) = self.by_name.get(py_name(s)) {
                        v.extend(self.meta_at(base, depth + 1).indexes.unwrap_or_default());
                    }
                }
                v.extend(o.indexes.clone().unwrap_or_default());
                m.indexes = Some(v);
            }
            if m.constraints.is_none() && (o.constraints.is_some() || !o.constraint_splats.is_empty()) {
                let mut v: Vec<IndexDef> = Vec::new();
                for s in &o.constraint_splats {
                    if depth > 16 {
                        break;
                    }
                    if let Some(base) = self.by_name.get(py_name(s)) {
                        v.extend(self.meta_at(base, depth + 1).constraints.unwrap_or_default());
                    }
                }
                v.extend(o.constraints.clone().unwrap_or_default());
                m.constraints = Some(v);
            }
            if m.unique_together.is_none() {
                m.unique_together.clone_from(&o.unique_together);
            }
            if m.index_together.is_none() {
                m.index_together.clone_from(&o.index_together);
            }
        }
        m
    }

    /// Model ancestors of `c`, nearest first, resolved through class names.
    fn ancestors(&self, c: &DjangoClass) -> Vec<&'a DjangoClass> {
        let mut out: Vec<&'a DjangoClass> = Vec::new();
        let mut seen: HashSet<&str> = HashSet::new();
        let mut stack: Vec<&'a DjangoClass> = c.bases.iter().filter_map(|b| self.by_name.get(py_name(b)).copied()).collect();
        while let Some(p) = stack.pop() {
            if !seen.insert(p.name.as_str()) {
                continue;
            }
            out.push(p);
            stack.extend(p.bases.iter().filter_map(|b| self.by_name.get(py_name(b)).copied()));
        }
        out
    }

    /// Columns of a concrete model: its own fields plus the ones Django copies
    /// from abstract bases (a subclass inherits their columns).
    fn fields(&self, c: &'a DjangoClass) -> Vec<&'a DjangoField> {
        let mut out: Vec<&'a DjangoField> = Vec::new();
        let mut seen: HashSet<&str> = HashSet::new();
        self.abstract_fields(c, &mut out, &mut seen, 0);
        for f in &c.fields {
            if seen.insert(f.name.as_str()) {
                out.push(f);
            }
        }
        out
    }

    /// Fields the abstract bases of `c` contribute, deepest base first. A
    /// concrete model ancestor stops the walk: under multi-table inheritance
    /// its columns (including the ones it copied from abstract bases) live in
    /// its own table, which the child reaches through its parent link.
    fn abstract_fields(&self, c: &'a DjangoClass, out: &mut Vec<&'a DjangoField>, seen: &mut HashSet<&'a str>, depth: u32) {
        if depth > 16 {
            return;
        }
        for b in &c.bases {
            let Some(base) = self.by_name.get(py_name(b)) else { continue };
            if base.is_abstract && !self.concrete.get(&base.name).copied().unwrap_or(false) {
                self.abstract_fields(base, out, seen, depth + 1);
                for f in &base.fields {
                    if seen.insert(f.name.as_str()) {
                        out.push(f);
                    }
                }
            }
        }
    }

    /// Concrete model ancestors: multi-table inheritance, where the child's
    /// primary key is a one-to-one link to the parent.
    fn concrete_bases(&self, c: &DjangoClass) -> Vec<&'a DjangoClass> {
        self.ancestors(c).into_iter().filter(|a| self.concrete.get(&a.name).copied().unwrap_or(false)).collect()
    }

    fn table_of_class(&self, name: &str) -> Option<&str> {
        self.table_of.get(name).map(|s| s.as_str())
    }

    /// The table a relation points at: `self`, a class name, `"app.Model"`, or
    /// the `AUTH_USER_MODEL` setting when the repository defines one.
    fn ref_table(&self, own: &str, target: &str) -> String {
        let t = target.trim();
        if t == "self" {
            return own.to_string();
        }
        if t == "settings.AUTH_USER_MODEL" {
            return match self.auth_user_model {
                Some(m) => self.ref_table(own, m),
                None => t.to_string(),
            };
        }
        if let Some((app, model)) = t.rsplit_once('.') {
            // Django's default name for `"app.Model"` when the model is not in this repository.
            return self.table_of_class(t).map(|s| s.to_string()).unwrap_or_else(|| format!("{app}_{}", model.to_lowercase()));
        }
        self.table_of_class(t).map(|s| s.to_string()).unwrap_or_else(|| t.to_string())
    }

    /// Model name a relation target stands for: `"auth.User"` and
    /// `settings.AUTH_USER_MODEL` both end up as `user`.
    fn target_model_name(&self, target: &str) -> String {
        let t = if target.trim() == "settings.AUTH_USER_MODEL" {
            self.auth_user_model.unwrap_or("").to_string()
        } else {
            target.trim().to_string()
        };
        if t.is_empty() || t == "self" {
            return String::new();
        }
        py_name(&t).to_lowercase()
    }

    /// Columns a relation points at: `to_field` (as its column), else the
    /// target's primary key.
    fn ref_columns(&self, own: &'a DjangoClass, target: &str, to_field: Option<&str>) -> Vec<String> {
        let t = if target.trim() == "settings.AUTH_USER_MODEL" {
            self.auth_user_model.unwrap_or(target)
        } else {
            target
        };
        if let Some(f) = to_field {
            // `to_field="name"` points at the `name` field's column
            // (`db_column="author_name"`).
            if target.trim() == "self" {
                return vec![self.column_of(&self.fields(own), f)];
            }
            if let Some(c) = self.by_name.get(py_name(t)) {
                return vec![self.column_of(&self.fields(c), f)];
            }
            return vec![f.to_string()];
        }
        if target.trim() == "self" {
            return vec![self.pk_column(own)];
        }
        match self.by_name.get(py_name(t)) {
            Some(c) => vec![self.pk_column(c)],
            None => vec!["id".to_string()],
        }
    }

    /// Primary key column of a model: its `primary_key=True` field, else `id`
    /// (or the parent link under multi-table inheritance).
    fn pk_column(&self, c: &DjangoClass) -> String {
        if let Some(f) = self.fields(c).into_iter().find(|f| f.primary_key) {
            return f.column.clone();
        }
        if let Some(p) = self.concrete_bases(c).first() {
            return format!("{}_ptr_id", p.name.to_lowercase());
        }
        "id".to_string()
    }

    /// Field name -> column, for `unique_together` and `index_together`.
    fn column_of(&self, fields: &[&DjangoField], name: &str) -> String {
        fields.iter().find(|f| f.name == name).map(|f| f.column.clone()).unwrap_or_else(|| name.to_string())
    }
}

fn django_file_table(ctx: &DjangoCtx, f: &DjangoFile, c: &DjangoClass) -> String {
    let meta = ctx.meta_of(c);
    let app = meta.app_label.clone().unwrap_or_else(|| f.app.clone());
    django_table_name(&app, &c.name, meta.db_table.as_deref())
}

/// Tables for one concrete model: columns, keys, indexes and the join tables
/// its `ManyToManyField`s own.
fn django_tables(schema: &mut DbSchema, ctx: &DjangoCtx, f: &DjangoFile, c: &DjangoClass) {
    let table = django_file_table(ctx, f, c);
    let source = format!("{}:{}", f.path, c.line);
    let mut t = Table { name: table.clone(), kind: "table".into(), source: source.clone(), aliases: vec![c.name.clone()], ..Default::default() };
    let fields = ctx.fields(c);
    let mut has_pk = false;
    for fl in &fields {
        if fl.many_to_many {
            continue;
        }
        if fl.primary_key {
            has_pk = true;
        }
        t.columns.push(Column { name: fl.column.clone(), data_type: fl.type_name.to_ascii_lowercase(), nullable: fl.nullable && !fl.primary_key, primary_key: fl.primary_key });
        if let Some(target) = &fl.target {
            t.foreign_keys.push(ForeignKey {
                columns: vec![fl.column.clone()],
                ref_table: ctx.ref_table(&table, target),
                ref_columns: ctx.ref_columns(c, target, fl.to_field.as_deref()),
            });
        }
        if fl.unique {
            t.indexes.push(IndexDef { name: String::new(), columns: vec![fl.column.clone()], unique: true });
        } else if fl.db_index {
            t.indexes.push(IndexDef { name: String::new(), columns: vec![fl.column.clone()], unique: false });
        }
    }
    let parents = ctx.concrete_bases(c);
    if let Some(p) = parents.first() {
        // Multi-table inheritance: the child's primary key links to the parent.
        let col = format!("{}_ptr_id", p.name.to_lowercase());
        let ref_table = ctx.table_of_class(&p.name).unwrap_or(&p.name).to_string();
        t.columns.insert(0, Column { name: col.clone(), data_type: "onetoonefield".into(), nullable: false, primary_key: true });
        t.foreign_keys.push(ForeignKey { columns: vec![col], ref_table, ref_columns: vec![ctx.pk_column(p)] });
    } else if !has_pk {
        // Django adds an auto primary key unless a field declares one.
        t.columns.insert(0, Column { name: "id".into(), data_type: "bigautofield".into(), nullable: false, primary_key: true });
    }
    // `Meta.indexes` / `constraints`, `unique_together` and `index_together`,
    // as inherited through `class Meta(Base.Meta)`.
    let meta = ctx.meta_of(c);
    let app_label = meta.app_label.clone().unwrap_or_else(|| f.app.clone());
    let model_name = c.name.to_lowercase();
    for mut i in meta.indexes.unwrap_or_default().into_iter().chain(meta.constraints.unwrap_or_default()) {
        // Django fills `%(class)s` / `%(app_label)s` in per model, so one
        // abstract base can name each child's index.
        if i.name.contains('%') {
            i.name = i.name.replace("%(app_label)s", &app_label).replace("%(class)s", &model_name);
        }
        t.indexes.push(i);
    }
    for g in meta.unique_together.unwrap_or_default() {
        t.indexes.push(IndexDef { name: String::new(), columns: g.iter().map(|n| ctx.column_of(&fields, n)).collect(), unique: true });
    }
    for g in meta.index_together.unwrap_or_default() {
        t.indexes.push(IndexDef { name: String::new(), columns: g.iter().map(|n| ctx.column_of(&fields, n)).collect(), unique: false });
    }
    schema.upsert(t);
    // A plain ManyToManyField gets an implicit join table `<table>_<field>`
    // with a primary key, a foreign key per side and a unique pair.
    for fl in &fields {
        if !fl.many_to_many || fl.through.is_some() {
            continue;
        }
        let target = fl.target.clone().unwrap_or_default();
        let target_table = ctx.ref_table(&table, &target);
        let self_ref = target == "self" || target_table == table;
        let own_name = c.name.to_lowercase();
        let target_name = {
            let n = ctx.target_model_name(&target);
            if n.is_empty() {
                py_name(&target).to_lowercase()
            } else {
                n
            }
        };
        let (from_col, to_col) = if self_ref {
            (format!("from_{own_name}_id"), format!("to_{own_name}_id"))
        } else {
            (format!("{own_name}_id"), format!("{target_name}_id"))
        };
        let mut j = Table { name: fl.join_table.clone().unwrap_or_else(|| format!("{table}_{}", fl.name)), kind: "table".into(), source: source.clone(), ..Default::default() };
        j.columns = vec![
            Column { name: "id".into(), data_type: "bigautofield".into(), nullable: false, primary_key: true },
            Column { name: from_col.clone(), data_type: "foreignkey".into(), nullable: false, primary_key: false },
            Column { name: to_col.clone(), data_type: "foreignkey".into(), nullable: false, primary_key: false },
        ];
        j.foreign_keys = vec![
            ForeignKey { columns: vec![from_col.clone()], ref_table: table.clone(), ref_columns: vec![ctx.pk_column(c)] },
            ForeignKey { columns: vec![to_col.clone()], ref_table: target_table, ref_columns: ctx.ref_columns(c, &target, None) },
        ];
        j.indexes = vec![
            IndexDef { name: String::new(), columns: vec![from_col.clone(), to_col.clone()], unique: true },
            IndexDef { name: String::new(), columns: vec![from_col, to_col], unique: false },
        ];
        schema.upsert(j);
    }
}

/// Django tables from one models file (base classes in the same file resolve).
pub fn parse_django(schema: &mut DbSchema, path: &str, src: &str) {
    let file = DjangoFile { path: path.to_string(), app: django_app_label(&path.to_ascii_lowercase()), classes: django_classes(src) };
    let files = [file];
    let ctx = DjangoCtx::new(&files, None);
    for f in &files {
        for c in &f.classes {
            if ctx.concrete.get(&c.name).copied().unwrap_or(false) {
                django_tables(schema, &ctx, f, c);
            }
        }
    }
}

/// Django tables from every models file in the repository.
fn django_schema(schema: &mut DbSchema, mut files: Vec<DjangoFile>, auth_user_model: Option<&str>) {
    if files.is_empty() {
        return;
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let ctx = DjangoCtx::new(&files, auth_user_model);
    for f in &files {
        for c in &f.classes {
            if ctx.concrete.get(&c.name).copied().unwrap_or(false) {
                django_tables(schema, &ctx, f, c);
            }
        }
    }
}

// ---------------------------------------------------------------- Diesel

pub fn parse_diesel(schema: &mut DbSchema, path: &str, src: &str) {
    let re_table = Regex::new(r"(?s)table!\s*\{\s*(?:(?:use\s[^;]*;|//[^\n]*|#\[[^\]]*\])\s*)*(?:(\w+)\.)?(\w+)\s*(?:\(([^)]*)\))?\s*\{([^}]*)\}").unwrap();
    let re_col = Regex::new(r"(?m)^\s*(?:#\[[^\]]*\]\s*)*(\w+)\s*->\s*([^,\n]+),?").unwrap();
    let re_join = Regex::new(r"joinable!\s*\(\s*(\w+)\s*->\s*(\w+)\s*\(\s*(\w+)\s*\)\s*\)").unwrap();
    for c in re_table.captures_iter(src) {
        let pks: Vec<String> = c.get(3).map(|p| p.as_str().split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()).unwrap_or_else(|| vec!["id".into()]);
        let mut t = Table {
            schema: c.get(1).map(|s| s.as_str().to_string()).filter(|s| s != "public"),
            name: c[2].to_string(),
            kind: "table".into(),
            source: format!("{path}:{}", line_at(src, c.get(0).unwrap().start())),
            aliases: vec![c[2].to_string()],
            ..Default::default()
        };
        for col in re_col.captures_iter(&c[4]) {
            let ty = col[2].trim().to_string();
            t.columns.push(Column { name: col[1].to_string(), nullable: ty.starts_with("Nullable<"), data_type: ty.to_ascii_lowercase(), primary_key: pks.contains(&col[1].to_string()) });
        }
        schema.upsert(t);
    }
    for j in re_join.captures_iter(src) {
        let child = j[1].to_string();
        let fk = ForeignKey { columns: vec![j[3].to_string()], ref_table: j[2].to_string(), ref_columns: Vec::new() };
        if let Some(t) = schema.find_mut(&child) {
            t.foreign_keys.push(fk);
        }
    }
}

// ---------------------------------------------------------------- from repo

/// Static schema from the repository's schema sources.
pub fn from_repo(index: &Index) -> DbSchema {
    let mut schema = DbSchema { origin: "repo".into(), ..Default::default() };
    let mut sql_files: Vec<&str> = Vec::new();
    for f in &index.files {
        let p = f.path.as_str();
        let lower = p.to_ascii_lowercase();
        let name = lower.rsplit('/').next().unwrap_or(&lower);
        // Down / rollback migrations undo schema; seeds and fixtures hold data.
        let rollback = name == "down.sql" || name.ends_with(".down.sql") || name.contains("rollback") || name.starts_with("down");
        if lower.ends_with(".sql") && !f.is_test && !rollback && !lower.contains("seed") && !lower.contains("fixture") {
            sql_files.push(p);
        }
    }
    // Migrations apply in path order (timestamps/sequence numbers sort).
    sql_files.sort();
    let read = |p: &str| std::fs::read_to_string(index.root.join(p)).ok();
    let mut sources: Vec<String> = Vec::new();
    for p in &sql_files {
        if let Some(src) = read(p) {
            let before = schema.tables.len();
            parse_sql(&mut schema, p, &src);
            if schema.tables.len() != before || src.to_ascii_lowercase().contains("alter table") {
                sources.push(p.to_string());
            }
        }
    }
    // ORM sources override/extend SQL (they carry model names for linking).
    let re_auth = Regex::new(r#"AUTH_USER_MODEL\s*[:=]\s*["']([\w.]+)["']"#).unwrap();
    let mut auth_user_model: Option<String> = None;
    // Django models are parsed after this loop: a base, a `ForeignKey` or a
    // `"app.Model"` string can name a model in another app.
    let mut django_files: Vec<DjangoFile> = Vec::new();
    for f in &index.files {
        let p = f.path.as_str();
        let lower = p.to_ascii_lowercase();
        if f.is_test {
            continue;
        }
        let parsed = if lower.ends_with(".prisma") {
            read(p).map(|s| parse_prisma(&mut schema, p, &s)).is_some()
        } else if lower.ends_with(".ts") || lower.ends_with(".js") || lower.ends_with(".mts") {
            match read(p) {
                Some(s) if s.contains("drizzle-orm") && (s.contains("pgTable(") || s.contains("mysqlTable(") || s.contains("sqliteTable(")) => {
                    parse_drizzle(&mut schema, p, &s);
                    true
                }
                _ => false,
            }
        } else if lower.ends_with(".py") {
            match read(p) {
                // Alembic revisions describe changes, not models.
                Some(s) if !lower.contains("/versions/") && !s.contains("from alembic import op") && (s.contains("__tablename__") || (s.contains("sqlalchemy") || s.contains("db.Model")) && (s.contains("Column(") || s.contains("mapped_column("))) => {
                    parse_sqlalchemy(&mut schema, p, &s);
                    true
                }
                // Django models: `models.py` and `models/` packages.
                Some(s) if is_django_models_file(&lower) && (s.contains("models.Model") || s.contains("django.db") || s.contains("django.contrib")) => {
                    django_files.push(DjangoFile { path: p.to_string(), app: django_app_label(&lower), classes: django_classes(&s) });
                    true
                }
                Some(s) => {
                    // `AUTH_USER_MODEL = "accounts.User"` names the model `settings.AUTH_USER_MODEL` stands for.
                    if lower.contains("settings") {
                        if let Some(c) = re_auth.captures(&s) {
                            auth_user_model = Some(c[1].to_string());
                        }
                    }
                    false
                }
                None => false,
            }
        } else if lower.ends_with(".rs") {
            match read(p) {
                Some(s) if s.contains("table!") && s.contains("->") => {
                    parse_diesel(&mut schema, p, &s);
                    true
                }
                _ => false,
            }
        } else {
            false
        };
        if parsed {
            sources.push(p.to_string());
        }
    }
    django_schema(&mut schema, django_files, auth_user_model.as_deref());
    sources.dedup();
    schema.sources = sources;
    schema.tables.sort_by(|a, b| a.key().cmp(&b.key()));
    schema
}

// ---------------------------------------------------------------- code links

/// Link each table to the code that queries it: raw SQL (`FROM users`), Prisma
/// (`prisma.user.findMany`), Diesel (`users::table`) and ORM models or
/// variables (Django `Post.objects`) used by files that import their
/// definition.
pub fn link_code(index: &Index, schema: &mut DbSchema) {
    if schema.tables.is_empty() {
        return;
    }
    let esc = |s: &str| regex::escape(s);
    let names: Vec<String> = schema.tables.iter().map(|t| t.name.clone()).collect();
    let alt = names.iter().map(|n| esc(n)).collect::<Vec<_>>().join("|");
    let Ok(re_sql) = Regex::new(&format!(r#"(?i)\b(?:from|join|into|update|table|exists)\s+["`\[]?(?:\w+["`\]]?\.["`\[]?)?({alt})\b"#)) else { return };
    let prisma: Vec<(usize, String)> = schema.tables.iter().enumerate().flat_map(|(i, t)| t.aliases.iter().filter(|a| a.chars().next().map_or(false, |c| c.is_ascii_lowercase())).map(move |a| (i, a.clone()))).collect();
    let re_prisma = if prisma.is_empty() { None } else { Regex::new(&format!(r"\b(?:prisma|db|tx|client)\.({})\s*\.\s*(?:find|create|update|delete|upsert|count|aggregate|groupBy)", prisma.iter().map(|(_, a)| esc(a)).collect::<Vec<_>>().join("|"))).ok() };
    let re_diesel = Regex::new(&format!(r"\b({alt})::(?:table|dsl|columns)\b")).ok();

    // Candidate files: those whose index mentions a table name or alias.
    let mut candidates: HashSet<u32> = HashSet::new();
    for t in &schema.tables {
        for term in std::iter::once(&t.name).chain(t.aliases.iter()) {
            candidates.extend(index.bm25.files_with(&term.to_ascii_lowercase()));
        }
    }
    let defining: HashSet<String> = schema.tables.iter().filter_map(|t| t.source.rsplit_once(':').map(|(f, _)| f.to_string())).collect();
    // ORM identifiers (class / variable names) resolve only in files that import the defining file.
    let mut alias_home: HashMap<String, (usize, u32)> = HashMap::new();
    for (i, t) in schema.tables.iter().enumerate() {
        if let Some((f, _)) = t.source.rsplit_once(':') {
            if let Some(&fid) = index.path_ix.get(f) {
                for a in &t.aliases {
                    if a.chars().next().map_or(false, |c| c.is_ascii_alphabetic()) && a.len() >= 3 {
                        alias_home.insert(a.clone(), (i, fid));
                    }
                }
            }
        }
    }
    let re_alias = if alias_home.is_empty() { None } else { Regex::new(&format!(r"\b({})\b", alias_home.keys().map(|a| esc(a)).collect::<Vec<_>>().join("|"))).ok() };

    let by_name: HashMap<String, usize> = names.iter().enumerate().map(|(i, n)| (n.to_ascii_lowercase(), i)).collect();
    let mut refs: Vec<Vec<CodeRef>> = vec![Vec::new(); schema.tables.len()];
    let mut ids: Vec<u32> = candidates.into_iter().collect();
    ids.sort();
    for fid in ids {
        let f = &index.files[fid as usize];
        if f.lang.is_none() || defining.contains(&f.path) {
            continue;
        }
        let Ok(src) = std::fs::read_to_string(index.root.join(&f.path)) else { continue };
        let imports: HashSet<u32> = index.file_out[fid as usize].iter().map(|&e| index.file_edges[e as usize].to).collect();
        let line_starts: Vec<usize> = std::iter::once(0).chain(src.match_indices('\n').map(|(i, _)| i + 1)).collect();
        let mut push = |ti: usize, byte: usize, via: &'static str| {
            let line = line_at(&src, byte);
            // Import / use lines name the model without querying it.
            let ls = line_starts[(line as usize - 1).min(line_starts.len() - 1)];
            let text = src[ls..].lines().next().unwrap_or("").trim_start();
            if text.starts_with("import ") || text.starts_with("from ") || text.starts_with("use ") || text.starts_with("pub use ") || text.starts_with("export {") || text.starts_with("} from") {
                return;
            }
            if refs[ti].iter().any(|r: &CodeRef| r.file == f.path && r.line == line) || refs[ti].len() >= 60 {
                return;
            }
            let symbol = index.symbol_at(fid, line).map(|s| index.symbols[s as usize].qualified());
            refs[ti].push(CodeRef { file: f.path.clone(), line, symbol, via });
        };
        for m in re_sql.captures_iter(&src) {
            if let Some(&ti) = by_name.get(&m[1].to_ascii_lowercase()) {
                push(ti, m.get(0).unwrap().start(), "sql");
            }
        }
        if let Some(re) = &re_prisma {
            for m in re.captures_iter(&src) {
                if let Some((ti, _)) = prisma.iter().find(|(_, a)| a == &m[1]) {
                    push(*ti, m.get(0).unwrap().start(), "prisma");
                }
            }
        }
        if let Some(re) = &re_diesel {
            for m in re.captures_iter(&src) {
                if let Some(&ti) = by_name.get(&m[1].to_ascii_lowercase()) {
                    push(ti, m.get(0).unwrap().start(), "diesel");
                }
            }
        }
        if let Some(re) = &re_alias {
            for m in re.captures_iter(&src) {
                if let Some(&(ti, home)) = alias_home.get(&m[1]) {
                    if imports.contains(&home) {
                        push(ti, m.get(0).unwrap().start(), "orm");
                    }
                }
            }
        }
    }
    for (t, r) in schema.tables.iter_mut().zip(refs) {
        t.used_by = r;
    }
}

// ---------------------------------------------------------------- output

fn fk_edges(schema: &DbSchema) -> Vec<(usize, usize, usize)> {
    let mut out = Vec::new();
    for (i, t) in schema.tables.iter().enumerate() {
        for (k, fk) in t.foreign_keys.iter().enumerate() {
            let (s, n) = split_name(&fk.ref_table);
            let hit = schema.tables.iter().position(|x| x.name.eq_ignore_ascii_case(&n) && (s.is_none() || x.schema == s)).or_else(|| schema.tables.iter().position(|x| x.aliases.iter().any(|a| a == &n)));
            if let Some(j) = hit {
                out.push((i, j, k));
            }
        }
    }
    out
}

fn col_summary(t: &Table, c: &Column) -> String {
    let mut s = format!("{} {}", c.name, if c.data_type.is_empty() { "?" } else { &c.data_type });
    if c.primary_key {
        s.push_str(" PK");
    } else if !c.nullable {
        s.push_str(" NOT NULL");
    }
    if let Some(fk) = t.foreign_keys.iter().find(|f| f.columns.len() == 1 && f.columns[0].eq_ignore_ascii_case(&c.name)) {
        s.push_str(&format!(" → {}{}", fk.ref_table, fk.ref_columns.first().map(|c| format!(".{c}")).unwrap_or_default()));
    }
    s
}

impl DbSchema {
    pub fn text(&self, focus: Option<&str>) -> String {
        let mut o = String::new();
        if self.tables.is_empty() {
            let _ = writeln!(o, "No database schema found. repomap reads SQL migrations, Prisma, Drizzle, SQLAlchemy, Diesel and Django models, or a live database with --url-env / url_env (Postgres, MySQL, SQLite; read-only).");
            return o;
        }
        let edges = fk_edges(self);
        let mut referenced_by: HashMap<usize, Vec<String>> = HashMap::new();
        for &(i, j, k) in &edges {
            let fk = &self.tables[i].foreign_keys[k];
            referenced_by.entry(j).or_default().push(format!("{}.{}", self.tables[i].key(), fk.columns.join(",")));
        }
        if let Some(name) = focus {
            let Some(t) = self.table(name) else {
                let _ = writeln!(o, "No table `{name}`. Tables: {}", self.tables.iter().map(|t| t.key()).collect::<Vec<_>>().join(", "));
                return o;
            };
            let i = self.tables.iter().position(|x| x.key() == t.key()).unwrap();
            let _ = writeln!(o, "# {} {} ({})", t.kind, t.key(), t.source);
            if t.aliases.len() > 0 {
                let _ = writeln!(o, "Code names: {}", t.aliases.join(", "));
            }
            let _ = writeln!(o, "\n## Columns ({})", t.columns.len());
            for c in &t.columns {
                let _ = writeln!(o, "- {}", col_summary(t, c));
            }
            if !t.indexes.is_empty() {
                let _ = writeln!(o, "\n## Indexes");
                for ix in &t.indexes {
                    let _ = writeln!(o, "- {}{} ({})", if ix.unique { "UNIQUE " } else { "" }, if ix.name.is_empty() { "-" } else { &ix.name }, ix.columns.join(", "));
                }
            }
            if let Some(rb) = referenced_by.get(&i) {
                let _ = writeln!(o, "\n## Referenced by ({})", rb.len());
                for r in rb {
                    let _ = writeln!(o, "- {r}");
                }
            }
            if !t.used_by.is_empty() {
                let _ = writeln!(o, "\n## Queried from ({})", t.used_by.len());
                for r in &t.used_by {
                    let _ = writeln!(o, "- {}:{}{} [{}]", r.file, r.line, r.symbol.as_ref().map(|s| format!(" {s}")).unwrap_or_default(), r.via);
                }
            }
            return o;
        }
        let fk_total: usize = self.tables.iter().map(|t| t.foreign_keys.len()).sum();
        let origin = if self.origin == "repo" {
            let sql = self.sources.iter().filter(|s| s.to_ascii_lowercase().ends_with(".sql")).count();
            let other: Vec<&String> = self.sources.iter().filter(|s| !s.to_ascii_lowercase().ends_with(".sql")).collect();
            let mut parts: Vec<String> = Vec::new();
            if sql > 3 {
                parts.push(format!("{sql} SQL migrations"));
            } else {
                parts.extend(self.sources.iter().filter(|s| s.to_ascii_lowercase().ends_with(".sql")).cloned());
            }
            parts.extend(other.iter().take(6).map(|s| s.to_string()));
            format!("from {}", parts.join(", "))
        } else {
            format!("live {}, read-only", self.origin)
        };
        let _ = writeln!(o, "# Database map ({origin})");
        let _ = writeln!(o, "{} tables, {} foreign keys, {} indexes, {} code references.", self.tables.len(), fk_total, self.tables.iter().map(|t| t.indexes.len()).sum::<usize>(), self.tables.iter().map(|t| t.used_by.len()).sum::<usize>());
        for (i, t) in self.tables.iter().enumerate() {
            let _ = writeln!(o, "\n## {}{} — {} columns ({})", t.key(), if t.kind == "view" { " (view)" } else { "" }, t.columns.len(), t.source);
            let cols: Vec<String> = t.columns.iter().take(14).map(|c| col_summary(t, c)).collect();
            if !cols.is_empty() {
                let _ = writeln!(o, "  {}{}", cols.join(" · "), if t.columns.len() > 14 { " · …" } else { "" });
            }
            if let Some(rb) = referenced_by.get(&i) {
                let _ = writeln!(o, "  referenced by: {}", rb.join(", "));
            }
            if !t.used_by.is_empty() {
                let top: Vec<String> = t.used_by.iter().take(5).map(|r| format!("{}:{}{}", r.file, r.line, r.symbol.as_ref().map(|s| format!(" ({s})")).unwrap_or_default())).collect();
                let _ = writeln!(o, "  queried from: {}{}", top.join(", "), if t.used_by.len() > 5 { format!(" … ({} total)", t.used_by.len()) } else { String::new() });
            }
        }
        o
    }

    /// Graph payload in the same shape as the code map, for the web UI.
    pub fn graph_json(&self, repo: &str, version: &str, web: Option<&str>, commit: Option<&str>) -> Value {
        let n = self.tables.len();
        let edges = fk_edges(self);
        let mut und: BTreeMap<(u32, u32), f32> = BTreeMap::new();
        let mut fe: Vec<crate::index::FileEdge> = Vec::new();
        for &(i, j, _) in &edges {
            if i == j {
                continue;
            }
            let key = if i < j { (i as u32, j as u32) } else { (j as u32, i as u32) };
            *und.entry(key).or_insert(0.0) += 1.0;
            fe.push(crate::index::FileEdge { from: i as u32, to: j as u32, imports: 1, calls: 0 });
        }
        let rank = crate::graph::pagerank(n, &fe);
        let und: Vec<(u32, u32, f32)> = und.into_iter().map(|((a, b), w)| (a, b, w)).collect();
        let raw = crate::graph::louvain(n, &und);
        // Communities: FK clusters; isolated tables share one group.
        let degree: Vec<usize> = (0..n).map(|i| und.iter().filter(|e| e.0 as usize == i || e.1 as usize == i).count()).collect();
        let mut groups: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
        for i in 0..n {
            let key = if degree[i] == 0 { u32::MAX } else { raw[i] };
            groups.entry(key).or_default().push(i);
        }
        let mut ordered: Vec<(u32, Vec<usize>)> = groups.into_iter().collect();
        ordered.sort_by(|a, b| (a.0 == u32::MAX).cmp(&(b.0 == u32::MAX)).then(b.1.len().cmp(&a.1.len())));
        let mut comm = vec![0u32; n];
        let mut communities = Vec::new();
        for (cid, (key, members)) in ordered.iter().enumerate() {
            for &m in members {
                comm[m] = cid as u32;
            }
            let name = if *key == u32::MAX {
                "standalone tables".to_string()
            } else {
                let top = members.iter().max_by(|a, b| rank[**a].partial_cmp(&rank[**b]).unwrap()).copied().unwrap_or(0);
                format!("{} group", self.tables[top].name)
            };
            communities.push(json!({"id": cid, "name": name, "size": members.len(), "kind": "core"}));
        }
        let nodes: Vec<Value> = self
            .tables
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let cols: Vec<Value> = t
                    .columns
                    .iter()
                    .map(|c| {
                        let is_fk = t.foreign_keys.iter().any(|f| f.columns.iter().any(|x| x.eq_ignore_ascii_case(&c.name)));
                        json!([format!("{}: {}", c.name, c.data_type), if c.primary_key { "pk" } else if is_fk { "fk" } else { "column" }, 0, 0, 0])
                    })
                    .collect();
                let (src_file, src_line) = t.source.rsplit_once(':').map(|(f, l)| (f.to_string(), l.parse::<u32>().unwrap_or(0))).unwrap_or((String::new(), 0));
                json!({
                    "p": t.key(),
                    "c": comm[i],
                    "r": (rank.get(i).copied().unwrap_or(0.0) * 10000.0).round() / 10000.0,
                    "l": t.columns.len(),
                    "g": t.kind,
                    "t": false,
                    "s": cols,
                    "q": t.used_by.iter().map(|r| json!([r.file, r.line, r.symbol, r.via])).collect::<Vec<_>>(),
                    "src": [src_file, src_line],
                    "ix": t.indexes.iter().map(|ix| json!([ix.name, ix.columns, ix.unique])).collect::<Vec<_>>(),
                })
            })
            .collect();
        let edge_json: Vec<Value> = edges.iter().filter(|(i, j, _)| i != j).map(|&(i, j, _)| json!([i, j, 1, 0])).collect();
        json!({
            "mode": "db",
            "version": version,
            "repo": repo,
            "web": web,
            "commit": commit,
            "origin": self.origin,
            "stats": {
                "files": n, "code_files": n,
                "symbols": self.tables.iter().map(|t| t.columns.len()).sum::<usize>(),
                "call_edges": self.tables.iter().map(|t| t.used_by.len()).sum::<usize>(),
                "file_edges": edge_json.len(),
                "index_ms": 0,
            },
            "communities": communities,
            "nodes": nodes,
            "edges": edge_json,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sql_migrations() {
        let mut s = DbSchema::default();
        parse_sql(&mut s, "m/001.sql", r#"
-- users
CREATE TABLE IF NOT EXISTS "public"."users" (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  email varchar(255) NOT NULL UNIQUE,
  org_id integer REFERENCES orgs(id) ON DELETE CASCADE,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE orgs (id serial, name text, PRIMARY KEY (id));
CREATE TABLE posts (
  id bigint NOT NULL,
  author_id uuid NOT NULL,
  body text,
  CONSTRAINT posts_pk PRIMARY KEY (id),
  CONSTRAINT posts_author_fk FOREIGN KEY (author_id) REFERENCES users (id)
);
CREATE UNIQUE INDEX posts_author_idx ON posts USING btree (author_id, id);
CREATE FUNCTION f() RETURNS trigger AS $$ BEGIN; select 1; END; $$ LANGUAGE plpgsql;
"#);
        parse_sql(&mut s, "m/002.sql", "ALTER TABLE posts ADD COLUMN title text NOT NULL, DROP COLUMN body; ALTER TABLE users RENAME COLUMN email TO email_address; DROP TABLE IF EXISTS orgs CASCADE;");
        let users = s.table("users").unwrap();
        assert!(users.columns.iter().any(|c| c.name == "id" && c.primary_key));
        assert!(users.columns.iter().any(|c| c.name == "email_address" && !c.nullable && c.data_type == "varchar(255)"), "{:?}", users.columns);
        assert_eq!(users.foreign_keys[0].ref_table, "orgs");
        let posts = s.table("posts").unwrap();
        assert!(posts.columns.iter().any(|c| c.name == "id" && c.primary_key));
        assert!(posts.columns.iter().any(|c| c.name == "title"));
        assert!(!posts.columns.iter().any(|c| c.name == "body"));
        assert_eq!(posts.foreign_keys[0].columns, vec!["author_id"]);
        assert!(posts.indexes.iter().any(|i| i.unique && i.columns == vec!["author_id", "id"]));
        assert!(s.table("orgs").is_none());
    }

    #[test]
    fn prisma() {
        let mut s = DbSchema::default();
        parse_prisma(&mut s, "prisma/schema.prisma", r#"
model User {
  id        String   @id @default(cuid())
  email     String   @unique
  posts     Post[]
  createdAt DateTime @default(now()) @map("created_at")
  @@map("users")
}

model Post {
  id       Int    @id @default(autoincrement())
  title    String
  author   User   @relation(fields: [authorId], references: [id])
  authorId String @map("author_id")
  @@index([authorId])
}
"#);
        let u = s.table("users").unwrap();
        assert!(u.columns.iter().any(|c| c.name == "created_at"));
        assert!(!u.columns.iter().any(|c| c.name == "posts"));
        assert!(u.aliases.contains(&"user".to_string()));
        let p = s.table("Post").unwrap();
        assert_eq!(p.foreign_keys[0].ref_table, "users");
        assert_eq!(p.foreign_keys[0].columns, vec!["authorId"]);
    }

    #[test]
    fn drizzle_sqlalchemy_diesel() {
        let mut s = DbSchema::default();
        parse_drizzle(&mut s, "src/db/schema.ts", r#"
import { pgTable, serial, text, integer } from "drizzle-orm/pg-core";
export const users = pgTable("users", {
  id: serial("id").primaryKey(),
  name: text("name").notNull(),
});
export const posts = pgTable("posts", {
  id: serial("id").primaryKey(),
  authorId: integer("author_id").references(() => users.id),
});
"#);
        let p = s.table("posts").unwrap();
        assert_eq!(p.foreign_keys[0].ref_table, "users");
        assert_eq!(p.foreign_keys[0].columns, vec!["author_id"]);
        parse_sqlalchemy(&mut s, "app/models.py", r#"
class Order(Base):
    __tablename__ = "orders"
    id = Column(Integer, primary_key=True)
    user_id = Column(
        Integer, ForeignKey("users.id"), nullable=False
    )
    total: Mapped[int] = mapped_column()
"#);
        let o = s.table("orders").unwrap();
        assert_eq!(o.foreign_keys[0].ref_table, "users");
        assert!(o.columns.iter().any(|c| c.name == "total" && c.data_type == "int"));
        parse_diesel(&mut s, "src/schema.rs", "diesel::table! {\n    /// Representation of the `comments` table.\n    ///\n    /// (Automatically generated by Diesel.)\n    comments (id) {\n        id -> Int4,\n        post_id -> Int4,\n        body -> Nullable<Text>,\n    }\n}\ndiesel::joinable!(comments -> posts (post_id));\n");
        let c = s.table("comments").unwrap();
        assert_eq!(c.foreign_keys[0].ref_table, "posts");
        assert!(c.columns.iter().any(|x| x.name == "body" && x.nullable));
    }

    #[test]
    fn django() {
        let mut s = DbSchema::default();
        parse_django(&mut s, "blog/models.py", r#"
from django.db import models


class TimestampedModel(models.Model):
    created_at = models.DateTimeField(auto_now_add=True)

    class Meta:
        abstract = True


class Author(TimestampedModel):
    name = models.CharField(max_length=100)
    email = models.EmailField(unique=True)
    bio = models.TextField(null=True, db_column="biography")
    rating = models.IntegerField(default=0)

    class Meta:
        db_table = "writers"
        indexes = [models.Index(fields=["name"], name="author_name_idx")]
        index_together = [["name", "rating"]]


class Post(TimestampedModel):
    title = models.CharField(max_length=200)
    author = models.ForeignKey(Author, on_delete=models.CASCADE, related_name="posts")
    parent = models.ForeignKey("self", null=True, on_delete=models.SET_NULL)
    tags = models.ManyToManyField("Tag")
    slug = models.SlugField(unique=True)

    class Meta:
        unique_together = ("title", "slug")


class Tag(models.Model):
    label = models.CharField(max_length=40)


class FeaturedPost(Post):
    class Meta:
        proxy = True
"#);
        // Meta.db_table, the implicit id, and a column from the abstract base.
        let a = s.table("writers").unwrap();
        assert!(a.columns.iter().any(|c| c.name == "created_at" && !c.nullable), "{:?}", a.columns);
        assert!(a.columns.iter().any(|c| c.name == "id" && c.primary_key), "{:?}", a.columns);
        assert!(a.columns.iter().any(|c| c.name == "biography" && c.nullable));
        assert!(a.columns.iter().any(|c| c.name == "email"));
        assert!(a.indexes.iter().any(|i| i.unique && i.columns == vec!["email"]));
        assert!(a.indexes.iter().any(|i| !i.unique && i.columns == vec!["name", "rating"]));
        assert!(a.aliases.contains(&"Author".to_string()));
        // Foreign keys: a class reference, a self reference, and M2M.
        let p = s.table("blog_post").unwrap();
        assert!(p.columns.iter().any(|c| c.name == "author_id" && !c.nullable));
        assert_eq!(p.foreign_keys[0].ref_table, "writers");
        assert_eq!(p.foreign_keys[0].columns, vec!["author_id"]);
        assert_eq!(p.foreign_keys[0].ref_columns, vec!["id"]);
        assert!(p.foreign_keys.iter().any(|f| f.ref_table == "blog_post" && f.columns == vec!["parent_id"]));
        assert!(p.indexes.iter().any(|i| i.unique && i.columns == vec!["title", "slug"]));
        // M2M: the implicit join table, its keys and its unique pair.
        let j = s.table("blog_post_tags").unwrap();
        assert_eq!(j.columns.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), vec!["id", "post_id", "tag_id"]);
        assert_eq!(j.foreign_keys[0].ref_table, "blog_post");
        assert_eq!(j.foreign_keys[1].ref_table, "blog_tag");
        assert!(j.indexes.iter().any(|i| i.unique && i.columns == vec!["post_id", "tag_id"]));
        // A proxy model is not a table.
        assert!(s.table("blog_featuredpost").is_none());
        assert!(s.table("Tag").is_some());
    }

    #[test]
    fn django_meta_inheritance_and_mti() {
        let mut s = DbSchema::default();
        parse_django(&mut s, "blog/models.py", r#"
from django.contrib.auth.models import AbstractUser
from django.contrib.postgres.indexes import GinIndex
from django.db import models


class Timestamped(models.Model):
    created_at = models.DateTimeField(auto_now_add=True)

    class Meta:
        abstract = True
        indexes = [models.Index(fields=["created_at"], name="%(class)s_created_idx")]


class Tagged(models.Model):
    tag = models.CharField(max_length=20)

    class Meta:
        abstract = True
        index_together = [["tag", "slug"]]


class Tag(models.Model):
    name = models.CharField(max_length=40)


class Author(models.Model):
    name = models.CharField(max_length=80, unique=True, db_column="author_name")


class User(AbstractUser):
    uuid = models.UUIDField(primary_key=True)

    class Meta:
        db_table = "users"


class Editor(Timestamped):
    name = models.CharField(max_length=80)


class Post(Timestamped):
    title = models.CharField(max_length=200)
    body = models.TextField(default="")
    editor = models.ForeignKey(Author, on_delete=models.CASCADE, to_field="name")
    tags = models.ManyToManyField(Tag)

    class Meta:
        db_table = "articles"
        indexes = [GinIndex(fields=["body"], name="articles_body_gin")]


class BlogPost(Post):
    subtitle = models.CharField(max_length=200, blank=True)

    class Meta:
        db_table = "blogposts"


class Story(Tagged):
    slug = models.SlugField()

    class Meta(Tagged.Meta):
        db_table = "stories"


class Searchable(models.Model):
    search_document = models.TextField(default="")

    class Meta:
        abstract = True
        indexes = [models.Index(fields=["search_document"], name="%(class)s_search_idx")]


class Article(Searchable):
    title = models.CharField(max_length=200)

    class Meta:
        db_table = "articles_search"
        indexes = [*Searchable.Meta.indexes, GinIndex(fields=["title"], name="article_title_gin")]


class Notice(Timestamped):
    pinned = models.BooleanField(default=False)

    class Meta(Timestamped.Meta):
        db_table = "notices"
        constraints = [models.UniqueConstraint(fields=["pinned"], name="unique_pinned_notice")]


class Memo(Searchable):
    text = models.TextField()

    class Meta:
        db_table = "memos"
        indexes = []
        indexes.extend(Searchable.Meta.indexes)
"#);
        // No Meta of its own: the abstract base's Meta applies.
        let e = s.table("blog_editor").unwrap();
        assert!(e.indexes.iter().any(|i| i.columns == vec!["created_at"]), "{:?}", e.indexes);
        assert!(s.table("blog_timestamped").is_none() && s.table("blog_tagged").is_none());
        // Its own Meta replaces the abstract base's, and backend index classes
        // (`GinIndex`) count.
        let p = s.table("articles").unwrap();
        assert!(p.indexes.iter().any(|i| i.columns == vec!["body"]), "{:?}", p.indexes);
        assert!(!p.indexes.iter().any(|i| i.columns == vec!["created_at"]), "{:?}", p.indexes);
        // `to_field` points at the target field's column.
        assert_eq!(p.foreign_keys[0].ref_table, "blog_author");
        assert_eq!(p.foreign_keys[0].ref_columns, vec!["author_name"]);
        // `class Meta(Base.Meta)` extends the base's Meta.
        let st = s.table("stories").unwrap();
        assert!(st.indexes.iter().any(|i| !i.unique && i.columns == vec!["tag", "slug"]), "{:?}", st.indexes);
        // `indexes = [*Base.Meta.indexes, …]` splices the base's list in, with
        // `%(class)s` filled in per model.
        let a = s.table("articles_search").unwrap();
        assert!(a.indexes.iter().any(|i| i.name == "article_search_idx" && i.columns == vec!["search_document"]), "{:?}", a.indexes);
        assert!(a.indexes.iter().any(|i| i.name == "article_title_gin"), "{:?}", a.indexes);
        // `indexes.extend(Base.Meta.indexes)` does the same after the fact.
        let n = s.table("memos").unwrap();
        assert!(n.indexes.iter().any(|i| i.name == "memo_search_idx"), "{:?}", n.indexes);
        // `constraints` does not shadow the inherited `indexes`.
        let no = s.table("notices").unwrap();
        assert!(no.indexes.iter().any(|i| i.columns == vec!["created_at"]), "{:?}", no.indexes);
        assert!(no.indexes.iter().any(|i| i.unique && i.columns == vec!["pinned"]), "{:?}", no.indexes);
        // Multi-table inheritance: the child links to the parent and holds only
        // its own columns (the parent's table keeps the inherited ones).
        let b = s.table("blogposts").unwrap();
        assert_eq!(b.columns.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), vec!["post_ptr_id", "subtitle"]);
        assert_eq!(b.columns[0].primary_key, true);
        assert_eq!(b.foreign_keys[0].ref_table, "articles");
        assert_eq!(b.foreign_keys[0].ref_columns, vec!["id"]);
        // A base from a library (`AbstractUser`) still makes a model.
        assert!(s.table("users").unwrap().columns.iter().any(|c| c.name == "uuid" && c.primary_key));
        // The parent's M2M join table is named after its `db_table`.
        let j = s.table("articles_tags").unwrap();
        assert_eq!(j.columns.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), vec!["id", "post_id", "tag_id"]);
    }

    #[test]
    fn django_project() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let write = |path: &str, body: &str| {
            let p = root.join(path);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, body).unwrap();
        };
        write("project/settings.py", "INSTALLED_APPS = [\"blog\", \"accounts\"]\nAUTH_USER_MODEL = \"accounts.User\"\n");
        write("accounts/models.py", r#"
from django.db import models


class User(models.Model):
    uuid = models.UUIDField(primary_key=True)
    handle = models.CharField(max_length=30, unique=True)

    class Meta:
        db_table = "users"
"#);
        write("blog/models.py", r#"
from django.conf import settings
from django.db import models

from shared.models import TimestampedModel


class Post(TimestampedModel):
    title = models.CharField(max_length=200)
    author = models.ForeignKey(settings.AUTH_USER_MODEL, on_delete=models.CASCADE)
    tags = models.ManyToManyField("Tag")
    reviewers = models.ManyToManyField(settings.AUTH_USER_MODEL)

    class Meta:
        db_table = "articles"


class Tag(TimestampedModel):
    label = models.CharField(max_length=40)


class Rating(TimestampedModel):
    post = models.ForeignKey("Post", on_delete=models.CASCADE)
    user = models.ForeignKey("accounts.User", on_delete=models.CASCADE)
    score = models.IntegerField()


class Review(TimestampedModel):
    post = models.ForeignKey("Post", on_delete=models.CASCADE)
    raters = models.ManyToManyField("accounts.User", through=Rating)
"#);
        write("shared/models.py", r#"
from django.db import models


class TimestampedModel(models.Model):
    created_at = models.DateTimeField(auto_now_add=True)

    class Meta:
        abstract = True
"#);
        // Another app queries the model, so the table links back to the code.
        write("blog/views.py", r#"
from .models import Post


def index(request):
    return Post.objects.filter(author=request.user)
"#);
        let index = Index::build(root, &crate::BuildOptions { use_cache: false, ..Default::default() }).unwrap();
        let mut s = from_repo(&index);
        link_code(&index, &mut s);
        // App label from the directory; Meta.db_table wins over it.
        assert!(s.table("articles").is_some(), "{:?}", s.tables.iter().map(|t| t.key()).collect::<Vec<_>>());
        assert!(s.table("blog_tag").is_some());
        assert!(s.table("blog_rating").is_some());
        // The abstract base in another app contributes its column.
        let p = s.table("articles").unwrap();
        assert!(p.columns.iter().any(|c| c.name == "created_at"), "{:?}", p.columns);
        // `settings.AUTH_USER_MODEL` resolves through the settings file.
        assert_eq!(p.foreign_keys[0].ref_table, "users");
        assert_eq!(p.foreign_keys[0].ref_columns, vec!["uuid"], "the target's primary key");
        // M2M to another app: join tables per target, columns named after the models.
        let j = s.table("articles_tags").unwrap();
        assert_eq!(j.columns.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), vec!["id", "post_id", "tag_id"]);
        assert_eq!(j.foreign_keys[1].ref_table, "blog_tag");
        let r = s.table("articles_reviewers").unwrap();
        assert_eq!(r.columns.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), vec!["id", "post_id", "user_id"]);
        assert_eq!(r.foreign_keys[1].ref_table, "users");
        // `through=` names a model, so no join table is invented for it.
        assert!(s.table("articles_raters").is_none());
        assert!(s.table("blog_rating").unwrap().foreign_keys.iter().any(|f| f.ref_table == "articles"));
        // Classification stays with Django (`updated_at` is a column, `save` is not).
        assert!(!p.columns.iter().any(|c| c.name == "save"));
        // Code links: `Post.objects.filter(...)` in a file that imports the models.
        assert!(p.used_by.iter().any(|u| u.file == "blog/views.py"), "{:?}", p.used_by);
        assert!(s.sources.iter().any(|s| s == "blog/models.py"));
    }
}

