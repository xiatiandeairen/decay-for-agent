use std::path::PathBuf;
use std::time::Instant;

use clap::Args;

use crate::config::{Thresholds, DEFAULT_THRESHOLDS};
use crate::error::{DecayError, Result};
use crate::pipeline;
use crate::scope::ScanScope;
use crate::types::{Function, Metrics};

#[derive(Args, Clone, Debug, Default)]
pub struct ScanArgs {
    /// Exclude a basename, relative path prefix, or simple glob from scanning.
    #[arg(long = "exclude", value_name = "PATTERN", global = true)]
    pub excludes: Vec<String>,

    /// Select which Rust source roles to scan.
    #[arg(long = "scope", value_enum, default_value_t = ScanScope::Prod, global = true)]
    pub scope: ScanScope,
}

pub(crate) struct ProjectContext {
    pub root: PathBuf,
    pub project_id: String,
}

pub(crate) struct ScanResult {
    pub funcs: Vec<Function>,
    pub file_count: usize,
    pub elapsed_secs: f64,
}

pub(crate) struct MetricBreach {
    pub metric: &'static str,
    pub value: u32,
    pub threshold: u32,
    pub overage: u32,
}

pub(crate) fn resolve_project() -> Result<ProjectContext> {
    let root = std::env::current_dir().map_err(|source| DecayError::Io {
        path: ".".to_string(),
        source,
    })?;
    let canonical = root.canonicalize().map_err(|source| DecayError::Io {
        path: root.display().to_string(),
        source,
    })?;
    Ok(ProjectContext {
        root,
        project_id: canonical.to_string_lossy().to_string(),
    })
}

pub(crate) fn scan_current(project_root: &std::path::Path, args: &ScanArgs) -> Result<ScanResult> {
    let started = Instant::now();
    let funcs = pipeline::scan_with_excludes(project_root, &args.excludes, args.scope)?;
    let elapsed_secs = started.elapsed().as_secs_f64();

    let mut files: Vec<&str> = funcs.iter().map(|f| f.file.as_str()).collect();
    files.sort_unstable();
    files.dedup();
    let file_count = files.len();

    Ok(ScanResult {
        funcs,
        file_count,
        elapsed_secs,
    })
}

pub(crate) fn print_scan_summary(scan: &ScanResult) {
    println!(
        "Scanned {} files, {} functions in {:.2}s",
        scan.file_count,
        scan.funcs.len(),
        scan.elapsed_secs,
    );
}

pub(crate) fn collect_exceeded<'a>(
    funcs: &'a [Function],
    thresholds: &Thresholds,
) -> Vec<(&'a Function, Vec<MetricBreach>)> {
    let mut exceeded: Vec<(&Function, Vec<MetricBreach>)> = funcs
        .iter()
        .filter_map(|f| {
            let breaches = collect_breaches(&f.metrics, thresholds);
            (!breaches.is_empty()).then_some((f, breaches))
        })
        .collect();

    for (_, breaches) in &mut exceeded {
        breaches.sort_by(|a, b| b.overage.cmp(&a.overage));
    }
    exceeded.sort_by(|a, b| {
        let am = a.1.iter().map(|m| m.overage).max().unwrap_or(0);
        let bm = b.1.iter().map(|m| m.overage).max().unwrap_or(0);
        bm.cmp(&am)
    });
    exceeded
}

pub(crate) fn print_exceeded(funcs: &[Function]) {
    let exceeded = collect_exceeded(funcs, &DEFAULT_THRESHOLDS);
    if exceeded.is_empty() {
        println!("\u{2713} All functions within threshold.");
        return;
    }
    println!("{} functions exceed threshold:", exceeded.len());
    println!();
    for (f, breaches) in exceeded {
        println!("  {}:{}  {}", f.file, f.start_line, f.name);
        for b in breaches {
            println!("    {}: {} \u{26a0} (>{})", b.metric, b.value, b.threshold);
        }
    }
}

fn collect_breaches(m: &Metrics, t: &Thresholds) -> Vec<MetricBreach> {
    let mut out = Vec::with_capacity(4);
    if m.nesting > t.nesting {
        out.push(MetricBreach {
            metric: "nesting",
            value: m.nesting,
            threshold: t.nesting,
            overage: m.nesting - t.nesting,
        });
    }
    if m.cyclomatic > t.cyclomatic {
        out.push(MetricBreach {
            metric: "cyclomatic",
            value: m.cyclomatic,
            threshold: t.cyclomatic,
            overage: m.cyclomatic - t.cyclomatic,
        });
    }
    if m.cognitive > t.cognitive {
        out.push(MetricBreach {
            metric: "cognitive",
            value: m.cognitive,
            threshold: t.cognitive,
            overage: m.cognitive - t.cognitive,
        });
    }
    if m.params > t.params {
        out.push(MetricBreach {
            metric: "params",
            value: m.params,
            threshold: t.params,
            overage: m.params - t.params,
        });
    }
    if m.statement_count > t.statement_count {
        out.push(MetricBreach {
            metric: "statement_count",
            value: m.statement_count,
            threshold: t.statement_count,
            overage: m.statement_count - t.statement_count,
        });
    }
    if m.max_condition_ops > t.max_condition_ops {
        out.push(MetricBreach {
            metric: "max_condition_ops",
            value: m.max_condition_ops,
            threshold: t.max_condition_ops,
            overage: m.max_condition_ops - t.max_condition_ops,
        });
    }
    if m.mutable_bindings > t.mutable_bindings {
        out.push(MetricBreach {
            metric: "mutable_bindings",
            value: m.mutable_bindings,
            threshold: t.mutable_bindings,
            overage: m.mutable_bindings - t.mutable_bindings,
        });
    }
    out
}
