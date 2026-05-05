use crate::cli::common;
use crate::diff;
use crate::error::Result;
use crate::metric::{self, ProblemGroupId};
use crate::store;
use crate::types::{DiffEntry, DiffKind, FunctionSet, MetricId};

/// Run `decay diff <from> [to]`.
///
/// One version compares baseline `<from>` to the current workspace.
/// Two versions compare baseline `<from>` to baseline `<to>`.
pub fn run(args: &common::ScanArgs, from: &str, to: Option<&str>) -> Result<i32> {
    let project = common::resolve_project()?;
    let conn = store::open_db()?;
    let from_baseline =
        match store::load_baseline(&conn, &project.project_id, args.scope.as_str(), from)? {
            Some(b) => b,
            None => {
                print_missing(args, from);
                return Ok(2);
            }
        };

    let from_partial = from_baseline.is_partial;
    let from_diagnostics = from_baseline.diagnostic_count;

    let (to_label, current_scan, to_partial, to_diagnostics, to_set) = match to {
        Some(version) => {
            let baseline = match store::load_baseline(
                &conn,
                &project.project_id,
                args.scope.as_str(),
                version,
            )? {
                Some(b) => b,
                None => {
                    print_missing(args, version);
                    return Ok(2);
                }
            };
            (
                version.to_string(),
                None,
                baseline.is_partial,
                baseline.diagnostic_count,
                FunctionSet {
                    functions: baseline.functions,
                },
            )
        }
        None => {
            let scan = common::scan_current(&project.root, args)?;
            let set = FunctionSet {
                functions: scan.funcs.clone(),
            };
            ("current".to_string(), Some(scan), false, 0, set)
        }
    };

    let from_set = FunctionSet {
        functions: from_baseline.functions,
    };
    let entries = diff::diff(&from_set, &to_set);
    let report = build_report(&entries);
    let print_ctx = DiffPrintContext {
        from,
        to: &to_label,
        scan: current_scan.as_ref(),
        from_partial,
        from_diagnostics,
        to_partial,
        to_diagnostics,
    };

    if args.verbose {
        print_verbose(args, &print_ctx, &report);
    } else {
        print_concise(&print_ctx, &report);
    }

    Ok(if entries.is_empty() { 0 } else { 1 })
}

fn print_missing(args: &common::ScanArgs, version: &str) {
    println!(
        "status=error reason=baseline_not_found version={} scope={}",
        version,
        args.scope.as_str()
    );
}

struct DiffPrintContext<'a> {
    from: &'a str,
    to: &'a str,
    scan: Option<&'a common::ScanResult>,
    from_partial: bool,
    from_diagnostics: u32,
    to_partial: bool,
    to_diagnostics: u32,
}

fn print_concise(ctx: &DiffPrintContext<'_>, report: &[ChangeGroup]) {
    let count = total_records(report);
    if count == 0 {
        println!("status=ok from={} to={} degradations=0", ctx.from, ctx.to);
        print_partial_warnings(ctx);
        return;
    }

    println!(
        "status=degraded from={} to={} degradations={}",
        ctx.from, ctx.to, count
    );
    print_partial_warnings(ctx);
    for change_group in report {
        println!();
        println!("[{}]", change_group.name);
        for problem_group in &change_group.problem_groups {
            for record in &problem_group.records {
                print_record(record, false);
            }
        }
    }
}

fn print_verbose(args: &common::ScanArgs, ctx: &DiffPrintContext<'_>, report: &[ChangeGroup]) {
    println!("decay v{}", env!("CARGO_PKG_VERSION"));
    println!("Mode: diff");
    println!("Scope: {}", args.scope.as_str());
    println!("From: baseline {}", ctx.from);
    if ctx.to == "current" {
        println!("To: current workspace");
    } else {
        println!("To: baseline {}", ctx.to);
    }
    if let Some(scan) = ctx.scan {
        println!(
            "Scanned: {} files, {} functions in {:.2}s",
            scan.file_count,
            scan.funcs.len(),
            scan.elapsed_secs
        );
    }
    print_partial_warnings(ctx);
    println!();

    let count = total_records(report);
    if count == 0 {
        println!("Result:");
        println!("  No maintainability regressions found.");
        return;
    }

    println!("Result:");
    println!("  {} maintainability regressions found.", count);

    for change_group in report {
        println!();
        println!("[{}]", change_group.name);
        println!();
        println!("What changed:");
        println!("  {}", change_group.what_changed);
        println!();
        println!("Why it matters:");
        println!("  {}", change_group.why);
        println!();
        println!("Records:");
        for problem_group in &change_group.problem_groups {
            println!("  [{}]", problem_group.name);
            for record in &problem_group.records {
                print_record(record, true);
            }
        }
    }
}

