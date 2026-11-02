//! Corpus harness for numbers, ordinals and dates (epic #343, phase E).
//!
//! Every `tests/corpus/*.toml` file is one category; the file stem is the
//! category name (`dates`, `ordinals`, …). Each case carries a sentence and the
//! exact normalized text a Russian speaker should hear, so the corpus measures
//! the pipeline end to end rather than the number helpers in isolation.
//!
//! Two statuses, documented in `tests/corpus/README.md`:
//!
//! - `locked` — the pipeline already reads the case correctly; the
//!   [`corpus_locked_matches_current_behavior`] test fails on drift.
//! - `target` — the desired reading that phases B and C implement; the
//!   [`corpus_target_rate`] test is ignored so the known gap does not turn CI
//!   red before the epic lands. Run it with `--ignored --nocapture` for the
//!   per-category pass rate.
//!
//! [`corpus_runtime_within_budget`] pins the runtime side of the yardstick: a
//! sample assembled from the corpus (256 KB by default, overridable through
//! `RUVOX_CORPUS_SAMPLE_BYTES` for branch comparison) must stay inside a
//! debug-build budget the pipeline performance lock also uses.

use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tts_normalize::pipeline::TTSPipeline;
use tts_normalize::profiles::SILERO_NATIVE_PROFILE;

/// One corpus file per category; the file stem is the category name.
const CORPUS_DIR: &str = "tests/corpus";
/// Default size of the runtime sample built by repeating corpus sentences.
/// The CI guard stays small: a debug build normalizes numbers-heavy prose at
/// roughly 50 KB/s locally, and GitHub runners are 4-5x slower. Override with
/// `RUVOX_CORPUS_SAMPLE_BYTES` for the branch comparison on a fixed machine.
const DEFAULT_SAMPLE_BYTES: usize = 256_000;
const SAMPLE_BYTES_ENV: &str = "RUVOX_CORPUS_SAMPLE_BYTES";
/// Same budget as `src-tauri/tests/perf.rs`: far above a healthy debug build,
/// low enough to catch a quadratic or allocation-heavy regression.
const RUNTIME_BUDGET: Duration = Duration::from_secs(60);

#[derive(Debug, Deserialize)]
struct CorpusFile {
    #[serde(default)]
    case: Vec<Case>,
}

#[derive(Debug, Deserialize)]
struct Case {
    id: String,
    subcategory: String,
    status: Status,
    input: String,
    expected: String,
    #[serde(default)]
    note: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Status {
    Locked,
    Target,
}

impl Status {
    fn label(self) -> &'static str {
        match self {
            Status::Locked => "locked",
            Status::Target => "target",
        }
    }
}

struct LoadedCase {
    category: String,
    case: Case,
}

struct Mismatch<'a> {
    case: &'a LoadedCase,
    actual: String,
}

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(CORPUS_DIR)
}

/// Read, validate and sort every corpus file. Validation is deliberately
/// strict: a duplicate id or an empty field is a corpus bug, not a test
/// failure, and must point at the offending file.
fn load_corpus() -> Vec<LoadedCase> {
    let dir = corpus_dir();
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read corpus directory {}: {e}", dir.display()))
        .map(|entry| entry.expect("corpus directory entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no corpus files under {}", dir.display());

    let mut seen_ids = BTreeSet::new();
    let mut cases = Vec::new();
    for path in paths {
        let category = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .expect("corpus file stem")
            .to_string();
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let file: CorpusFile =
            toml::from_str(&raw).unwrap_or_else(|e| panic!("cannot parse {}: {e}", path.display()));
        assert!(!file.case.is_empty(), "{} has no cases", path.display());
        for case in file.case {
            assert!(
                !case.id.trim().is_empty(),
                "{}: empty case id",
                path.display()
            );
            assert!(
                seen_ids.insert(case.id.clone()),
                "{}: duplicate case id {:?}",
                path.display(),
                case.id
            );
            assert!(
                !case.subcategory.trim().is_empty(),
                "{}: case {:?} has no subcategory",
                path.display(),
                case.id
            );
            assert!(
                !case.input.is_empty(),
                "{}: case {:?} has an empty input",
                path.display(),
                case.id
            );
            assert!(
                !case.expected.is_empty(),
                "{}: case {:?} has an empty expected",
                path.display(),
                case.id
            );
            cases.push(LoadedCase {
                category: category.clone(),
                case,
            });
        }
    }
    cases.sort_by(|a, b| {
        (a.category.as_str(), a.case.id.as_str()).cmp(&(b.category.as_str(), b.case.id.as_str()))
    });
    cases
}

/// Run one case through the pipeline under the default Russian engine profile.
fn normalize(pipeline: &mut TTSPipeline, input: &str) -> String {
    pipeline
        .process_for_engine(input, &SILERO_NATIVE_PROFILE)
        .text
}

/// Evaluate every case with `status`, returning a per-category rate table and
/// the mismatches. A fresh pipeline per case keeps the cases order-independent.
fn evaluate<'a>(cases: &'a [LoadedCase], status: Status) -> (String, Vec<Mismatch<'a>>) {
    let mut per_category: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    let mut failures = Vec::new();
    let mut pass = 0;
    let mut total = 0;

    for loaded in cases.iter().filter(|c| c.case.status == status) {
        let actual = normalize(&mut TTSPipeline::new(), &loaded.case.input);
        let entry = per_category.entry(loaded.category.as_str()).or_default();
        entry.1 += 1;
        total += 1;
        if actual == loaded.case.expected {
            entry.0 += 1;
            pass += 1;
        } else {
            failures.push(Mismatch {
                case: loaded,
                actual,
            });
        }
    }

    let mut table = String::new();
    for (category, (cat_pass, cat_total)) in per_category {
        table.push_str(&format!(
            "  {category:<22} {cat_pass:>3}/{cat_total:<3} {:>6.1}%\n",
            100.0 * cat_pass as f64 / cat_total as f64
        ));
    }
    if total > 0 {
        table.push_str(&format!(
            "  {:<22} {pass:>3}/{total:<3} {:>6.1}%\n",
            "total",
            100.0 * pass as f64 / total as f64
        ));
    }
    (table, failures)
}

