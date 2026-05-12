use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;
use std::process::Command;

use crate::error::{DecayError, Result};
use crate::evidence::history::{analyze_history_at, HistoryFileChange, HistoryReport};
use crate::evidence::quality::{detect_quality, QualityFileChange, QualityReport};
use crate::evidence::stability::detect_stability;
use crate::evidence::EvidenceRecord;
use crate::evidence_store::{DiffEvidenceRecord, EvidenceFileChange};
use crate::file_role;
use crate::git_diff::{parse_git_diff, GitDiff, GitFileStatus, HunkLineKind};
use crate::metric;
use crate::pipeline;
use crate::types::{Function, Metrics};

#[derive(Debug, Clone)]
pub struct ImpactReport {
    pub changed_functions: usize,
    pub evidences: Vec<ImpactEvidence>,
    pub stability_evidence: Vec<EvidenceRecord>,
    pub semantic_evidence: Vec<EvidenceRecord>,
    pub quality: QualityReport,
    pub history: HistoryReport,
    pub file_changes: Vec<EvidenceFileChange>,
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
    let mut stability_evidence = Vec::new();
    let mut diagnostics = Vec::new();
    let mut changed_keys = BTreeSet::new();

    for file in &diff.files {
        let before_path = file.old_path.as_deref();
        let after_path = file.new_path.as_deref();
        if !is_rust_change(before_path, after_path) {
            continue;
        }
        let before_source = match before_path {
            Some(path) => match git_show(project_root, base_ref, path) {
                Ok(source) => source,
                Err(err) => {
                    diagnostics.push(err.to_string());
                    None
                }
            },
            None => None,
        };
        let after_source = match after_path {
            Some(path) => {
                let path_abs = project_root.join(path);
                match std::fs::read_to_string(&path_abs) {
                    Ok(source) => Some(source),
                    Err(source) => {
                        diagnostics.push(format!(
                            "failed to read {}: {}",
                            path_abs.display(),
                            source
                        ));
                        None
                    }
                }
            }
            None => None,
        };
        let before_funcs = before_path
            .zip(before_source.as_deref())
            .map(|(path, source)| scan_side(path, source, &mut diagnostics))
            .map(filter_complexity_functions)
            .unwrap_or_default();
        let after_funcs = after_path
            .zip(after_source.as_deref())
            .map(|(path, source)| scan_side(path, source, &mut diagnostics))
            .map(filter_complexity_functions)
            .unwrap_or_default();

        let changed_before = changed_lines(file, false);
        let changed_after = changed_lines(file, true);
        if let Some(path) = after_path.or(before_path) {
            stability_evidence.extend(detect_stability(
                path,
                before_source.as_deref(),
                after_source.as_deref(),
                &changed_after,
                &changed_after,
            ));
        }
        let touched_before = touched_functions(&before_funcs, &changed_before);
        let touched_after = touched_functions(&after_funcs, &changed_after);
        insert_changed_keys(&mut changed_keys, &touched_before, &touched_after);

        let before_by_stable = unique_by_stable(&before_funcs);
        let after_by_stable = unique_by_stable(&after_funcs);
        let before_by_identity = unique_by_identity(&before_funcs);
        let after_unique_identities = unique_identity_keys(&after_funcs);
        let mut matched_before_keys = HashSet::new();
        let before_touched_stable: HashSet<u64> = touched_before
            .iter()
            .filter(|f| f.stable_signature_hash != 0)
            .map(|f| f.stable_signature_hash)
            .collect();

        evidences.extend(collect_after_impacts(
            &touched_after,
            &before_by_stable,
            &before_by_identity,
            &after_unique_identities,
            &mut matched_before_keys,
        ));
        evidences.extend(collect_unmatched_before_impacts(
            &touched_before,
            &after_by_stable,
            &before_touched_stable,
            &matched_before_keys,
        ));
    }

    evidences.sort_by(|a, b| a.file.cmp(&b.file).then(a.function.cmp(&b.function)));
    stability_evidence.sort_by(|a, b| a.file.cmp(&b.file).then(a.kind.cmp(&b.kind)));
    let mut semantic_evidence = detect_semantic(diff);
    semantic_evidence.sort_by(|a, b| a.file.cmp(&b.file).then(a.kind.cmp(&b.kind)));
    let quality = detect_quality(&quality_file_changes(diff, &stability_evidence));
    let history = analyze_history_at(project_root, base_ref, &history_file_changes(diff));
    let file_changes = evidence_file_changes(diff);
    diagnostics.extend(history.diagnostics.iter().cloned());

