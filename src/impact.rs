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
use crate::git_diff::{parse_git_diff, GitDiff, GitFileStatus, GitHunk, HunkLineKind};
use crate::metric;
use crate::pipeline;
use crate::types::{Function, Metrics};

#[derive(Debug, Clone)]
pub struct ImpactReport {
    pub changed_functions: usize,
    pub evidences: Vec<ImpactEvidence>,
    pub stability_evidence: Vec<EvidenceRecord>,
    pub semantic_evidence: Vec<EvidenceRecord>,
    pub coverage_evidence: Vec<EvidenceRecord>,
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
    let coverage_evidence = coverage_evidence(&diagnostics);

    Ok(ImpactReport {
        changed_functions: changed_keys.len(),
        evidences,
        stability_evidence,
        semantic_evidence,
        coverage_evidence,
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
            let mechanical = is_mechanical_lint_change(file);
            QualityFileChange {
                old_path: file.old_path.clone(),
                new_path: file.new_path.clone(),
                added_lines: file.added_lines.unwrap_or(0),
                deleted_lines: file.deleted_lines.unwrap_or(0),
                sensitive: stability_sensitive || path.is_some_and(is_sensitive_path),
                executable_code_changed: has_executable_code_change(file) && !mechanical,
                test_code_changed: has_test_code_change(file),
            }
        })
        .collect()
}

fn is_mechanical_lint_change(file: &crate::git_diff::GitFileDiff) -> bool {
    let mut changed_pairs = 0;
    for hunk in &file.hunks {
        let deleted = hunk
            .lines
            .iter()
            .filter(|line| matches!(line.kind, HunkLineKind::Deletion))
            .map(|line| normalize_mechanical_line(&line.content))
            .collect::<Vec<_>>();
        let added = hunk
            .lines
            .iter()
            .filter(|line| matches!(line.kind, HunkLineKind::Addition))
            .map(|line| normalize_mechanical_line(&line.content))
            .collect::<Vec<_>>();

        if deleted.len() != added.len() || deleted.is_empty() {
            return false;
        }
        for (before, after) in deleted.iter().zip(added.iter()) {
            if !is_map_clone_to_cloned(before, after) {
                return false;
            }
            changed_pairs += 1;
        }
    }
    changed_pairs > 0
}

fn normalize_mechanical_line(line: &str) -> String {
    line.split_whitespace().collect::<String>()
}

fn is_map_clone_to_cloned(before: &str, after: &str) -> bool {
    if before.contains(".map(|") && before.contains(".clone())") && after.contains(".cloned()") {
        let Some((_prefix, rest)) = before.split_once(".map(|") else {
            return false;
        };
        let Some((param, _suffix)) = rest.split_once('|') else {
            return false;
        };
        let closure = format!(".map(|{0}|{0}.clone())", param.trim());
        return before.replace(&closure, ".cloned()") == after;
    }

    if after.contains(".copied()") {
        return map_deref_to_copied(before, after) || map_identity_ref_to_copied(before, after);
    }

    false
}

fn map_deref_to_copied(before: &str, after: &str) -> bool {
    let Some((_prefix, rest)) = before.split_once(".map(|") else {
        return false;
    };
    let Some((param, _suffix)) = rest.split_once('|') else {
        return false;
    };
    let param = param.trim();
    let closure = format!(".map(|{param}|*{param})");
    before.replace(&closure, ".copied()") == after
}

