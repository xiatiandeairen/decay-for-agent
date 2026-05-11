use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use std::cmp::Reverse;

use clap::Args;

use crate::error::{DecayError, Result};
use crate::metric;
use crate::pipeline;
use crate::scope::ScanScope;
use crate::types::{Function, MetricId};

#[derive(Args, Clone, Debug, Default)]
pub struct ScanArgs {
    /// Exclude a basename, relative path prefix, or simple glob from scanning.
    #[arg(long = "exclude", value_name = "PATTERN", global = true)]
    pub excludes: Vec<String>,

    /// Select which Rust source roles to scan.
    #[arg(long = "scope", value_enum, default_value_t = ScanScope::Prod, global = true)]
    pub scope: ScanScope,

    /// Print expanded diagnostic context.
    #[arg(long, global = true)]
    pub verbose: bool,
}

pub(crate) struct ProjectContext {
    pub root: PathBuf,
    pub project_id: String,
    pub project_key: String,
    pub workspace_root: String,
}

pub(crate) struct ScanResult {
    pub funcs: Vec<Function>,
    pub file_count: usize,
    pub elapsed_secs: f64,
    pub diagnostic_count: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct MetricBreach {
    pub metric: MetricId,
    pub value: u32,
    pub threshold: u32,
    pub overage: u32,
}

pub(crate) fn resolve_project() -> Result<ProjectContext> {
    let cwd = std::env::current_dir().map_err(|source| DecayError::Io {
        path: ".".to_string(),
        source,
    })?;
    let root = git_root(&cwd).unwrap_or(cwd);
    let canonical = root.canonicalize().map_err(|source| DecayError::Io {
        path: root.display().to_string(),
        source,
    })?;
    let workspace_root = canonical.to_string_lossy().to_string();
    let project_key = git_remote_key(&canonical).unwrap_or_else(|| workspace_root.clone());
    Ok(ProjectContext {
        root,
        project_id: workspace_root.clone(),
        project_key,
        workspace_root,
    })
}

fn git_root(cwd: &std::path::Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(cwd)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let root = String::from_utf8(output.stdout).ok()?;
    let root = root.trim();
    (!root.is_empty()).then(|| PathBuf::from(root))
}

fn git_remote_key(root: &std::path::Path) -> Option<String> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let remote = String::from_utf8(output.stdout).ok()?;
    normalize_git_remote(remote.trim())
}

fn normalize_git_remote(remote: &str) -> Option<String> {
    if remote.is_empty() {
        return None;
    }
    let mut s = remote.trim().to_string();
    if let Some(rest) = s.strip_prefix("git@") {
        if let Some((host, path)) = rest.split_once(':') {
            s = format!("{host}/{path}");
        }
    }
    for prefix in ["ssh://git@", "https://", "http://", "ssh://"] {
        if let Some(rest) = s.strip_prefix(prefix) {
            s = rest.to_string();
            break;
        }
    }
    if let Some(rest) = s.strip_prefix("git@") {
        s = rest.to_string();
    }
    if let Some(rest) = s.strip_suffix(".git") {
        s = rest.to_string();
    }
    Some(s.trim_end_matches('/').to_ascii_lowercase())
}

pub(crate) fn scan_current(project_root: &std::path::Path, args: &ScanArgs) -> Result<ScanResult> {
    let started = Instant::now();
    let output = pipeline::scan_with_excludes(project_root, &args.excludes, args.scope)?;
    let elapsed_secs = started.elapsed().as_secs_f64();
    for diagnostic in &output.diagnostics {
        log::warn!(
            "scan diagnostic {}: {}",
            diagnostic.path,
            diagnostic.message
        );
    }

    let mut files: Vec<&str> = output.funcs.iter().map(|f| f.file.as_str()).collect();
    files.sort_unstable();
    files.dedup();
    let file_count = files.len();

    Ok(ScanResult {
        funcs: output.funcs,
        file_count,
        elapsed_secs,
        diagnostic_count: output.diagnostics.len(),
    })
}

pub(crate) fn collect_exceeded(funcs: &[Function]) -> Vec<(&Function, Vec<MetricBreach>)> {
    let mut exceeded: Vec<(&Function, Vec<MetricBreach>)> = funcs
        .iter()
        .filter_map(|f| {
            let breaches = collect_breaches(f);
            (!breaches.is_empty()).then_some((f, breaches))
        })
        .collect();

    for (_, breaches) in &mut exceeded {
        breaches.sort_by_key(|breach| Reverse(breach.overage));
    }
    exceeded.sort_by(|a, b| {
        let am = a.1.iter().map(|m| m.overage).max().unwrap_or(0);
        let bm = b.1.iter().map(|m| m.overage).max().unwrap_or(0);
        bm.cmp(&am)
    });
    exceeded
}

pub(crate) fn collect_breaches(function: &Function) -> Vec<MetricBreach> {
    metric::active_values(function.metrics)
        .filter(|(def, value)| metric::breaches_threshold(*value, def.threshold))
        .map(|(def, value)| MetricBreach {
            metric: def.id,
            value,
            threshold: def.threshold,
            overage: value - def.threshold,
        })
        .collect()
}