    Ok(ImpactReport {
        changed_functions: changed_keys.len(),
        evidences,
        stability_evidence,
        semantic_evidence,
        quality,
        history,
        file_changes,
        diagnostics,
    })
}

fn is_rust_change(before_path: Option<&str>, after_path: Option<&str>) -> bool {
    before_path
        .or(after_path)
        .is_some_and(|path| path.ends_with(".rs"))
        || after_path.is_some_and(|path| path.ends_with(".rs"))
}

fn quality_file_changes(diff: &GitDiff, stability: &[EvidenceRecord]) -> Vec<QualityFileChange> {
    diff.files
        .iter()
        .map(|file| {
            let path = file.new_path.as_deref().or(file.old_path.as_deref());
            let stability_sensitive = path.is_some_and(|path| {
                stability
                    .iter()
                    .any(|evidence| evidence.file == path && !evidence.kind.is_empty())
            });
            QualityFileChange {
                old_path: file.old_path.clone(),
                new_path: file.new_path.clone(),
                added_lines: file.added_lines.unwrap_or(0),
                deleted_lines: file.deleted_lines.unwrap_or(0),
                sensitive: stability_sensitive || path.is_some_and(is_sensitive_path),
                executable_code_changed: has_executable_code_change(file),
                test_code_changed: has_test_code_change(file),
            }
        })
        .collect()
}

fn has_executable_code_change(file: &crate::git_diff::GitFileDiff) -> bool {
    file.hunks.iter().flat_map(|hunk| &hunk.lines).any(|line| {
        matches!(line.kind, HunkLineKind::Addition | HunkLineKind::Deletion)
            && is_executable_code_line(&line.content)
    })
}

fn is_executable_code_line(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty()
        && !trimmed.starts_with("//")
        && !trimmed.starts_with("/*")
        && !trimmed.starts_with('*')
        && !trimmed.starts_with("*/")
}

fn has_test_code_change(file: &crate::git_diff::GitFileDiff) -> bool {
    file.hunks.iter().any(|hunk| {
        let has_changed_line = hunk
            .lines
            .iter()
            .any(|line| matches!(line.kind, HunkLineKind::Addition | HunkLineKind::Deletion));
        has_changed_line
            && (hunk.header.contains("mod tests")
                || hunk.header.contains("#[cfg(test)]")
                || hunk.lines.iter().any(|line| {
                    let trimmed = line.content.trim();
                    trimmed.starts_with("#[test]") || trimmed.starts_with("#[cfg(test)]")
                }))
    })
}

fn history_file_changes(diff: &GitDiff) -> Vec<HistoryFileChange> {
    diff.files
        .iter()
        .map(|file| HistoryFileChange {
            old_path: file.old_path.clone(),
            new_path: file.new_path.clone(),
        })
        .collect()
}

fn evidence_file_changes(diff: &GitDiff) -> Vec<EvidenceFileChange> {
    diff.files
        .iter()
        .map(|file| EvidenceFileChange {
            status: match file.status {
                GitFileStatus::Added => "added",
                GitFileStatus::Modified => "modified",
                GitFileStatus::Deleted => "deleted",
                GitFileStatus::Renamed => "renamed",
            }
            .to_string(),
            old_file: file.old_path.clone(),
            new_file: file.new_path.clone(),
            added_lines: file.added_lines.unwrap_or(0),
            removed_lines: file.deleted_lines.unwrap_or(0),
        })
        .collect()
}

pub fn diff_evidence_records(report: &ImpactReport) -> Vec<DiffEvidenceRecord> {
    let mut records = Vec::new();
    for evidence in &report.evidences {
        records.push(DiffEvidenceRecord {
            category: "complexity".to_string(),
            kind: evidence.kind.as_str().to_string(),
            file: evidence.file.clone(),
            subject: evidence.function.clone(),
            source: evidence.source.to_string(),
        });
    }
    for evidence in &report.stability_evidence {
        records.push(record_from_evidence("stability", evidence));
    }
    for evidence in &report.semantic_evidence {
        records.push(record_from_evidence("semantic", evidence));
    }
    for evidence in &report.quality.records {
        records.push(record_from_evidence("quality", evidence));
    }
    for evidence in &report.history.records {
        records.push(record_from_evidence("history", evidence));
    }
    records
}