fn map_identity_ref_to_copied(before: &str, after: &str) -> bool {
    let Some((_prefix, rest)) = before.split_once(".map(|&") else {
        return false;
    };
    let Some((param, _suffix)) = rest.split_once('|') else {
        return false;
    };
    let param = param.trim();
    let closure = format!(".map(|&{param}|{param})");
    before.replace(&closure, ".copied()") == after
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
        let (confidence, value) = evidence_labels("complexity", evidence.kind.as_str());
        records.push(DiffEvidenceRecord {
            category: "complexity".to_string(),
            kind: evidence.kind.as_str().to_string(),
            file: evidence.file.clone(),
            subject: evidence.function.clone(),
            source: evidence.source.to_string(),
            confidence: confidence.to_string(),
            value: value.to_string(),
        });
    }
    for evidence in &report.coverage_evidence {
        records.push(record_from_evidence("coverage", evidence));
    }
    for evidence in &report.stability_evidence {
        records.push(record_from_evidence("stability", evidence));
    }
    for evidence in &report.quality.records {
        records.push(record_from_evidence("quality", evidence));
    }
    for evidence in &report.semantic_evidence {
        records.push(record_from_evidence("semantic", evidence));
    }
    for evidence in &report.history.records {
        records.push(record_from_evidence("history", evidence));
    }
    records
}

pub fn evidence_labels(category: &str, kind: &str) -> (&'static str, &'static str) {
    (
        evidence_confidence(category, kind),
        evidence_value(category, kind),
    )
}

pub fn evidence_confidence(category: &str, kind: &str) -> &'static str {
    match category {
        "complexity" => match kind {
            "new_high_risk" | "worsened" => "high",
            "improved" => "medium",
            "touched_existing_risky" | "identity_uncertain" => "low",
            _ => "low",
        },
        "coverage" => "high",
        "stability" => match kind {
            "unsafe_added"
            | "panic_like_added"
            | "public_api_signature_changed"
            | "public_api_added"
            | "public_api_function_added"
            | "storage_migration_touched"
            | "protocol_shape_changed" => "high",
            "ffi_boundary_touched" => "medium",
            _ => "medium",
        },
        "quality" => match kind {
            "sensitive_test_gap" => "medium",
            "test_gap" | "prod_change_with_tests" => "low",
            "blast_radius" | "mixed_concerns" | "dependency_changed" => "low",
            _ => "low",
        },
        "semantic" => match kind {
            "protocol_header_authority_changed"
            | "boundary_condition_changed"
            | "boundary_expression_changed"
            | "integer_platform_boundary_changed"
            | "concurrency_ordering_changed"
            | "lifecycle_ordering_changed"
            | "panic_to_error_propagation_changed"
            | "readiness_error_propagation_changed"
            | "drop_error_propagation_changed" => "high",
            "error_propagation_changed" => "medium",
            "default_value_changed"
            | "config_value_changed"
            | "env_default_changed"
            | "security_sensitive_dependency_changed" => "medium",
            _ => "medium",
        },
        "history" => "low",
        _ => "low",
    }
}

pub fn evidence_value(category: &str, kind: &str) -> &'static str {
    match category {
        "complexity" => match kind {
            "new_high_risk" | "worsened" | "improved" => "direct",
            "touched_existing_risky" | "identity_uncertain" => "context",
            _ => "context",
        },
        "coverage" => "coverage_limit",
        "stability" | "semantic" => "direct",
        "quality" => match kind {
            "test_gap" | "sensitive_test_gap" | "prod_change_with_tests" => "supporting",
            "blast_radius" | "mixed_concerns" | "dependency_changed" => "context",
            _ => "context",
        },
        "history" => "context",
        _ => "context",
    }
}

fn coverage_evidence(diagnostics: &[String]) -> Vec<EvidenceRecord> {
    let mut seen = BTreeSet::new();
    let mut records = Vec::new();
    for diagnostic in diagnostics {
        let Some(file) = parse_error_file(diagnostic) else {
            continue;
        };
        if seen.insert(file.clone()) {
            records.push(EvidenceRecord::new(
                "parse_coverage_gap",
                file,
                "decay scan coverage is partial",
                "scan",
            ));
        }
    }
    records
}

fn parse_error_file(diagnostic: &str) -> Option<String> {
    let rest = diagnostic.strip_prefix("parse error in ")?;
    let (file, _) = rest.split_once(':')?;
    Some(file.to_string())
}

