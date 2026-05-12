use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use crate::evidence::EvidenceRecord;
use crate::file_role;

const SOURCE: &str = "git_history";
const COMMIT_MARKER: &str = "__DECAY_COMMIT__";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HistoryReport {
    pub records: Vec<EvidenceRecord>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryFileChange {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
}

impl HistoryFileChange {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            old_path: None,
            new_path: Some(path.into()),
        }
    }

    pub fn path(&self) -> Option<&str> {
        self.new_path
            .as_deref()
            .or(self.old_path.as_deref())
            .filter(|path| !path.trim().is_empty())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct PathHistory {
    commits_30d: usize,
    commits_90d: usize,
    commits_180d: usize,
    churn_180d: u32,
    bugfix_subjects: Vec<String>,
}

pub fn detect_history<P>(project_root: &Path, touched_paths: &[P]) -> HistoryReport
where
    P: AsRef<Path>,
{
    let files = touched_paths
        .iter()
        .filter_map(|path| normalize_repo_path(project_root, path.as_ref()))
        .map(HistoryFileChange::new)
        .collect::<Vec<_>>();
    analyze_history(project_root, &files)
}

pub fn analyze_history(project_root: &Path, files: &[HistoryFileChange]) -> HistoryReport {
    analyze_history_for_ref(project_root, None, files)
}

pub fn analyze_history_at(
    project_root: &Path,
    until_ref: &str,
    files: &[HistoryFileChange],
) -> HistoryReport {
    analyze_history_for_ref(project_root, Some(until_ref), files)
}

fn analyze_history_for_ref(
    project_root: &Path,
    until_ref: Option<&str>,
    files: &[HistoryFileChange],
) -> HistoryReport {
    let mut report = HistoryReport::default();
    let mut seen = BTreeSet::new();

    for file in files {
        let Some(path) = file
            .path()
            .and_then(|path| normalize_repo_path(project_root, Path::new(path)))
        else {
            continue;
        };

        if !file_role::classify(&path).is_prod() || !seen.insert(path.clone()) {
            continue;
        }

        match load_file_history(project_root, until_ref, file) {
            Ok(history) => push_history_records(&mut report.records, &path, &history),
            Err(diagnostic) => report.diagnostics.push(diagnostic),
        }
    }

    report.records.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.kind.cmp(&b.kind))
            .then(a.subject.cmp(&b.subject))
    });
    report.diagnostics.sort();
    report
}

fn load_file_history(
    project_root: &Path,
    until_ref: Option<&str>,
    file: &HistoryFileChange,
) -> Result<PathHistory, String> {
    let mut diagnostics = Vec::new();

    for path in candidate_paths(project_root, file) {
        match load_path_history(project_root, until_ref, &path) {
            Ok(history) if history.commits_180d > 0 => return Ok(history),
            Ok(_) => {}
            Err(diagnostic) => diagnostics.push(diagnostic),
        }
    }

    if diagnostics.is_empty() {
        Ok(PathHistory::default())
    } else {
        Err(diagnostics.join("; "))
    }
}

fn candidate_paths(project_root: &Path, file: &HistoryFileChange) -> Vec<String> {
    let mut paths = Vec::new();
    if let Some(path) = file.new_path.as_deref() {
        if let Some(path) = normalize_repo_path(project_root, Path::new(path)) {
            paths.push(path);
        }
    }
    if let Some(path) = file.old_path.as_deref() {
        if let Some(path) = normalize_repo_path(project_root, Path::new(path)) {
            if !paths.iter().any(|existing| existing == &path) {
                paths.push(path);
            }
        }
    }
    paths
}

fn load_path_history(
    project_root: &Path,
    until_ref: Option<&str>,
    path: &str,
) -> Result<PathHistory, String> {
    let log_180d = git_log_follow(project_root, until_ref, path, "180 days")?;
    let log_90d = git_log_follow(project_root, until_ref, path, "90 days")?;
    let log_30d = git_log_follow(project_root, until_ref, path, "30 days")?;

    let parsed_180d = parse_git_log(&log_180d);
    Ok(PathHistory {
        commits_30d: count_commits(&log_30d),
        commits_90d: count_commits(&log_90d),
        commits_180d: parsed_180d.commits,
        churn_180d: parsed_180d.churn,
        bugfix_subjects: parsed_180d.bugfix_subjects,
    })
}