fn detect_semantic(diff: &GitDiff) -> Vec<EvidenceRecord> {
    let mut records = Vec::new();
    for file in &diff.files {
        let Some(path) = file.new_path.as_deref().or(file.old_path.as_deref()) else {
            continue;
        };
        records.extend(detect_default_value_changes(path, file));
        records.extend(detect_dependency_feature_changes(path, file));
        records.extend(detect_config_value_changes(path, file));
    }
    records
}

fn detect_default_value_changes(
    path: &str,
    file: &crate::git_diff::GitFileDiff,
) -> Vec<EvidenceRecord> {
    if !path.ends_with(".rs") {
        return Vec::new();
    }

    let mut records = Vec::new();
    for hunk in &file.hunks {
        let deleted: Vec<_> = hunk
            .lines
            .iter()
            .filter(|line| matches!(line.kind, HunkLineKind::Deletion))
            .filter_map(|line| parse_named_value(&line.content))
            .collect();
        let added: Vec<_> = hunk
            .lines
            .iter()
            .filter(|line| matches!(line.kind, HunkLineKind::Addition))
            .filter_map(|line| parse_named_value(&line.content))
            .collect();

        for before in &deleted {
            for after in &added {
                if before.name == after.name && before.value != after.value {
                    records.push(EvidenceRecord::new(
                        "default_value_changed",
                        path,
                        format!("{}: {} -> {}", before.name, before.value, after.value),
                        "semantic_diff",
                    ));
                }
            }
        }
    }
    records
}

fn detect_dependency_feature_changes(
    path: &str,
    file: &crate::git_diff::GitFileDiff,
) -> Vec<EvidenceRecord> {
    if !path.ends_with("Cargo.toml") {
        return Vec::new();
    }

    let mut records = Vec::new();
    for line in file
        .hunks
        .iter()
        .flat_map(|hunk| &hunk.lines)
        .filter(|line| matches!(line.kind, HunkLineKind::Addition))
    {
        let trimmed = line.content.trim();
        let lower = trimmed.to_ascii_lowercase();
        if !lower.contains("features") {
            continue;
        }

        let Some(feature) = first_sensitive_dependency_feature(&lower) else {
            continue;
        };
        let dependency = dependency_name(trimmed).unwrap_or("unknown");
        records.push(EvidenceRecord::new(
            "security_sensitive_dependency_changed",
            path,
            format!("{dependency} feature {feature}"),
            "semantic_diff",
        ));
    }
    records
}

fn detect_config_value_changes(
    path: &str,
    file: &crate::git_diff::GitFileDiff,
) -> Vec<EvidenceRecord> {
    if !is_config_value_path(path) {
        return Vec::new();
    }

    let mut records = Vec::new();
    for hunk in &file.hunks {
        let deleted: Vec<_> = hunk
            .lines
            .iter()
            .filter(|line| matches!(line.kind, HunkLineKind::Deletion))
            .filter_map(|line| parse_config_value(&line.content))
            .collect();
        let added: Vec<_> = hunk
            .lines
            .iter()
            .filter(|line| matches!(line.kind, HunkLineKind::Addition))
            .filter_map(|line| parse_config_value(&line.content))
            .collect();

        for before in &deleted {
            for after in &added {
                if before.name == after.name && before.value != after.value {
                    records.push(EvidenceRecord::new(
                        "config_value_changed",
                        path,
                        format!("{}: {} -> {}", before.name, before.value, after.value),
                        "semantic_diff",
                    ));
                }
            }
        }
    }
    records
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NamedValue {
    name: String,
    value: String,
}

fn parse_named_value(line: &str) -> Option<NamedValue> {
    let trimmed = strip_visibility(line.trim());
    let rest = trimmed
        .strip_prefix("const ")
        .or_else(|| trimmed.strip_prefix("static "))
        .or_else(|| trimmed.strip_prefix("let mut "))
        .or_else(|| trimmed.strip_prefix("let "))?;
    let (lhs, rhs) = rest.split_once('=')?;
    let name = lhs.split(':').next()?.trim();
    if !is_default_like_name(name) {
        return None;
    }
    let value = rhs.trim().trim_end_matches(';').trim();
    if name.is_empty() || value.is_empty() {
        return None;
    }
    Some(NamedValue {
        name: name.to_string(),
        value: value.to_string(),
    })
}

fn strip_visibility(line: &str) -> &str {
    line.strip_prefix("pub(crate) ")
        .or_else(|| line.strip_prefix("pub(super) "))
        .or_else(|| line.strip_prefix("pub "))
        .unwrap_or(line)
}

fn is_default_like_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    name.chars()
        .filter(|ch| ch.is_ascii_alphabetic())
        .all(|ch| ch.is_ascii_uppercase())
        || [
            "default",
            "limit",
            "max",
            "min",
            "timeout",
            "retry",
            "capacity",
            "buffer",
            "window",
            "threshold",
        ]
        .iter()
        .any(|needle| lower.contains(needle))
}

