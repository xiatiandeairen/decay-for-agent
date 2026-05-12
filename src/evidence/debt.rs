use std::collections::BTreeMap;
use std::path::Path;

use crate::evidence::history::detect_history;
use crate::file_role;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebtInput {
    pub file: String,
    pub function: String,
    pub complexity_overage: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DebtReport {
    pub records: Vec<DebtRecord>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebtRecord {
    pub kind: &'static str,
    pub file: String,
    pub subject: String,
    pub source: &'static str,
    pub score: u32,
    pub tags: Vec<String>,
}

pub fn detect_hot_debt(project_root: &Path, inputs: &[DebtInput]) -> DebtReport {
    let mut by_file: BTreeMap<String, FileDebt> = BTreeMap::new();
    for input in inputs {
        if !file_role::classify(&input.file).is_prod() {
            continue;
        }
        let entry = by_file.entry(input.file.clone()).or_default();
        entry.functions += 1;
        entry.overage = entry.overage.saturating_add(input.complexity_overage);
        if entry.first_function.is_empty() {
            entry.first_function = input.function.clone();
        }
    }

    let files = by_file.keys().map(String::as_str).collect::<Vec<_>>();
    let history = detect_history(project_root, &files);
    let mut report = DebtReport {
        records: Vec::new(),
        diagnostics: history.diagnostics,
    };

    for (file, debt) in by_file {
        let mut tags = Vec::new();
        let mut score = debt
            .overage
            .saturating_add((debt.functions as u32).saturating_mul(10));

        for tag in boundary_tags(&file) {
            score = score.saturating_add(20);
            tags.push(tag);
        }

        for evidence in history.records.iter().filter(|record| record.file == file) {
            match evidence.kind.as_str() {
                "churn_hotspot" => {
                    score = score.saturating_add(40);
                    tags.push("history:churn_hotspot".to_string());
                }
                "recently_bugfixed_area" => {
                    score = score.saturating_add(30);
                    tags.push("history:recently_bugfixed_area".to_string());
                }
                "frequent_rework" => {
                    score = score.saturating_add(25);
                    tags.push("history:frequent_rework".to_string());
                }
                _ => {}
            }
        }

        dedup(&mut tags);
        report.records.push(DebtRecord {
            kind: "hot_complexity_debt",
            file,
            subject: debt.first_function,
            source: "doctor+git_history",
            score,
            tags,
        });
    }

    report.records.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then(a.file.cmp(&b.file))
            .then(a.subject.cmp(&b.subject))
    });
    report
}

#[derive(Debug, Default)]
struct FileDebt {
    functions: usize,
    overage: u32,
    first_function: String,
}

fn boundary_tags(path: &str) -> Vec<String> {
    let lower = path.to_ascii_lowercase();
    [
        ("ffi", "boundary:ffi"),
        ("storage", "boundary:storage"),
        ("migration", "boundary:migration"),
        ("protocol", "boundary:protocol"),
        ("serde", "boundary:serialization"),
        ("config", "boundary:config"),
        ("unsafe", "boundary:unsafe"),
    ]
    .iter()
    .filter(|(needle, _)| lower.contains(needle))
    .map(|(_, tag)| (*tag).to_string())
    .collect()
}

fn dedup(items: &mut Vec<String>) {
    let mut seen = BTreeMap::new();
    items.retain(|item| seen.insert(item.clone(), ()).is_none());
}