fn push_history_records(records: &mut Vec<EvidenceRecord>, path: &str, history: &PathHistory) {
    if history.commits_90d >= 3 || history.churn_180d >= 80 {
        records.push(EvidenceRecord::new(
            "churn_hotspot",
            path,
            format!(
                "{} commits in 90d, {} lines changed in 180d",
                history.commits_90d, history.churn_180d
            ),
            SOURCE,
        ));
    }

    if let Some(subject) = history.bugfix_subjects.first() {
        records.push(EvidenceRecord::new(
            "recently_bugfixed_area",
            path,
            subject.clone(),
            SOURCE,
        ));
    }

    if history.commits_30d >= 5 || history.commits_180d >= 5 {
        records.push(EvidenceRecord::new(
            "frequent_rework",
            path,
            format!(
                "{} commits in 30d, {} commits in 180d",
                history.commits_30d, history.commits_180d
            ),
            SOURCE,
        ));
    }
}

#[derive(Debug, Default)]
struct ParsedLog {
    commits: usize,
    churn: u32,
    bugfix_subjects: Vec<String>,
}

fn parse_git_log(log: &str) -> ParsedLog {
    let mut parsed = ParsedLog::default();

    for line in log.lines() {
        if let Some(subject) = line.strip_prefix(COMMIT_MARKER) {
            parsed.commits += 1;
            if is_bugfix_subject(subject) {
                parsed.bugfix_subjects.push(subject.trim().to_string());
            }
            continue;
        }

        let mut fields = line.split('\t');
        let Some(added) = fields.next().and_then(parse_numstat) else {
            continue;
        };
        let Some(deleted) = fields.next().and_then(parse_numstat) else {
            continue;
        };
        parsed.churn = parsed.churn.saturating_add(added.saturating_add(deleted));
    }

    parsed
}

fn count_commits(log: &str) -> usize {
    log.lines()
        .filter(|line| line.starts_with(COMMIT_MARKER))
        .count()
}

fn parse_numstat(value: &str) -> Option<u32> {
    if value == "-" {
        return Some(0);
    }
    value.parse().ok()
}

fn git_log_follow(
    project_root: &Path,
    until_ref: Option<&str>,
    path: &str,
    since: &str,
) -> Result<String, String> {
    let mut args = vec![
        "log",
        "--follow",
        "--numstat",
        "--format=__DECAY_COMMIT__%s",
        "--since",
        since,
    ];
    if let Some(until_ref) = until_ref {
        args.push(until_ref);
    }
    args.extend(["--", path]);

    let output = Command::new("git")
        .args(args)
        .current_dir(project_root)
        .output()
        .map_err(|source| format!("git log --follow failed for {path}: {source}"))?;

    if !output.status.success() {
        return Err(format!(
            "git log --follow failed for {}: {}",
            path,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn is_bugfix_subject(subject: &str) -> bool {
    let subject = subject.to_ascii_lowercase();
    [
        "fix",
        "bug",
        "defect",
        "regression",
        "hotfix",
        "crash",
        "panic",
    ]
    .iter()
    .any(|needle| subject.contains(needle))
}

fn normalize_repo_path(project_root: &Path, path: &Path) -> Option<String> {
    let path = if path.is_absolute() {
        path.strip_prefix(project_root).ok()?
    } else {
        path
    };
    let path = strip_current_dir(path);
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {}
            Component::ParentDir => return None,
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }

    let value = normalized.to_string_lossy().replace('\\', "/");
    (!value.trim().is_empty()).then_some(value)
}

fn strip_current_dir(path: &Path) -> &Path {
    path.strip_prefix(".").unwrap_or(path)
}