fn format_mismatches(failures: &[Mismatch<'_>]) -> String {
    let mut out = String::new();
    for failure in failures {
        let case = &failure.case.case;
        out.push_str(&format!(
            "\n[{}] {}\n  input:    {}\n  expected: {}\n  actual:   {}\n",
            case.id, case.subcategory, case.input, case.expected, failure.actual
        ));
        if !case.note.is_empty() {
            out.push_str(&format!("  note:     {}\n", case.note));
        }
    }
    out
}

#[test]
fn corpus_locked_matches_current_behavior() {
    let cases = load_corpus();
    let (table, failures) = evaluate(&cases, Status::Locked);
    assert!(
        failures.is_empty(),
        "{} locked case(s) drifted from the current pipeline:\n{table}{}",
        failures.len(),
        format_mismatches(&failures)
    );
}

/// Guard the corpus itself: a shrinking or lopsided corpus must fail loudly,
/// otherwise deleting cases silently lowers the yardstick.
#[test]
fn corpus_distribution_is_balanced() {
    let cases = load_corpus();
    let mut per_category: BTreeMap<&str, BTreeMap<&str, usize>> = BTreeMap::new();
    for loaded in &cases {
        *per_category
            .entry(loaded.category.as_str())
            .or_default()
            .entry(loaded.case.subcategory.as_str())
            .or_default() += 1;
    }

    for (category, buckets) in &per_category {
        let total: usize = buckets.values().sum();
        assert!(
            total >= 25,
            "category {category} has only {total} cases; keep at least 25"
        );
        assert!(
            buckets.len() >= 3,
            "category {category} has only {} subcategory bucket(s)",
            buckets.len()
        );
        let biggest = buckets.values().max().copied().unwrap_or(0);
        assert!(
            biggest * 100 <= total * 60,
            "a subcategory in {category} holds {biggest}/{total} cases; keep one bucket under 60%"
        );
    }

    let cyrillic = cases
        .iter()
        .filter(|loaded| {
            loaded
                .case
                .input
                .chars()
                .any(|ch| ch.is_alphabetic() && !ch.is_ascii())
        })
        .count();
    assert!(
        cyrillic * 100 >= cases.len() * 60,
        "only {cyrillic}/{} inputs carry Cyrillic context; keep at least 60%",
        cases.len()
    );
}

#[test]
#[ignore = "phase B/C yardstick: expected to fail until the target corpus is implemented"]
fn corpus_target_rate() {
    let cases = load_corpus();
    let (table, failures) = evaluate(&cases, Status::Target);
    println!("\n{} corpus rate:\n{table}", Status::Target.label());
    assert!(
        failures.is_empty(),
        "{} target case(s) still differ from the natural reading:\n{table}{}",
        failures.len(),
        format_mismatches(&failures)
    );
}

#[test]
fn corpus_runtime_within_budget() {
    let cases = load_corpus();
    let sample_bytes = std::env::var(SAMPLE_BYTES_ENV)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(DEFAULT_SAMPLE_BYTES);
    let mut sample = String::with_capacity(sample_bytes + 1024);
    while sample.len() < sample_bytes {
        for loaded in &cases {
            sample.push_str(&loaded.case.input);
            sample.push('\n');
        }
    }

    let mut pipeline = TTSPipeline::new();
    let start = Instant::now();
    let processed = pipeline.process_for_engine(&sample, &SILERO_NATIVE_PROFILE);
    let elapsed = start.elapsed();
    let megabytes = sample.len() as f64 / 1_000_000.0;
    println!(
        "{} KB corpus sample normalized in {elapsed:?} ({:.2} MB/s)",
        sample.len() / 1000,
        megabytes / elapsed.as_secs_f64()
    );

    assert!(
        !processed.text.is_empty(),
        "the runtime sample produced empty output"
    );
    assert!(
        elapsed < RUNTIME_BUDGET,
        "{} KB corpus sample normalized in {elapsed:?}, budget is {RUNTIME_BUDGET:?}",
        sample.len() / 1000
    );
}