fn print_partial_warnings(ctx: &DiffPrintContext<'_>) {
    if ctx.from_partial {
        println!(
            "warning=from_baseline_partial diagnostics={}",
            ctx.from_diagnostics
        );
    }
    if ctx.to_partial {
        println!(
            "warning=to_baseline_partial diagnostics={}",
            ctx.to_diagnostics
        );
    }
    if let Some(scan) = ctx.scan {
        if scan.diagnostic_count > 0 {
            println!(
                "warning=current_scan_partial diagnostics={}",
                scan.diagnostic_count
            );
        }
    }
}

fn print_record(record: &DiffRecord, verbose: bool) {
    let f = &record.entry.function;
    if verbose {
        println!("  - {}:{} {}", f.file, f.start_line, f.name);
        println!("    Problem:");
        println!("      {}", record.problem);
        if !record.changes.is_empty() {
            println!("    Change:");
            for change in &record.changes {
                println!("      - {}", change.change_sentence());
            }
        }
        println!("    Bad points:");
        for change in &record.changes {
            println!("      - {}", change.evidence_sentence());
        }
    } else {
        println!("- {}:{} {}", f.file, f.start_line, f.name);
        println!("  problem={}", record.problem);
        let changes = record
            .changes
            .iter()
            .map(ChangeEvidence::concise_sentence)
            .collect::<Vec<_>>()
            .join("; ");
        match record.entry.kind {
            DiffKind::Added => println!("  evidence={}", changes),
            _ => println!("  change={}", changes),
        }
    }
}

fn build_report(entries: &[DiffEntry]) -> Vec<ChangeGroup> {
    let mut groups = change_groups();
    for entry in entries {
        let changes = changes_for_entry(entry);
        for change_group in &mut groups {
            if change_group.kind != entry.kind {
                continue;
            }
            for problem_group in &mut change_group.problem_groups {
                let matched: Vec<ChangeEvidence> = changes
                    .iter()
                    .filter(|c| problem_group.group == metric::def(c.metric).group)
                    .cloned()
                    .collect();
                if !matched.is_empty() {
                    problem_group.records.push(DiffRecord {
                        entry: entry.clone(),
                        problem: problem_for(&matched, entry.kind),
                        changes: matched,
                    });
                }
            }
        }
    }

    groups
        .into_iter()
        .filter_map(|mut group| {
            group
                .problem_groups
                .retain(|problem_group| !problem_group.records.is_empty());
            (!group.problem_groups.is_empty()).then_some(group)
        })
        .collect()
}

fn changes_for_entry(entry: &DiffEntry) -> Vec<ChangeEvidence> {
    match (entry.kind, entry.previous) {
        (DiffKind::Added, _) => current_metric_values(entry.function.metrics)
            .into_iter()
            .filter(|m| metric::breaches_threshold(m.value, m.threshold))
            .map(|m| ChangeEvidence {
                metric: m.metric,
                previous: None,
                value: m.value,
                threshold: m.threshold,
            })
            .collect(),
        (_, Some(prev)) => {
            let curr = entry.function.metrics;
            metric_pairs(prev, curr)
                .into_iter()
                .filter(|m| {
                    let p = m.previous.unwrap_or(0);
                    metric::crossed_threshold(p, m.value, m.threshold)
                        || metric::worsened_over_threshold(p, m.value, m.threshold)
                })
                .collect()
        }
        (_, None) => Vec::new(),
    }
}

fn current_metric_values(m: crate::types::Metrics) -> Vec<ChangeEvidence> {
    metric::active_values(m)
        .map(|(def, value)| ChangeEvidence::current(def.id, value, def.threshold))
        .collect()
}

fn metric_pairs(prev: crate::types::Metrics, curr: crate::types::Metrics) -> Vec<ChangeEvidence> {
    metric::ACTIVE_METRICS
        .iter()
        .map(|def| {
            ChangeEvidence::changed(
                def.id,
                prev.value(def.id),
                curr.value(def.id),
                def.threshold,
            )
        })
        .collect()
}

fn total_records(groups: &[ChangeGroup]) -> usize {
    groups
        .iter()
        .flat_map(|g| &g.problem_groups)
        .map(|g| g.records.len())
        .sum()
}

#[derive(Clone)]
struct ChangeEvidence {
    metric: MetricId,
    previous: Option<u32>,
    value: u32,
    threshold: u32,
}