fn first_sensitive_dependency_feature(line: &str) -> Option<&'static str> {
    [
        "dangerous_configuration",
        "dangerous",
        "unsafe",
        "native-tls",
        "openssl",
        "rustls",
        "crypto",
        "ffi",
    ]
    .iter()
    .copied()
    .find(|feature| line.contains(feature))
}

fn dependency_name(line: &str) -> Option<&str> {
    let (name, _) = line.split_once('=')?;
    let name = name.trim();
    if name.is_empty() || name.contains(' ') || name.starts_with('[') {
        None
    } else {
        Some(name)
    }
}

fn is_config_value_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with("cargo.toml") || lower.ends_with("cargo.lock") {
        return false;
    }
    lower.contains("config")
        && (lower.ends_with(".toml")
            || lower.ends_with(".yaml")
            || lower.ends_with(".yml")
            || lower.ends_with(".json")
            || lower.ends_with(".env"))
}

fn parse_config_value(line: &str) -> Option<NamedValue> {
    let trimmed = line.trim().trim_end_matches(',').trim();
    if trimmed.is_empty()
        || trimmed.starts_with('#')
        || trimmed.starts_with("//")
        || trimmed.starts_with('[')
    {
        return None;
    }

    let (key, value) = trimmed
        .split_once('=')
        .or_else(|| trimmed.split_once(':'))?;
    let key = key.trim().trim_matches('"').trim_matches('\'');
    if !is_default_like_name(key) {
        return None;
    }
    let value = value.trim().trim_matches('"').trim_matches('\'');
    if key.is_empty() || value.is_empty() || value == "{" || value == "[" {
        return None;
    }
    Some(NamedValue {
        name: key.to_string(),
        value: value.to_string(),
    })
}

fn record_from_evidence(category: &str, evidence: &EvidenceRecord) -> DiffEvidenceRecord {
    DiffEvidenceRecord {
        category: category.to_string(),
        kind: evidence.kind.clone(),
        file: evidence.file.clone(),
        subject: evidence.subject.clone(),
        source: evidence.source.to_string(),
    }
}

