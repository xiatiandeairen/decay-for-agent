use crate::cli::common;
use crate::diff;
use crate::error::Result;
use crate::evidence_store::{EvidenceStore, SavedDiffEvidence};
use crate::impact;
use crate::metric::{self, ProblemGroupId};
use crate::store;
use crate::types::{DiffEntry, DiffKind, FunctionSet, MetricId};

/// Run `decay diff <from> [to]`.
///
/// One version compares baseline `<from>` to the current workspace.
/// Two versions compare baseline `<from>` to baseline `<to>`.
pub fn run(args: &common::ScanArgs, from: &str, to: Option<&str>) -> Result<i32> {
    let project = common::resolve_project()?;
    if to.is_none() {
        if let Some(git_ref) = resolve_git_ref(&project.root, from)? {
            let report = impact::analyze_worktree_diff(&project.root, &git_ref.commit)?;
            let recording = record_impact_report(&project, &git_ref.commit, &report);
            let context = GitDiffContext {
                repo_root: project.root.display().to_string(),
                base_input: from.to_string(),
                base_commit: git_ref.commit,
                head_commit: git_rev_parse(&project.root, "HEAD")?,
                changed_files: report.file_changes.len(),
                short_ref: git_ref.short_input,
            };
            print_impact_report(&context, &report, Some(&recording));
            return Ok(
                if report.evidences.iter().any(|e| {
                    matches!(
                        e.kind,
                        impact::EvidenceKind::Worsened | impact::EvidenceKind::NewHighRisk
                    )
                }) {
                    1
                } else {
                    0
                },
            );
        }
        if looks_like_git_ref(from) {
            println!(
                "warning=git_ref_not_found input={} message=\"trying stored baseline version\"",
                from
            );
        }
    }

    let conn = store::open_db()?;
    let from_baseline = match store::load_baseline(
        &conn,
        &project.project_id,
        &project.project_key,
        args.scope.as_str(),
        from,
    )? {
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
                &project.project_key,
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

#[derive(Debug, Clone)]
struct ResolvedGitRef {
    commit: String,
    short_input: bool,
}

fn resolve_git_ref(project_root: &std::path::Path, name: &str) -> Result<Option<ResolvedGitRef>> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--verify", &format!("{name}^{{commit}}")])
        .current_dir(project_root)
        .output()
        .map_err(|source| crate::error::DecayError::Io {
            path: "git".to_string(),
            source,
        })?;
    if !output.status.success() {
        return Ok(None);
    }
    Ok(Some(ResolvedGitRef {
        commit: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        short_input: name.len() < 40 && looks_like_git_ref(name),
    }))
}

fn looks_like_git_ref(name: &str) -> bool {
    (7..=40).contains(&name.len()) && name.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn record_impact_report(
    project: &common::ProjectContext,
    base_ref: &str,
    report: &impact::ImpactReport,
) -> std::result::Result<SavedDiffEvidence, String> {
    let conn = store::open_db().map_err(|err| err.to_string())?;
    let evidence_store = EvidenceStore::from_connection(conn).map_err(|err| err.to_string())?;
    let base_commit = git_rev_parse(&project.root, base_ref).map_err(|err| err.to_string())?;
    let head_commit = git_rev_parse(&project.root, "HEAD").map_err(|err| err.to_string())?;
    let evidence = impact::diff_evidence_records(report);
    evidence_store
        .save_diff_evidence(
            &project.project_key,
            base_ref,
            &base_commit,
            &head_commit,
            &report.file_changes,
            &evidence,
        )
        .map_err(|err| err.to_string())
}

fn git_rev_parse(project_root: &std::path::Path, name: &str) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--verify", name])
        .current_dir(project_root)
        .output()
        .map_err(|source| crate::error::DecayError::Io {
            path: "git".to_string(),
            source,
        })?;
    if !output.status.success() {
        return Err(crate::error::DecayError::InvalidProject(format!(
            "git rev-parse {name} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[derive(Debug, Clone)]
struct GitDiffContext {
    repo_root: String,
    base_input: String,
    base_commit: String,
    head_commit: String,
    changed_files: usize,
    short_ref: bool,
}

fn print_impact_report(
    context: &GitDiffContext,
    report: &impact::ImpactReport,
    recording: Option<&std::result::Result<SavedDiffEvidence, String>>,
) {
    println!("Diff context:");
    println!("  repo_root={}", context.repo_root);
    println!("  mode=git_ref");
    println!("  base_input={}", context.base_input);
    println!("  base={}", context.base_commit);
    println!("  head={}", context.head_commit);
    println!("  changed_files={}", context.changed_files);
    if context.short_ref {
        println!(
            "warning=short_git_ref_resolved input={} resolved={}",
            context.base_input, context.base_commit
        );
    }
    println!();
    print_risk_findings(report);
    println!();
    println!("Complexity impact:");
    println!(
        "  changed_functions={} evidence={} partial={} diagnostics={}",
        report.changed_functions,
        report.evidences.len(),
        !report.diagnostics.is_empty(),
        report.diagnostics.len()
    );
    for (idx, evidence) in report.evidences.iter().enumerate() {
        println!(
            "- id={} category=complexity kind={} subject=function source={}",
            idx + 1,
            evidence.kind.as_str(),
            evidence.source
        );
        println!("  function={}:{}", evidence.file, evidence.function);
        if let (Some(previous), Some(current)) = (evidence.previous, evidence.current) {
            println!(
                "  metrics=cognitive:{}->{}, cyclomatic:{}->{}, statements:{}->{}",
                previous.cognitive,
                current.cognitive,
                previous.cyclomatic,
                current.cyclomatic,
                previous.statement_count,
                current.statement_count
            );
        }
    }
    println!();
    println!("Stability impact:");
    println!("  evidence={}", report.stability_evidence.len());
    for (idx, evidence) in report.stability_evidence.iter().enumerate() {
        println!(
            "- id={} category=stability kind={} subject={} source={}",
            report.evidences.len() + idx + 1,
            evidence.kind,
            evidence.subject,
            evidence.source
        );
        println!("  file={}", evidence.file);
    }
    println!();
    println!("Quality impact:");
    println!(
        "  evidence={} prod_files_changed={} test_files_changed={} docs_files_changed={}",
        report.quality.records.len(),
        report.quality.prod_files_changed,
        report.quality.test_files_changed,
        report.quality.docs_files_changed
    );
    for (idx, evidence) in report.quality.records.iter().enumerate() {
        println!(
            "- id={} category=quality kind={} subject={} source={}",
            report.evidences.len() + report.stability_evidence.len() + idx + 1,
            evidence.kind,
            evidence.subject,
            evidence.source
        );
        println!("  file={}", evidence.file);
    }
    println!();
    println!("Semantic impact:");
    println!("  evidence={}", report.semantic_evidence.len());
    for (idx, evidence) in report.semantic_evidence.iter().enumerate() {
        println!(
            "- id={} category=semantic kind={} subject={} source={}",
            report.evidences.len()
                + report.stability_evidence.len()
                + report.quality.records.len()
                + idx
                + 1,
            evidence.kind,
            evidence.subject,
            evidence.source
        );
        println!("  file={}", evidence.file);
    }
    println!();
    println!("History impact:");
    println!("  evidence={}", report.history.records.len());
    for (idx, evidence) in report.history.records.iter().enumerate() {
        println!(
            "- id={} category=history kind={} subject={} source={}",
            report.evidences.len()
                + report.stability_evidence.len()
                + report.quality.records.len()
                + report.semantic_evidence.len()
                + idx
                + 1,
            evidence.kind,
            evidence.subject,
            evidence.source
        );
        println!("  file={}", evidence.file);
    }
    for diagnostic in &report.diagnostics {
        println!("diagnostic={diagnostic}");
    }
    match recording {
        Some(Ok(saved)) => println!(
            "recorded=true changeset={} evidence_saved={}",
            saved.changeset_id, saved.evidence_saved
        ),
        Some(Err(diagnostic)) => {
            println!("recorded=false evidence_saved=0");
            println!("diagnostic={diagnostic}");
        }
        None => {}
    }
}

#[derive(Debug, Clone)]
struct RiskFinding {
    severity: &'static str,
    title: String,
    detail: String,
    action: &'static str,
    score: u32,
}

fn print_risk_findings(report: &impact::ImpactReport) {
    let mut findings = risk_findings(report);
    println!("Top risk findings:");
    if findings.is_empty() {
        println!("  none");
        return;
    }
    findings.sort_by(|a, b| b.score.cmp(&a.score).then(a.title.cmp(&b.title)));
    for (idx, finding) in findings.iter().take(10).enumerate() {
        println!("{}. [{}] {}", idx + 1, finding.severity, finding.title);
        println!("   {}", finding.detail);
        println!("   action: {}", finding.action);
    }
    if findings.len() > 10 {
        println!("  truncated=true remaining={}", findings.len() - 10);
    }
}

fn risk_findings(report: &impact::ImpactReport) -> Vec<RiskFinding> {
    let mut findings = Vec::new();

    for evidence in &report.evidences {
        let score = match evidence.kind {
            impact::EvidenceKind::NewHighRisk => 90,
            impact::EvidenceKind::Worsened => 80,
            impact::EvidenceKind::TouchedExistingRisky => 55,
            impact::EvidenceKind::IdentityUncertain => 50,
            impact::EvidenceKind::Improved => 10,
        };
        if score < 50 {
            continue;
        }
        findings.push(RiskFinding {
            severity: if score >= 80 { "high" } else { "medium" },
            title: format!("{}:{}", evidence.file, evidence.function),
            detail: format!("complexity {}", evidence.kind.as_str()),
            action: "review function complexity and add focused regression tests",
            score,
        });
    }

    let mut stability_by_file = std::collections::BTreeMap::<String, Vec<&str>>::new();
    for evidence in &report.stability_evidence {
        stability_by_file
            .entry(evidence.file.clone())
            .or_default()
            .push(evidence.kind.as_str());
    }
    for (file, kinds) in stability_by_file {
        let unsafe_count = kinds.iter().filter(|kind| **kind == "unsafe_added").count();
        let panic_count = kinds
            .iter()
            .filter(|kind| **kind == "panic_like_added")
            .count();
        let ffi_count = kinds
            .iter()
            .filter(|kind| **kind == "ffi_boundary_touched")
            .count();
        let public_api_count = kinds
            .iter()
            .filter(|kind| **kind == "public_api_signature_changed")
            .count();
        let public_api_added_count = kinds
            .iter()
            .filter(|kind| **kind == "public_api_added")
            .count();
        let migration_count = kinds
            .iter()
            .filter(|kind| **kind == "storage_migration_touched")
            .count();
        let protocol_count = kinds
            .iter()
            .filter(|kind| **kind == "protocol_shape_changed")
            .count();
        let score = if migration_count > 0 {
            87
        } else if unsafe_count > 0 || panic_count > 0 {
            85
        } else if protocol_count > 0 || public_api_count > 0 || public_api_added_count > 0 {
            83
        } else if ffi_count > 0 {
            70
        } else {
            60
        };
        let mut parts = Vec::new();
        if unsafe_count > 0 {
            parts.push(format!("{unsafe_count} unsafe additions"));
        }
        if panic_count > 0 {
            parts.push(format!("{panic_count} panic-like additions"));
        }
        if ffi_count > 0 {
            parts.push("FFI boundary touched".to_string());
        }
        if public_api_count > 0 {
            parts.push(format!("{public_api_count} public API signature changed"));
        }
        if public_api_added_count > 0 {
            parts.push(format!("{public_api_added_count} public API added"));
        }
        if migration_count > 0 {
            parts.push(format!("{migration_count} storage migration touched"));
        }
        if protocol_count > 0 {
            parts.push(format!("{protocol_count} protocol shape changed"));
        }
        findings.push(RiskFinding {
            severity: if score >= 80 { "high" } else { "medium" },
            title: file,
            detail: parts.join(" + "),
            action: "review runtime failure and boundary-safety invariants",
            score,
        });
    }

    let dependency_count = report
        .quality
        .records
        .iter()
        .filter(|evidence| evidence.kind == "dependency_changed")
        .count();
    if dependency_count > 0 {
        findings.push(RiskFinding {
            severity: "medium",
            title: "dependency manifests changed".to_string(),
            detail: format!("{dependency_count} dependency files changed"),
            action: "review dependency and feature changes for behavior/security impact",
            score: 45,
        });
    }
    for evidence in &report.quality.records {
        let (score, action) = match evidence.kind.as_str() {
            "sensitive_test_gap" => (88, "add or justify missing tests for sensitive code"),
            "test_gap" => (
                65,
                "add focused tests or document why existing coverage is enough",
            ),
            "blast_radius" | "mixed_concerns" => {
                (60, "split the diff or document the shared review rationale")
            }
            _ => continue,
        };
        findings.push(RiskFinding {
            severity: if score >= 80 { "high" } else { "medium" },
            title: evidence.file.clone(),
            detail: format!("quality {}", evidence.kind),
            action,
            score,
        });
    }

    for evidence in &report.semantic_evidence {
        let (severity, score, action, detail) = match evidence.kind.as_str() {
            "protocol_header_authority_changed" => (
                "high",
                92,
                "review protocol header semantics, credential leakage, and compatibility tests",
                format!("protocol host header authority changed {}", evidence.subject),
            ),
            "security_sensitive_dependency_changed" => (
                "high",
                86,
                "review dependency feature semantics and require targeted compatibility/security checks",
                format!("semantic {} {}", evidence.kind, evidence.subject),
            ),
            "config_value_changed" => (
                "high",
                84,
                "review production configuration boundary change and add rollout/regression checks",
                format!("semantic {} {}", evidence.kind, evidence.subject),
            ),
            "env_default_changed" => (
                "high",
                84,
                "review runtime environment default change and add compatibility checks",
                format!("semantic {} {}", evidence.kind, evidence.subject),
            ),
            "default_value_changed" => (
                "high",
                82,
                "review behavior boundary change and add regression coverage for old and new defaults",
                format!("semantic {} {}", evidence.kind, evidence.subject),
            ),
            _ => (
                "medium",
                60,
                "review semantic behavior impact before merge",
                format!("semantic {} {}", evidence.kind, evidence.subject),
            ),
        };
        findings.push(RiskFinding {
            severity,
            title: evidence.file.clone(),
            detail,
            action,
            score,
        });
    }

    findings
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
            DiffKind::AddedNew | DiffKind::AddedMaybeMoved => println!("  evidence={}", changes),
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
        (DiffKind::AddedNew | DiffKind::AddedMaybeMoved, _) => {
            current_metric_values(entry.function.metrics)
                .into_iter()
                .filter(|m| metric::breaches_threshold(m.value, m.threshold))
                .map(|m| ChangeEvidence {
                    metric: m.metric,
                    previous: None,
                    value: m.value,
                    threshold: m.threshold,
                })
                .collect()
        }
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
            kind: DiffKind::AddedNew,
            name: "new high-risk functions",
            what_changed: "These functions did not exist in the baseline and are already above recommended complexity limits.",
            why: "New code entering the project above the risk boundary is likely to become expensive to modify quickly.",
            problem_groups: problem_groups(),
        },
        ChangeGroup {
            kind: DiffKind::AddedMaybeMoved,
            name: "unmatched high-risk functions",
            what_changed: "These functions were not matched to the baseline and are already above recommended complexity limits.",
            why: "This is a weaker signal: the function may be new, moved, renamed, or otherwise changed identity during refactoring.",
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
    if kind == DiffKind::AddedNew {
        return "New function is already hard to maintain safely.";
    }
    if kind == DiffKind::AddedMaybeMoved {
        return "Unmatched function may be new or changed identity and is hard to maintain safely.";
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