impl ChangeEvidence {
    fn current(metric: MetricId, value: u32, threshold: u32) -> Self {
        Self {
            metric,
            previous: None,
            value,
            threshold,
        }
    }

    fn changed(metric: MetricId, previous: u32, value: u32, threshold: u32) -> Self {
        Self {
            metric,
            previous: Some(previous),
            value,
            threshold,
        }
    }

    fn concise_sentence(&self) -> String {
        match self.previous {
            Some(previous) => format!(
                "{} changed from {} to {}; recommended limit is {}.",
                metric::def(self.metric).measure_name,
                format_metric(self.metric, previous),
                format_metric(self.metric, self.value),
                format_metric(self.metric, self.threshold)
            ),
            None => format!(
                "{} is {}; recommended limit is {}.",
                metric::def(self.metric).measure_name,
                format_metric(self.metric, self.value),
                format_metric(self.metric, self.threshold)
            ),
        }
    }

    fn change_sentence(&self) -> String {
        match self.previous {
            Some(previous) => format!(
                "{} changed from {} to {}.",
                metric::def(self.metric).measure_name,
                format_metric(self.metric, previous),
                format_metric(self.metric, self.value)
            ),
            None => "New function.".to_string(),
        }
    }

    fn evidence_sentence(&self) -> String {
        format!(
            "{} recommended limit is {}; current value is {}.",
            metric::def(self.metric).measure_name,
            format_metric(self.metric, self.threshold),
            format_metric(self.metric, self.value)
        )
    }
}

#[derive(Clone)]
struct DiffRecord {
    entry: DiffEntry,
    problem: &'static str,
    changes: Vec<ChangeEvidence>,
}

struct ProblemGroup {
    name: &'static str,
    group: ProblemGroupId,
    records: Vec<DiffRecord>,
}

struct ChangeGroup {
    kind: DiffKind,
    name: &'static str,
    what_changed: &'static str,
    why: &'static str,
    problem_groups: Vec<ProblemGroup>,
}

fn change_groups() -> Vec<ChangeGroup> {
    vec![
        ChangeGroup {
            kind: DiffKind::Added,
            name: "new high-risk functions",
            what_changed: "These functions did not exist in the baseline and are already above recommended complexity limits.",
            why: "New code entering the project above the risk boundary is likely to become expensive to modify quickly.",
            problem_groups: problem_groups(),
        },
        ChangeGroup {
            kind: DiffKind::CrossedThreshold,
            name: "functions that crossed a risk boundary",
            what_changed: "These functions existed in the baseline, but moved from acceptable complexity into risky territory.",
            why: "This is a clear regression signal: code that was previously within limits now needs attention.",
            problem_groups: problem_groups(),
        },
        ChangeGroup {
            kind: DiffKind::Worsened,
            name: "risks that got worse",
            what_changed: "These functions were already above recommended limits, and became worse.",
            why: "Existing complexity debt increased, which raises the cost and risk of future edits.",
            problem_groups: problem_groups(),
        },
    ]
}

fn problem_groups() -> Vec<ProblemGroup> {
    vec![
        ProblemGroup {
            name: "hard-to-follow logic",
            group: ProblemGroupId::HardToFollowLogic,
            records: Vec::new(),
        },
        ProblemGroup {
            name: "large function body",
            group: ProblemGroupId::LargeFunctionBody,
            records: Vec::new(),
        },
        ProblemGroup {
            name: "wide interface",
            group: ProblemGroupId::WideInterface,
            records: Vec::new(),
        },
        ProblemGroup {
            name: "compound conditions",
            group: ProblemGroupId::CompoundConditions,
            records: Vec::new(),
        },
    ]
}

fn problem_for(changes: &[ChangeEvidence], kind: DiffKind) -> &'static str {
    if kind == DiffKind::Added {
        return "New function is already hard to maintain safely.";
    }
    if changes.iter().any(|c| c.metric == MetricId::StatementCount) {
        "Function body grew beyond a focused size."
    } else if changes.iter().any(|c| c.metric == MetricId::Params) {
        "Function interface grew too wide."
    } else if changes
        .iter()
        .any(|c| c.metric == MetricId::MaxConditionOps)
    {
        "Condition logic became too dense."
    } else if kind == DiffKind::Worsened {
        "Already complex logic became more difficult to change safely."
    } else {
        "Control flow moved into risky complexity."
    }
}

fn format_metric(metric: MetricId, value: u32) -> String {
    (metric::def(metric).format)(value)
}