fn is_sensitive_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    [
        "ffi",
        "storage",
        "migration",
        "protocol",
        "serde",
        "config",
        "network",
        "transport",
        "crypto",
        "auth",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
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

fn filter_complexity_functions(functions: Vec<Function>) -> Vec<Function> {
    functions
        .into_iter()
        .filter(|function| file_role::classify(&function.file).is_prod())
        .filter(|function| !is_test_like_function(function))
        .collect()
}

fn is_test_like_function(function: &Function) -> bool {
    function.cfg_context.contains("test")
        || function.name.starts_with("test_")
        || function.name.ends_with("_test")
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

fn insert_changed_keys(
    changed_keys: &mut BTreeSet<String>,
    touched_before: &[&Function],
    touched_after: &[&Function],
) {
    for f in touched_before.iter().chain(touched_after.iter()) {
        changed_keys.insert(format!("{}::{}", f.file, f.name));
    }
}

fn collect_after_impacts(
    touched_after: &[&Function],
    before_by_stable: &HashMap<u64, &Function>,
    before_by_identity: &HashMap<String, &Function>,
    after_unique_identities: &HashSet<String>,
    matched_before_keys: &mut HashSet<String>,
) -> Vec<ImpactEvidence> {
    let mut evidences = Vec::new();
    for after in touched_after {
        let previous = match_previous_function(
            after,
            before_by_stable,
            before_by_identity,
            after_unique_identities,
        );
        match previous {
            Some(before) => {
                matched_before_keys.insert(function_instance_key(before));
                if let Some(kind) = classify_metrics(before.metrics, after.metrics) {
                    evidences.push(impact_evidence(
                        kind,
                        after,
                        Some(before.metrics),
                        Some(after.metrics),
                    ));
                }
            }
            None if exceeds_any_threshold(after.metrics) => {
                evidences.push(impact_evidence(
                    EvidenceKind::NewHighRisk,
                    after,
                    None,
                    Some(after.metrics),
                ));
            }
            None => {}
        }
    }
    evidences
}

fn collect_unmatched_before_impacts(
    touched_before: &[&Function],
    after_by_stable: &HashMap<u64, &Function>,
    before_touched_stable: &HashSet<u64>,
    matched_before_keys: &HashSet<String>,
) -> Vec<ImpactEvidence> {
    let mut evidences = Vec::new();
    for before in touched_before {
        if is_matched_before_function(
            before,
            after_by_stable,
            before_touched_stable,
            matched_before_keys,
        ) {
            continue;
        }
        if exceeds_any_threshold(before.metrics) {
            evidences.push(impact_evidence(
                EvidenceKind::TouchedExistingRisky,
                before,
                Some(before.metrics),
                None,
            ));
        }
    }
    evidences
}

fn is_matched_before_function(
    before: &Function,
    after_by_stable: &HashMap<u64, &Function>,
    before_touched_stable: &HashSet<u64>,
    matched_before_keys: &HashSet<String>,
) -> bool {
    matched_before_keys.contains(&function_instance_key(before))
        || before.stable_signature_hash != 0
            && after_by_stable.contains_key(&before.stable_signature_hash)
        || before.stable_signature_hash != 0
            && before_touched_stable.contains(&before.stable_signature_hash)
}

fn impact_evidence(
    kind: EvidenceKind,
    function: &Function,
    previous: Option<Metrics>,
    current: Option<Metrics>,
) -> ImpactEvidence {
    ImpactEvidence {
        kind,
        function: function.name.clone(),
        file: function.file.clone(),
        source: "diff_hunk+ast_snapshot",
        previous,
        current,
    }
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

fn match_previous_function<'a>(
    after: &Function,
    before_by_stable: &HashMap<u64, &'a Function>,
    before_by_identity: &HashMap<String, &'a Function>,
    after_unique_identities: &HashSet<String>,
) -> Option<&'a Function> {
    before_by_stable
        .get(&after.stable_signature_hash)
        .copied()
        .filter(|_| after.stable_signature_hash != 0)
        .or_else(|| {
            let identity = function_identity_key(after);
            after_unique_identities
                .contains(&identity)
                .then(|| before_by_identity.get(&identity).copied())
                .flatten()
        })
}

fn unique_by_identity(functions: &[Function]) -> HashMap<String, &Function> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for f in functions {
        *counts.entry(function_identity_key(f)).or_insert(0) += 1;
    }
    functions
        .iter()
        .filter(|f| counts.get(&function_identity_key(f)) == Some(&1))
        .map(|f| (function_identity_key(f), f))
        .collect()
}

fn unique_identity_keys(functions: &[Function]) -> HashSet<String> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for f in functions {
        *counts.entry(function_identity_key(f)).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .filter_map(|(key, count)| (count == 1).then_some(key))
        .collect()
}

fn function_identity_key(function: &Function) -> String {
    format!(
        "{}::{}::{}::{}",
        function.file, function.impl_context, function.cfg_context, function.name
    )
}

fn function_instance_key(function: &Function) -> String {
    format!(
        "{}::{}::{}::{}::{}::{}",
        function.file,
        function.impl_context,
        function.cfg_context,
        function.name,
        function.start_line,
        function.end_line
    )
}

fn classify_metrics(previous: Metrics, current: Metrics) -> Option<EvidenceKind> {
    let mut worsened = false;
    let mut improved = false;
    let mut touched_risky = exceeds_any_threshold(previous) || exceeds_any_threshold(current);

    for def in metric::ACTIVE_METRICS {
        let p = previous.value(def.id);
        let c = current.value(def.id);
        if metric::crossed_threshold(p, c, def.threshold)
            || metric::worsened_over_threshold(p, c, def.threshold)
        {
            worsened = true;
        } else if p > def.threshold && c < p {
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
