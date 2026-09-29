//! Search ranking knobs. `REPOMAP_TUNE="name=value,..."` overrides them for
//! experiments; the defaults are the shipped values.

use std::sync::OnceLock;

#[derive(Debug, Clone, Copy)]
pub struct Tune {
    /// Candidates per ranked list.
    pub cand: usize,
    /// Reciprocal-rank-fusion constant.
    pub k: f32,
    /// Weight of the whole-query embedding list.
    pub wd: f32,
    /// Weight of the title embedding list (0 = off).
    pub wh: f32,
    /// Weight of the identifiers-and-code-spans embedding list (0 = off).
    pub wc: f32,
    /// Weight of the exact-identifier symbol list (0 = off).
    pub wi: f32,
    /// BM25 weight of a query term that comes from an extracted identifier.
    pub wt: f32,
    /// Weight of the file-level BM25 list (0 = off).
    pub wf: f32,
    /// BM25 k1 at file level.
    pub k1f: f32,
    /// Weight of a file's further chunks (decayed sum); the previous rule when `agg` is 0.
    pub lam: f32,
    /// 1 = max plus decayed sum, 0 = previous coherence boost.
    pub agg: f32,
    /// Decay of a file's further result chunks.
    pub dec: f32,
    /// Boost for query words in a file's name or folder.
    pub pw: f32,
    /// Boost for a file path or module the query names.
    pub mw: f32,
    /// Keep this many query terms with the largest weight times idf (0 = all).
    pub qcap: usize,
    /// An identifier naming more symbols than this is too common to count.
    pub symcap: usize,
}

impl Default for Tune {
    fn default() -> Self {
        Tune { cand: 200, k: 20.0, wd: 1.0, wh: 0.0, wc: 0.0, wi: 1.0, wt: 2.0, wf: 0.5, k1f: 1.5, lam: 0.5, agg: 1.0, dec: 0.4, pw: 1.5, mw: 1.0, qcap: 0, symcap: 4 }
    }
}

pub fn get() -> &'static Tune {
    static T: OnceLock<Tune> = OnceLock::new();
    T.get_or_init(|| {
        let mut t = Tune::default();
        if let Ok(s) = std::env::var("REPOMAP_TUNE") {
            for kv in s.split(',') {
                let Some((k, v)) = kv.split_once('=') else { continue };
                let Ok(v) = v.trim().parse::<f32>() else { continue };
                match k.trim() {
                    "cand" => t.cand = v as usize,
                    "k" => t.k = v,
                    "wd" => t.wd = v,
                    "wh" => t.wh = v,
                    "wc" => t.wc = v,
                    "wi" => t.wi = v,
                    "wt" => t.wt = v,
                    "wf" => t.wf = v,
                    "k1f" => t.k1f = v,
                    "lam" => t.lam = v,
                    "agg" => t.agg = v,
                    "dec" => t.dec = v,
                    "pw" => t.pw = v,
                    "mw" => t.mw = v,
                    "qcap" => t.qcap = v as usize,
                    "symcap" => t.symcap = v as usize,
                    _ => {}
                }
            }
        }
        t
    })
}
