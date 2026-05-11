use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;
use std::process::Command;

use crate::error::{DecayError, Result};
use crate::git_diff::{parse_git_diff, GitDiff, HunkLineKind};
use crate::metric;
use crate::pipeline;
use crate::types::{Function, Metrics};

#[derive(Debug, Clone)]
pub struct ImpactReport {
    pub changed_functions: usize,
    pub evidences: Vec<ImpactEvidence>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImpactEvidence {
    pub kind: EvidenceKind,
    pub function: String,
    pub file: String,
    pub source: &'static str,
    pub previous: Option<Metrics>,
    pub current: Option<Metrics>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceKind {
    Worsened,
    Improved,
    NewHighRisk,
    TouchedExistingRisky,
    IdentityUncertain,
}

impl EvidenceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EvidenceKind::Worsened => "worsened",
            EvidenceKind::Improved => "improved",
            EvidenceKind::NewHighRisk => "new_high_risk",
            EvidenceKind::TouchedExistingRisky => "touched_existing_risky",
            EvidenceKind::IdentityUncertain => "identity_uncertain",
        }
    }
}

pub fn analyze_worktree_diff(project_root: &Path, base_ref: &str) -> Result<ImpactReport> {
    let diff = worktree_diff(project_root, base_ref)?;
    let parsed = parse_git_diff(&diff);
    analyze_parsed_diff(project_root, base_ref, &parsed)
}

fn analyze_parsed_diff(
    project_root: &Path,
    base_ref: &str,
    diff: &GitDiff,
) -> Result<ImpactReport> {
    let mut evidences = Vec::new();
    let mut diagnostics = Vec::new();
    let mut changed_keys = BTreeSet::new();

    for file in &diff.files {
        let before_path = file.old_path.as_deref();
        let after_path = file.new_path.as_deref();
        if !is_rust_change(before_path, after_path) {
            continue;
        }
        let before_funcs = match before_path {
            Some(path) => match git_show(project_root, base_ref, path) {
                Ok(Some(source)) => scan_side(path, &source, &mut diagnostics),
                Ok(None) => Vec::new(),
                Err(err) => {
                    diagnostics.push(err.to_string());
                    Vec::new()
                }
            },
            None => Vec::new(),
        };
        let after_funcs = match after_path {
            Some(path) => {
                let path_abs = project_root.join(path);
                match std::fs::read_to_string(&path_abs) {
                    Ok(source) => scan_side(path, &source, &mut diagnostics),
                    Err(source) => {
                        diagnostics.push(format!(
                            "failed to read {}: {}",
                            path_abs.display(),
                            source
                        ));
                        Vec::new()
                    }
                }
            }
            None => Vec::new(),
        };

        let changed_before = changed_lines(file, false);
        let changed_after = changed_lines(file, true);
        let touched_before = touched_functions(&before_funcs, &changed_before);
        let touched_after = touched_functions(&after_funcs, &changed_after);
        for f in touched_before.iter().chain(touched_after.iter()) {
            changed_keys.insert(format!("{}::{}", f.file, f.name));
        }

        let before_by_stable = unique_by_stable(&before_funcs);
        let after_by_stable = unique_by_stable(&after_funcs);
        let before_touched_stable: HashSet<u64> = touched_before
            .iter()
            .filter(|f| f.stable_signature_hash != 0)
            .map(|f| f.stable_signature_hash)
            .collect();

        for after in &touched_after {
            let previous = before_by_stable
                .get(&after.stable_signature_hash)
                .copied()
                .filter(|_| after.stable_signature_hash != 0);
            match previous {
                Some(before) => {
                    if let Some(kind) = classify_metrics(before.metrics, after.metrics) {
                        evidences.push(ImpactEvidence {
                            kind,
                            function: after.name.clone(),
                            file: after.file.clone(),
                            source: "diff_hunk+ast_snapshot",
                            previous: Some(before.metrics),
                            current: Some(after.metrics),
                        });
                    }
                }
                None => {
                    if exceeds_any_threshold(after.metrics) {
                        evidences.push(ImpactEvidence {
                            kind: EvidenceKind::NewHighRisk,
                            function: after.name.clone(),
                            file: after.file.clone(),
                            source: "diff_hunk+ast_snapshot",
                            previous: None,
                            current: Some(after.metrics),
                        });
                    }
                }
            }
        }

        for before in &touched_before {
            if before.stable_signature_hash != 0
                && after_by_stable.contains_key(&before.stable_signature_hash)
            {
                continue;
            }
            if before.stable_signature_hash != 0
                && before_touched_stable.contains(&before.stable_signature_hash)
            {
                continue;
            }
            if exceeds_any_threshold(before.metrics) {
                evidences.push(ImpactEvidence {
                    kind: EvidenceKind::TouchedExistingRisky,
                    function: before.name.clone(),
                    file: before.file.clone(),
                    source: "diff_hunk+ast_snapshot",
                    previous: Some(before.metrics),
                    current: None,
                });
            }
        }
    }

    evidences.sort_by(|a, b| a.file.cmp(&b.file).then(a.function.cmp(&b.function)));

    Ok(ImpactReport {
        changed_functions: changed_keys.len(),
        evidences,
        diagnostics,
    })
}