fn detect_semantic(diff: &GitDiff) -> Vec<EvidenceRecord> {
    let mut records = Vec::new();
    for file in &diff.files {
        let Some(path) = file.new_path.as_deref().or(file.old_path.as_deref()) else {
            continue;
        };
        records.extend(detect_default_value_changes(path, file));
        records.extend(detect_env_default_changes(path, file));
        records.extend(detect_dependency_feature_changes(path, file));
        records.extend(detect_config_value_changes(path, file));
        records.extend(detect_protocol_header_changes(path, file));
        records.extend(detect_boundary_condition_changes(path, file));
        records.extend(detect_boundary_expression_changes(path, file));
        records.extend(detect_integer_platform_boundary_changes(path, file));
        records.extend(detect_concurrency_ordering_changes(path, file));
        records.extend(detect_lifecycle_ordering_changes(path, file));
        records.extend(detect_panic_to_error_propagation_changes(path, file));
        records.extend(detect_readiness_error_propagation_changes(path, file));
        records.extend(detect_drop_error_propagation_changes(path, file));
        records.extend(detect_error_propagation_changes(path, file));
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

fn detect_env_default_changes(
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
            .filter_map(|line| parse_env_default(&line.content))
            .collect();
        let added: Vec<_> = hunk
            .lines
            .iter()
            .filter(|line| matches!(line.kind, HunkLineKind::Addition))
            .filter_map(|line| parse_env_default(&line.content))
            .collect();

        for before in &deleted {
            for after in &added {
                if before.name == after.name && before.value != after.value {
                    records.push(EvidenceRecord::new(
                        "env_default_changed",
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

fn parse_env_default(line: &str) -> Option<NamedValue> {
    let line = line.trim();
    if !(line.contains("env::var(") && (line.contains("unwrap_or") || line.contains("or_else"))) {
        return None;
    }

    let strings = quoted_strings(line);
    let name = strings.first()?;
    let value = strings.last()?;
    if strings.len() < 2 || name == value {
        return None;
    }
    Some(NamedValue {
        name: name.clone(),
        value: value.clone(),
    })
}

fn quoted_strings(line: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut chars = line.chars();
    while let Some(ch) = chars.next() {
        if ch != '"' {
            continue;
        }
        let mut value = String::new();
        let mut escaped = false;
        for next in chars.by_ref() {
            if escaped {
                value.push(next);
                escaped = false;
            } else if next == '\\' {
                escaped = true;
            } else if next == '"' {
                break;
            } else {
                value.push(next);
            }
        }
        values.push(value);
    }
    values
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

fn detect_protocol_header_changes(
    path: &str,
    file: &crate::git_diff::GitFileDiff,
) -> Vec<EvidenceRecord> {
    if !path.ends_with(".rs") {
        return Vec::new();
    }

    let authority_removed = file.hunks.iter().flat_map(|hunk| &hunk.lines).any(|line| {
        matches!(line.kind, HunkLineKind::Deletion) && line.content.contains(".authority()")
    });
    let host_header_rebuilt = file.hunks.iter().flat_map(|hunk| &hunk.lines).any(|line| {
        matches!(line.kind, HunkLineKind::Addition)
            && line.content.contains("header(")
            && line.content.contains("HOST")
            && line.content.contains("format!(")
            && line.content.contains("host")
            && line.content.contains("port")
    });

    let mut records = Vec::new();
    if authority_removed && host_header_rebuilt {
        records.push(EvidenceRecord::new(
            "protocol_header_authority_changed",
            path,
            "Host header authority -> host/port",
            "semantic_diff",
        ));
    }
    records
}

fn detect_boundary_condition_changes(
    path: &str,
    file: &crate::git_diff::GitFileDiff,
) -> Vec<EvidenceRecord> {
    if !path.ends_with(".rs") || !file_role::classify(path).is_prod() {
        return Vec::new();
    }

    let mut records = Vec::new();
    let mut seen = BTreeSet::new();
    for line in file
        .hunks
        .iter()
        .flat_map(|hunk| &hunk.lines)
        .filter(|line| matches!(line.kind, HunkLineKind::Addition))
    {
        let Some(condition) = boundary_condition_subject(&line.content) else {
            continue;
        };
        if seen.insert(condition.clone()) {
            records.push(EvidenceRecord::new(
                "boundary_condition_changed",
                path,
                condition,
                "semantic_diff",
            ));
        }
    }
    records
}

fn detect_boundary_expression_changes(
    path: &str,
    file: &crate::git_diff::GitFileDiff,
) -> Vec<EvidenceRecord> {
    if !path.ends_with(".rs") || !file_role::classify(path).is_prod() {
        return Vec::new();
    }

    let mut records = Vec::new();
    let mut seen = BTreeSet::new();
    for line in file
        .hunks
        .iter()
        .flat_map(|hunk| &hunk.lines)
        .filter(|line| matches!(line.kind, HunkLineKind::Addition))
    {
        let Some(expression) = boundary_expression_subject(&line.content) else {
            continue;
        };
        if seen.insert(expression.clone()) {
            records.push(EvidenceRecord::new(
                "boundary_expression_changed",
                path,
                expression,
                "semantic_diff",
            ));
        }
    }
    records
}

fn detect_integer_platform_boundary_changes(
    path: &str,
    file: &crate::git_diff::GitFileDiff,
) -> Vec<EvidenceRecord> {
    if !path.ends_with(".rs") || !file_role::classify(path).is_prod() {
        return Vec::new();
    }

    let mut records = Vec::new();
    let mut seen = BTreeSet::new();
    for hunk in &file.hunks {
        for line in hunk
            .lines
            .iter()
            .filter(|line| matches!(line.kind, HunkLineKind::Addition))
        {
            let Some(subject) = integer_platform_boundary_subject(&line.content) else {
                continue;
            };
            if seen.insert(subject.clone()) {
                records.push(EvidenceRecord::new(
                    "integer_platform_boundary_changed",
                    path,
                    subject,
                    "semantic_diff",
                ));
            }
        }
    }
    records
}

fn detect_concurrency_ordering_changes(
    path: &str,
    file: &crate::git_diff::GitFileDiff,
) -> Vec<EvidenceRecord> {
    if !path.ends_with(".rs") || !file_role::classify(path).is_prod() {
        return Vec::new();
    }

    let mut records = Vec::new();
    let mut seen = BTreeSet::new();
    for hunk in &file.hunks {
        for subject in [
            blocking_pool_race_subject(hunk),
            wakeup_repoll_subject(hunk),
            completion_abort_ordering_subject(hunk),
        ]
        .into_iter()
        .flatten()
        {
            if subject.is_empty() {
                continue;
            }
            if seen.insert(subject.clone()) {
                records.push(EvidenceRecord::new(
                    "concurrency_ordering_changed",
                    path,
                    subject,
                    "semantic_diff",
                ));
            }
        }
    }
    records
}

fn detect_lifecycle_ordering_changes(
    path: &str,
    file: &crate::git_diff::GitFileDiff,
) -> Vec<EvidenceRecord> {
    if !path.ends_with(".rs") || !file_role::classify(path).is_prod() {
        return Vec::new();
    }

    let mut records = Vec::new();
    let mut seen = BTreeSet::new();
    for hunk in &file.hunks {
        for subject in [
            graceful_shutdown_upgrade_subject(hunk),
            initial_state_shutdown_subject(hunk),
            cancellation_reset_subject(hunk),
        ]
        .into_iter()
        .flatten()
        {
            if subject.is_empty() {
                continue;
            }
            if seen.insert(subject.clone()) {
                records.push(EvidenceRecord::new(
                    "lifecycle_ordering_changed",
                    path,
                    subject,
                    "semantic_diff",
                ));
            }
        }
    }
    records
}

fn detect_panic_to_error_propagation_changes(
    path: &str,
    file: &crate::git_diff::GitFileDiff,
) -> Vec<EvidenceRecord> {
    if !path.ends_with(".rs") || !file_role::classify(path).is_prod() {
        return Vec::new();
    }

    let mut records = Vec::new();
    let mut seen = BTreeSet::new();
    for hunk in &file.hunks {
        let removed_panic_like = hunk.lines.iter().any(|line| {
            matches!(line.kind, HunkLineKind::Deletion)
                && contains_panic_like_conversion(&line.content)
        });
        if !removed_panic_like {
            continue;
        }
        for line in hunk
            .lines
            .iter()
            .filter(|line| matches!(line.kind, HunkLineKind::Addition))
        {
            let Some(subject) = error_propagation_subject(&line.content) else {
                continue;
            };
            if seen.insert(subject.clone()) {
                records.push(EvidenceRecord::new(
                    "panic_to_error_propagation_changed",
                    path,
                    subject,
                    "semantic_diff",
                ));
            }
        }
    }
    records
}

fn detect_readiness_error_propagation_changes(
    path: &str,
    file: &crate::git_diff::GitFileDiff,
) -> Vec<EvidenceRecord> {
    if !path.ends_with(".rs") || !file_role::classify(path).is_prod() {
        return Vec::new();
    }

    let mut records = Vec::new();
    let mut seen = BTreeSet::new();
    for hunk in &file.hunks {
        let removed_plain_readiness = hunk.lines.iter().any(|line| {
            matches!(line.kind, HunkLineKind::Deletion)
                && contains_readiness_io_registration(&line.content)
                && contains_receive_like_operation(&line.content)
                && !line.content.contains("Interest::ERROR")
                && (line.content.contains("Interest::READABLE")
                    || line.content.contains("Interest::WRITABLE"))
        });
        let receive_like_context = hunk
            .lines
            .iter()
            .any(|line| contains_receive_like_operation(&line.content));
        if !removed_plain_readiness || !receive_like_context {
            continue;
        }
        for line in hunk
            .lines
            .iter()
            .filter(|line| matches!(line.kind, HunkLineKind::Addition))
        {
            let Some(subject) = readiness_error_propagation_subject(&line.content) else {
                continue;
            };
            if seen.insert(subject.clone()) {
                records.push(EvidenceRecord::new(
                    "readiness_error_propagation_changed",
                    path,
                    subject,
                    "semantic_diff",
                ));
            }
        }
    }
    records
}

fn detect_drop_error_propagation_changes(
    path: &str,
    file: &crate::git_diff::GitFileDiff,
) -> Vec<EvidenceRecord> {
    if !path.ends_with(".rs") || !file_role::classify(path).is_prod() {
        return Vec::new();
    }

    let mut records = Vec::new();
    let mut seen = BTreeSet::new();
    for hunk in &file.hunks {
        let added_drop_context = hunk.lines.iter().any(|line| {
            matches!(line.kind, HunkLineKind::Addition)
                && (line.content.contains("impl Drop for")
                    || line.content.contains("struct SenderDropGuard")
                    || line.content.contains("fn drop(&mut self)"))
        });
        if !added_drop_context {
            continue;
        }
        for line in hunk
            .lines
            .iter()
            .filter(|line| matches!(line.kind, HunkLineKind::Addition))
        {
            let Some(subject) = drop_error_propagation_subject(&line.content) else {
                continue;
            };
            if seen.insert(subject.clone()) {
                records.push(EvidenceRecord::new(
                    "drop_error_propagation_changed",
                    path,
                    subject,
                    "semantic_diff",
                ));
            }
        }
    }
    records
}

fn detect_error_propagation_changes(
    path: &str,
    file: &crate::git_diff::GitFileDiff,
) -> Vec<EvidenceRecord> {
    if !path.ends_with(".rs") || !file_role::classify(path).is_prod() {
        return Vec::new();
    }

    let mut records = Vec::new();
    let mut seen = BTreeSet::new();
    for hunk in &file.hunks {
        let removed_error_flow = hunk.lines.iter().any(|line| {
            matches!(line.kind, HunkLineKind::Deletion) && contains_error_flow(&line.content)
        });
        let added_error_flow = hunk.lines.iter().any(|line| {
            matches!(line.kind, HunkLineKind::Addition) && contains_error_flow(&line.content)
        });
        if !removed_error_flow || !added_error_flow {
            continue;
        }
        let mut best_subject: Option<(u8, String)> = None;
        for line in hunk
            .lines
            .iter()
            .filter(|line| matches!(line.kind, HunkLineKind::Addition))
        {
            let Some(subject) = error_propagation_change_subject(&line.content) else {
                continue;
            };
            let score = error_propagation_subject_score(&subject);
            if best_subject
                .as_ref()
                .is_none_or(|(best_score, _)| score > *best_score)
            {
                best_subject = Some((score, subject));
            }
        }
        if let Some((_, subject)) = best_subject {
            if seen.insert(subject.clone()) {
                records.push(EvidenceRecord::new(
                    "error_propagation_changed",
                    path,
                    subject,
                    "semantic_diff",
                ));
            }
        }
    }
    records
}

fn boundary_condition_subject(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with("//") {
        return None;
    }
    if !(trimmed.starts_with("if ")
        || trimmed.starts_with("else if ")
        || trimmed.starts_with("while ")
        || trimmed.starts_with("assert!")
        || trimmed.starts_with("debug_assert!")
        || trimmed.starts_with("match "))
    {
        return None;
    }
    if !contains_condition_operator(trimmed) || !contains_boundary_token(trimmed) {
        return None;
    }
    Some(compact_subject(trimmed))
}

fn boundary_expression_subject(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with("//") || boundary_condition_subject(trimmed).is_some() {
        return None;
    }
    if contains_readiness_io_registration(trimmed) && trimmed.contains("Interest::ERROR") {
        return None;
    }
    if !contains_boundary_token(trimmed) || !contains_boundary_expression_operator(trimmed) {
        return None;
    }
    Some(compact_subject(trimmed.trim_end_matches(',')))
}

fn graceful_shutdown_upgrade_subject(hunk: &GitHunk) -> Option<String> {
    let removed_unwrap_shutdown = hunk.lines.iter().any(|line| {
        matches!(line.kind, HunkLineKind::Deletion)
            && line.content.contains("graceful_shutdown")
            && line.content.contains("unwrap")
    });
    if !removed_unwrap_shutdown {
        return None;
    }
    let added_guard = hunk.lines.iter().any(|line| {
        matches!(line.kind, HunkLineKind::Addition)
            && line.content.contains("if let Some(")
            && line.content.contains("inner.as_mut()")
    });
    if !added_guard {
        return None;
    }
    let added_shutdown = hunk.lines.iter().find(|line| {
        matches!(line.kind, HunkLineKind::Addition)
            && line.content.contains("graceful_shutdown")
            && !line.content.trim_start().starts_with("//")
    })?;
    Some(compact_subject(
        added_shutdown.content.trim_end_matches(','),
    ))
}

fn initial_state_shutdown_subject(hunk: &GitHunk) -> Option<String> {
    let condition = hunk.lines.iter().find(|line| {
        matches!(line.kind, HunkLineKind::Addition)
            && line.content.trim_start().starts_with("if ")
            && line.content.contains("is_write_closed()")
            && line.content.contains("has_initial_read_write_state()")
    })?;
    Some(compact_subject(condition.content.trim_end_matches(',')))
}

fn blocking_pool_race_subject(hunk: &GitHunk) -> Option<String> {
    let has_idle_marker = hunk.lines.iter().any(|line| {
        matches!(line.kind, HunkLineKind::Addition)
            && (line.content.contains("is_counted_idle")
                || line.content.contains("num_idle_threads"))
    });
    if !has_idle_marker {
        return None;
    }
    let subject = hunk.lines.iter().find(|line| {
        matches!(line.kind, HunkLineKind::Addition)
            && (line.content.contains("if is_counted_idle")
                || line.content.contains("let mut is_counted_idle")
                || line.content.contains("is_counted_idle = false"))
    })?;
    Some(compact_subject(subject.content.trim_end_matches(',')))
}

fn wakeup_repoll_subject(hunk: &GitHunk) -> Option<String> {
    let subject = hunk.lines.iter().find(|line| {
        matches!(line.kind, HunkLineKind::Addition)
            && line.content.contains("wants_write_again")
            && line.content.contains("conn_ready")
    })?;
    Some(compact_subject(subject.content.trim_end_matches(',')))
}

fn completion_abort_ordering_subject(hunk: &GitHunk) -> Option<String> {
    let completed_line = hunk.lines.iter().find(|line| {
        matches!(line.kind, HunkLineKind::Addition)
            && line.content.contains("Lifecycle::Completed(cqe)")
    })?;
    let has_cancel_cleanup = hunk.lines.iter().any(|line| {
        matches!(line.kind, HunkLineKind::Addition)
            && (line.content.contains("CancelData::Open(_)")
                || line.content.contains("OwnedFd::from_raw_fd")
                || line.content.contains("waker.wake_by_ref"))
    });
    if !has_cancel_cleanup {
        return None;
    }
    Some(compact_subject(
        completed_line.content.trim_end_matches(','),
    ))
}

fn integer_platform_boundary_subject(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with("//") {
        return None;
    }
    if !contains_pointer_sized_try_conversion(trimmed) {
        return None;
    }
    if !contains_saturating_integer_fallback(trimmed) {
        return None;
    }
    Some(compact_subject(trimmed.trim_end_matches(',')))
}

fn contains_condition_operator(line: &str) -> bool {
    ["==", "!=", "<=", ">=", ">", "<", "&", "|"]
        .iter()
        .any(|operator| line.contains(operator))
}

fn contains_boundary_expression_operator(line: &str) -> bool {
    [
        "&",
        "|",
        "^",
        " as ",
        ".min(",
        ".max(",
        "saturating_",
        "checked_",
    ]
    .iter()
    .any(|operator| line.contains(operator))
}

fn contains_pointer_sized_try_conversion(line: &str) -> bool {
    [
        "usize::try_from",
        "isize::try_from",
        ".try_into::<usize>",
        ".try_into::<isize>",
    ]
    .iter()
    .any(|conversion| line.contains(conversion))
}

fn contains_saturating_integer_fallback(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    (lower.contains("unwrap_or") || lower.contains("unwrap_or_else"))
        && (lower.contains("usize::max")
            || lower.contains("isize::max")
            || lower.contains("usize::min")
            || lower.contains("isize::min")
            || lower.contains("0"))
}

fn contains_panic_like_conversion(line: &str) -> bool {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    (trimmed.contains(".unwrap()") || trimmed.contains(".expect(") || trimmed.contains("panic!("))
        && (lower.contains("from_utf8")
            || lower.contains("to_str")
            || lower.contains("header")
            || lower.contains("parse")
            || lower.contains("invalid")
            || lower.contains("unicode")
            || lower.contains("utf"))
}

fn error_propagation_subject(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with("//") {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    let returns_error = trimmed.contains("return Err(")
        || trimmed.contains("return error(")
        || trimmed.contains("=> return Err(")
        || trimmed.contains("=> return error(")
        || trimmed.contains(".map_err(")
        || trimmed.contains("?;");
    let malformed_context = lower.contains("err")
        || lower.contains("error")
        || lower.contains("invalid")
        || lower.contains("unicode")
        || lower.contains("utf")
        || lower.contains("parse");
    if !returns_error || !malformed_context {
        return None;
    }
    Some(compact_subject(trimmed.trim_end_matches(',')))
}

fn readiness_error_propagation_subject(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with("//") {
        return None;
    }
    if !contains_readiness_io_registration(trimmed) || !trimmed.contains("Interest::ERROR") {
        return None;
    }
    if !contains_receive_like_operation(trimmed) {
        return None;
    }
    Some(compact_subject(trimmed.trim_end_matches(',')))
}

fn drop_error_propagation_subject(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with("//") {
        return None;
    }
    if !trimmed.contains("send_error(") || !trimmed.contains("new_incomplete") {
        return None;
    }
    Some(compact_subject(trimmed.trim_end_matches(',')))
}

fn cancellation_reset_subject(hunk: &GitHunk) -> Option<String> {
    let send_reset = hunk.lines.iter().find(|line| {
        matches!(line.kind, HunkLineKind::Addition)
            && line.content.contains("send_reset(")
            && line.content.contains("CANCEL")
    })?;
    let has_cancel_context = hunk.lines.iter().any(|line| {
        matches!(line.kind, HunkLineKind::Addition)
            && (line.content.contains("poll_canceled")
                || line.content.contains("cancel_rx")
                || line.content.contains("cancel_tx")
                || line.content.contains("cancel_result")
                || line.content.contains(".cancel()"))
    });
    let has_cleanup_action = hunk.lines.iter().any(|line| {
        matches!(line.kind, HunkLineKind::Addition)
            && (line.content.contains("drop(")
                || line.content.contains(".take()")
                || line.content.contains("oneshot::channel"))
    });
    if !has_cancel_context || !has_cleanup_action {
        return None;
    }
    Some(compact_subject(send_reset.content.trim_end_matches(',')))
}

fn error_propagation_change_subject(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with("//") {
        return None;
    }
    if !contains_error_flow(trimmed) {
        return None;
    }
    if !contains_error_flow_change_operator(trimmed) {
        return None;
    }
    Some(compact_subject(trimmed.trim_end_matches(',')))
}

fn contains_readiness_io_registration(line: &str) -> bool {
    line.contains(".async_io(") || line.contains(".try_io(")
}

fn contains_receive_like_operation(line: &str) -> bool {
    [
        ".recv(",
        ".recv_from(",
        ".recv_buf(",
        ".recv_buf_from(",
        ".peek(",
        ".peek_from(",
        "peek_sender_inner(",
    ]
    .iter()
    .any(|needle| line.contains(needle))
}

fn contains_error_flow(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("err(")
        || lower.contains(" err")
        || lower.contains("error")
        || lower.contains("reason::")
        || lower.contains("no_error")
        || lower.contains("cancel")
        || lower.contains("rst_stream")
        || lower.contains("send_error")
        || lower.contains("recv_msg(err")
        || lower.contains("return err")
        || lower.contains("poll::ready(err")
        || lower.contains("poll::ready(some(err")
        || lower.contains("poll::ready(none")
        || lower.contains("parse::internal")
}

fn contains_error_flow_change_operator(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("=>")
        || lower.contains("return")
        || lower.contains("send_error")
        || lower.contains("recv_msg")
        || lower.contains("poll::ready")
        || lower.contains("err(")
        || lower.contains("ok(")
        || lower.contains("none")
        || lower.contains("parse::internal")
}

fn error_propagation_subject_score(subject: &str) -> u8 {
    let lower = subject.to_ascii_lowercase();
    if lower.contains("no_error") || lower.contains("cancel") || lower.contains("rst_stream") {
        6
    } else if lower.contains("send_error") || lower.contains("recv_msg") {
        5
    } else if lower.contains("parse::internal") {
        4
    } else if lower.contains("return") {
        3
    } else if lower.contains("poll::ready") {
        2
    } else {
        1
    }
}

fn contains_boundary_token(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    [
        "== 0",
        "!= 0",
        "> 0",
        "< 0",
        ">= 0",
        "<= 0",
        "usize::max",
        "isize::max",
        "u64::max",
        "u32::max",
        "closed",
        "close",
        "invalid",
        "timeout",
        "cancel",
        "error",
        "eof",
        "empty",
        "null",
        "none",
        "overflow",
        "underflow",
    ]
    .iter()
    .any(|token| lower.contains(token))
}

fn compact_subject(line: &str) -> String {
    let mut value = line.split("//").next().unwrap_or(line).trim().to_string();
    if value.len() > 140 {
        value.truncate(137);
        value.push_str("...");
    }
    value
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
    let (confidence, value) = evidence_labels(category, evidence.kind.as_str());
    DiffEvidenceRecord {
        category: category.to_string(),
        kind: evidence.kind.clone(),
        file: evidence.file.clone(),
        subject: evidence.subject.clone(),
        source: evidence.source.to_string(),
        confidence: confidence.to_string(),
        value: value.to_string(),
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
