//! BM25 over code chunks (the Locus engine, now AST-chunked).

use crate::index::FileEntry;
use crate::parse::FileFacts;
use crate::tokenize::tokenize;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ChunkRef {
    pub file: u32,
    pub start: u32,
    pub end: u32,
    /// Global symbol index when the chunk is a definition.
    pub symbol: Option<u32>,
    pub len: u32,
}

#[derive(Default)]
pub struct Bm25 {
    pub chunks: Vec<ChunkRef>,
    postings: HashMap<String, Vec<(u32, u16)>>,
    avg_len: f32,
    /// Per file: the range of `chunks` it owns and its total length.
    file_span: Vec<(u32, u32)>,
    file_len: Vec<u32>,
    avg_file_len: f32,
    files_with_chunks: u32,
}

pub struct Hit {
    pub chunk: u32,
    pub score: f32,
    pub matched: Vec<String>,
}

const K1: f32 = 1.2;
const B: f32 = 0.75;

impl Bm25 {
    pub fn build<'a>(files: impl Iterator<Item = (u32, &'a FileEntry, &'a FileFacts)>) -> Bm25 {
        let mut chunks = Vec::new();
        let mut postings: HashMap<String, Vec<(u32, u16)>> = HashMap::new();
        let mut total: u64 = 0;
        let mut file_span: Vec<(u32, u32)> = Vec::new();
        let mut file_len: Vec<u32> = Vec::new();
        for (fi, entry, facts) in files {
            if file_span.len() <= fi as usize {
                file_span.resize(fi as usize + 1, (0, 0));
                file_len.resize(fi as usize + 1, 0);
            }
            file_span[fi as usize] = (chunks.len() as u32, chunks.len() as u32 + facts.chunks.len() as u32);
            for c in &facts.chunks {
                file_len[fi as usize] += c.len;
                let id = chunks.len() as u32;
                chunks.push(ChunkRef {
                    file: fi,
                    start: c.start,
                    end: c.end,
                    symbol: c.symbol.map(|s| s + entry.sym_start),
                    len: c.len,
                });
                total += c.len as u64;
                for (t, f) in c.terms.split(' ').zip(c.tfs.iter()) {
                    match postings.get_mut(t) {
                        Some(v) => v.push((id, *f)),
                        None => {
                            postings.insert(t.to_string(), vec![(id, *f)]);
                        }
                    }
                }
            }
        }
        let avg_len = if chunks.is_empty() { 1.0 } else { total as f32 / chunks.len() as f32 };
        let files_with_chunks = file_len.iter().filter(|l| **l > 0).count() as u32;
        let avg_file_len = if files_with_chunks == 0 { 1.0 } else { file_len.iter().map(|l| *l as u64).sum::<u64>() as f32 / files_with_chunks as f32 };
        Bm25 { chunks, postings, avg_len, file_span, file_len, avg_file_len, files_with_chunks }
    }

    /// The chunks of one file.
    pub fn chunks_of(&self, file: u32) -> &[ChunkRef] {
        match self.file_span.get(file as usize) {
            Some(&(a, b)) => &self.chunks[a as usize..b as usize],
            None => &[],
        }
    }

    /// Keep the `cap` terms that weigh most (weight times idf).
    pub fn cap_terms(&self, mut terms: Vec<(String, f32)>, cap: usize) -> Vec<(String, f32)> {
        if cap == 0 || terms.len() <= cap {
            return terms;
        }
        let n = self.chunks.len() as f32;
        let weight = |t: &(String, f32)| -> f32 {
            let df = self.postings.get(&t.0).map_or(0.0, |l| l.len() as f32);
            if df == 0.0 { 0.0 } else { t.1 * ((n - df + 0.5) / (df + 0.5) + 1.0).ln() }
        };
        terms.sort_by(|a, b| weight(b).total_cmp(&weight(a)).then(a.0.cmp(&b.0)));
        terms.truncate(cap);
        terms
    }

    /// Files that contain `term` (lowercased token) anywhere in a chunk.
    pub fn files_with(&self, term: &str) -> std::collections::HashSet<u32> {
        self.postings
            .get(term)
            .map(|v| v.iter().map(|(c, _)| self.chunks[*c as usize].file).collect())
            .unwrap_or_default()
    }

    pub fn terms(&self) -> usize {
        self.postings.len()
    }

    pub fn search(&self, query: &str, limit: usize, filter: impl Fn(&ChunkRef) -> bool) -> Vec<Hit> {
        let mut qterms = tokenize(query);
        qterms.sort();
        qterms.dedup();
        let terms: Vec<(String, f32)> = qterms.into_iter().map(|t| (t, 1.0)).collect();
        self.search_terms(&terms, limit, 0, 1.2, filter).0
    }

    /// Chunks and, from the same postings, whole files (their chunks' term
    /// counts summed) for weighted query terms. `file_limit` 0 skips the files.
    pub fn search_terms(&self, terms: &[(String, f32)], limit: usize, file_limit: usize, k1_file: f32, filter: impl Fn(&ChunkRef) -> bool) -> (Vec<Hit>, Vec<(u32, f32)>) {
        let n = self.chunks.len() as f32;
        let nf = self.files_with_chunks as f32;
        let mut scores: HashMap<u32, (f32, Vec<String>)> = HashMap::new();
        let mut fscores: HashMap<u32, f32> = HashMap::new();
        for (t, w) in terms {
            let Some(list) = self.postings.get(t) else { continue };
            let df = list.len() as f32;
            let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
            let mut per_file: Vec<(u32, u32)> = Vec::new();
            for &(cid, tf) in list {
                let c = &self.chunks[cid as usize];
                if !filter(c) {
                    continue;
                }
                let tf = tf as f32;
                let norm = tf * (K1 + 1.0) / (tf + K1 * (1.0 - B + B * c.len as f32 / self.avg_len));
                let e = scores.entry(cid).or_insert((0.0, Vec::new()));
                e.0 += w * idf * norm;
                e.1.push(t.clone());
                if file_limit > 0 {
                    // Chunks of a file are consecutive in a posting list.
                    match per_file.last_mut() {
                        Some(l) if l.0 == c.file => l.1 += tf as u32,
                        _ => per_file.push((c.file, tf as u32)),
                    }
                }
            }
            if file_limit > 0 {
                let dff = per_file.len() as f32;
                let idf_f = ((nf - dff + 0.5) / (dff + 0.5) + 1.0).ln();
                for (f, tf) in per_file {
                    let tf = tf as f32;
                    let len = self.file_len.get(f as usize).copied().unwrap_or(0) as f32;
                    let norm = tf * (k1_file + 1.0) / (tf + k1_file * (1.0 - B + B * len / self.avg_file_len));
                    *fscores.entry(f).or_insert(0.0) += w * idf_f * norm;
                }
            }
        }
        let mut hits: Vec<Hit> = scores
            .into_iter()
            .map(|(chunk, (score, matched))| {
                // Reward chunks that match more distinct query terms.
                let coverage = matched.len() as f32 / terms.len().max(1) as f32;
                Hit { chunk, score: score * (0.5 + coverage), matched }
            })
            .collect();
        hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal).then(a.chunk.cmp(&b.chunk)));
        hits.truncate(limit);
        let mut files: Vec<(u32, f32)> = fscores.into_iter().collect();
        files.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        files.truncate(file_limit);
        (hits, files)
    }
}