fn is_rust_change(before_path: Option<&str>, after_path: Option<&str>) -> bool {
    before_path
        .or(after_path)
        .is_some_and(|path| path.ends_with(".rs"))
        || after_path.is_some_and(|path| path.ends_with(".rs"))
}

fn worktree_diff(project_root: &Path, base_ref: &str) -> Result<String> {
    let numstat = git_output(project_root, &["diff", "--numstat", base_ref, "--"])?;
    let patch = git_output(
        project_root,
        &["diff", "--find-renames", "--unified=0", base_ref, "--"],
    )?;
    Ok(format!("{numstat}\n{patch}"))
}

fn git_output(project_root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(project_root)
        .output()
        .map_err(|source| DecayError::Io {
            path: "git".to_string(),
            source,
        })?;
    if !output.status.success() {
        return Err(DecayError::InvalidProject(format!(
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn git_show(project_root: &Path, base_ref: &str, path: &str) -> Result<Option<String>> {
    let output = Command::new("git")
        .args(["show", &format!("{base_ref}:{path}")])
        .current_dir(project_root)
        .output()
        .map_err(|source| DecayError::Io {
            path: "git".to_string(),
            source,
        })?;
    if output.status.success() {
        Ok(Some(String::from_utf8_lossy(&output.stdout).into_owned()))
    } else {
        Ok(None)
    }
}

fn scan_side(path: &str, source: &str, diagnostics: &mut Vec<String>) -> Vec<Function> {
    match pipeline::scan_source(path, source) {
        Ok(funcs) => funcs,
        Err(err) => {
            diagnostics.push(err.to_string());
            Vec::new()
        }
    }
}

fn changed_lines(file: &crate::git_diff::GitFileDiff, after: bool) -> BTreeSet<u32> {
    let mut lines = BTreeSet::new();
    for hunk in &file.hunks {
        let mut old_line = hunk.old_start;
        let mut new_line = hunk.new_start;
        for line in &hunk.lines {
            match line.kind {
                HunkLineKind::Context => {
                    old_line += 1;
                    new_line += 1;
                }
                HunkLineKind::Addition => {
                    if after {
                        lines.insert(new_line);
                    }
                    new_line += 1;
                }
                HunkLineKind::Deletion => {
                    if !after {
                        lines.insert(old_line);
                    }
                    old_line += 1;
                }
            }
        }
    }
    lines
}

fn touched_functions<'a>(functions: &'a [Function], lines: &BTreeSet<u32>) -> Vec<&'a Function> {
    functions
        .iter()
        .filter(|f| {
            lines
                .iter()
                .any(|line| *line >= f.start_line && *line <= f.end_line)
        })
        .collect()
}

fn unique_by_stable(functions: &[Function]) -> HashMap<u64, &Function> {
    let mut counts: HashMap<u64, usize> = HashMap::new();
    for f in functions {
        if f.stable_signature_hash != 0 {
            *counts.entry(f.stable_signature_hash).or_insert(0) += 1;
        }
    }
    functions
        .iter()
        .filter(|f| f.stable_signature_hash != 0)
        .filter(|f| counts.get(&f.stable_signature_hash) == Some(&1))
        .map(|f| (f.stable_signature_hash, f))
        .collect()
}

fn classify_metrics(previous: Metrics, current: Metrics) -> Option<EvidenceKind> {
    let mut worsened = false;
    let mut improved = false;
    let mut touched_risky = exceeds_any_threshold(previous) || exceeds_any_threshold(current);

    for def in metric::ACTIVE_METRICS {
        let p = previous.value(def.id);
        let c = current.value(def.id);
        if c > p {
            worsened = true;
        } else if c < p {
            improved = true;
        }
        if p > def.threshold || c > def.threshold {
            touched_risky = true;
        }
    }

    if worsened {
        Some(EvidenceKind::Worsened)
    } else if improved {
        Some(EvidenceKind::Improved)
    } else if touched_risky {
        Some(EvidenceKind::TouchedExistingRisky)
    } else {
        None
    }
}

fn exceeds_any_threshold(metrics: Metrics) -> bool {
    metric::active_values(metrics).any(|(def, value)| value > def.threshold)
}
