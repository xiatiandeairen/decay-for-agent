use crate::cli::common::{self, MetricBreach};
use crate::error::Result;
use crate::metric::{self, ProblemGroupId};
use crate::types::{Function, MetricId};

/// Run `decay doctor`: diagnose current tree without reading baselines.
pub fn run(args: &common::ScanArgs) -> Result<i32> {
    let project = common::resolve_project()?;
    let scan = common::scan_current(&project.root, args)?;
    let findings = build_findings(&scan.funcs);

    if args.verbose {
        print_verbose(args, &scan, &findings);
    } else {
        print_concise(args, &scan, &findings);
    }
    Ok(0)
}

pub(crate) fn build_findings(funcs: &[Function]) -> Vec<GroupFindings> {
    let exceeded = common::collect_exceeded(funcs);
    let mut groups = problem_groups();
    for (func, breaches) in exceeded {
        for group in &mut groups {
            let matched: Vec<MetricBreach> = breaches
                .iter()
                .filter(|b| group.group == metric::def(b.metric).group)
                .cloned()
                .collect();
            if !matched.is_empty() {
                group.records.push(FindingRecord {
                    function: func.clone(),
                    breaches: matched,
                });
            }
        }
    }
    groups
        .into_iter()
        .filter(|g| !g.records.is_empty())
        .collect()
}

fn print_concise(args: &common::ScanArgs, scan: &common::ScanResult, groups: &[GroupFindings]) {
    let finding_count = total_records(groups);
    if finding_count == 0 {
        println!("status=ok findings=0 scope={}", args.scope.as_str());
        if scan.diagnostic_count > 0 {
            println!("warning=scan_partial diagnostics={}", scan.diagnostic_count);
        }
        println!("summary=No complexity risks found in the scanned Rust functions.");
        return;
    }

    println!(
        "status=attention findings={} groups={} scope={}",
        finding_count,
        groups.len(),
        args.scope.as_str()
    );
    if scan.diagnostic_count > 0 {
        println!("warning=scan_partial diagnostics={}", scan.diagnostic_count);
    }
    for group in groups {
        println!();
        println!("[{}]", group.name);
        println!("summary={}", group.summary);
        println!("count={}", group.records.len());
        for record in &group.records {
            print_concise_record(record);
        }
    }
}

fn print_verbose(args: &common::ScanArgs, scan: &common::ScanResult, groups: &[GroupFindings]) {
    println!("decay v{}", env!("CARGO_PKG_VERSION"));
    println!("Mode: doctor");
    println!("Scope: {}", args.scope.as_str());
    println!(
        "Scanned: {} files, {} functions in {:.2}s",
        scan.file_count,
        scan.funcs.len(),
        scan.elapsed_secs
    );
    println!("Diagnostics: {}", scan.diagnostic_count);
    println!();

    let finding_count = total_records(groups);
    if finding_count == 0 {
        println!("Result:");
        println!("  No complexity risks found in the scanned Rust functions.");
        return;
    }

    println!("Result:");
    println!(
        "  {} findings across {} maintainability risk groups.",
        finding_count,
        groups.len()
    );

    for group in groups {
        println!();
        println!("[{}]", group.name);
        println!();
        println!("What this means:");
        println!("  {}", group.what);
        println!();
        println!("Why it matters:");
        println!("  {}", group.why);
        println!();
        println!("What to look for:");
        for item in group.look_for {
            println!("  - {}", item);
        }
        println!();
        println!("Records:");
        for record in &group.records {
            print_verbose_record(record);
        }
    }
}

pub(crate) fn print_concise_record(record: &FindingRecord) {
    let f = &record.function;
    println!("- {}:{} {}", f.file, f.start_line, f.name);
    println!("  problem={}", problem_for(&record.breaches));
    println!("  evidence={}", evidence_for(&record.breaches));
}

fn print_verbose_record(record: &FindingRecord) {
    let f = &record.function;
    println!("- {}:{} {}", f.file, f.start_line, f.name);
    println!("  Problem:");
    println!("    {}", problem_for(&record.breaches));
    println!("  Bad points:");
    for breach in &record.breaches {
        println!("    - {}", evidence_sentence(breach));
    }
    println!("  Reading:");
    println!("    {}", reading_for(&record.breaches));
    println!();
}

fn total_records(groups: &[GroupFindings]) -> usize {
    groups.iter().map(|g| g.records.len()).sum()
}

#[derive(Clone)]
pub(crate) struct FindingRecord {
    pub function: Function,
    pub breaches: Vec<MetricBreach>,
}

pub(crate) struct GroupFindings {
    pub name: &'static str,
    pub summary: &'static str,
    what: &'static str,
    why: &'static str,
    look_for: &'static [&'static str],
    group: ProblemGroupId,
    pub records: Vec<FindingRecord>,
}

fn problem_groups() -> Vec<GroupFindings> {
    vec![
        GroupFindings {
            name: "hard-to-follow logic",
            summary: "Some functions have control flow that is hard to review or safely change.",
            what: "These functions have too many branches, deeply nested control flow, or too many decision paths.",
            why: "Small changes can affect many execution paths, so review and testing become harder.",
            look_for: &[
                "Long if/else chains",
                "Deeply nested conditionals",
                "Large match arms with mixed responsibilities",
                "Boolean logic controlling several behaviors at once",
            ],
            group: ProblemGroupId::HardToFollowLogic,
            records: Vec::new(),
        },
        GroupFindings {
            name: "large function body",
            summary: "Some functions are doing too much work in one place.",
            what: "These functions contain too many executable steps.",
            why: "Large functions usually mix responsibilities and are harder to test precisely.",
            look_for: &[
                "Multiple phases inside one function",
                "Repeated setup/check/transform blocks",
                "Local variables that only belong to part of the function",
                "Comments separating sections that could become helper functions",
            ],
            group: ProblemGroupId::LargeFunctionBody,
            records: Vec::new(),
        },
        GroupFindings {
            name: "wide interface",
            summary: "Some functions require too much input context from callers.",
            what: "These functions take more input values than a caller should usually need to provide.",
            why: "Wide interfaces make call sites noisy and often indicate mixed responsibilities.",
            look_for: &[
                "Parameters that always travel together",
                "Boolean flags that change behavior",
                "Values that could be derived inside the function",
                "One function serving several use cases",
            ],
            group: ProblemGroupId::WideInterface,
            records: Vec::new(),
        },
        GroupFindings {
            name: "compound conditions",
            summary: "Some conditions have too many boolean parts to read safely.",
            what: "These functions contain condition expressions with too many combined boolean operations.",
            why: "Dense boolean expressions hide edge cases and make truth-table reasoning expensive.",
            look_for: &[
                "Long chains of && or ||",
                "Mixed positive and negative checks",
                "Conditions that encode several policies at once",
            ],
            group: ProblemGroupId::CompoundConditions,
            records: Vec::new(),
        },
    ]
}

fn problem_for(breaches: &[MetricBreach]) -> &'static str {
    if breaches
        .iter()
        .any(|b| b.metric == MetricId::StatementCount)
    {
        "The function body is larger than a focused operation should be."
    } else if breaches.iter().any(|b| b.metric == MetricId::Params) {
        "The function requires too much input context from callers."
    } else if breaches
        .iter()
        .any(|b| b.metric == MetricId::MaxConditionOps)
    {
        "The function has a condition with too many boolean parts."
    } else if breaches.iter().any(|b| b.metric == MetricId::Nesting) {
        "The function is nested too deeply."
    } else {
        "The function has too many decision paths."
    }
}

pub(crate) fn evidence_for(breaches: &[MetricBreach]) -> String {
    breaches
        .iter()
        .map(evidence_sentence)
        .collect::<Vec<_>>()
        .join("; ")
}

pub(crate) fn evidence_sentence(b: &MetricBreach) -> String {
    match b.metric {
        MetricId::Nesting => format!(
            "Nested control flow reaches depth {}; recommended limit is {}.",
            b.value, b.threshold
        ),
        MetricId::Cyclomatic => format!(
            "Branch count is {}; recommended limit is {}.",
            b.value, b.threshold
        ),
        MetricId::Cognitive => format!(
            "Branching complexity is {}; recommended limit is {}.",
            b.value, b.threshold
        ),
        MetricId::Params => format!(
            "Function takes {} parameters; recommended limit is {}.",
            b.value, b.threshold
        ),
        MetricId::StatementCount => format!(
            "Function contains {} statements; recommended limit is {}.",
            b.value, b.threshold
        ),
        MetricId::MaxConditionOps => format!(
            "A condition uses {} boolean operators; recommended limit is {}.",
            b.value, b.threshold
        ),
    }
}

fn reading_for(breaches: &[MetricBreach]) -> &'static str {
    if breaches
        .iter()
        .any(|b| b.metric == MetricId::StatementCount)
    {
        "The function appears to combine more steps than one focused operation should carry."
    } else if breaches.iter().any(|b| b.metric == MetricId::Params) {
        "The call contract is carrying more context than one focused operation usually needs."
    } else if breaches
        .iter()
        .any(|b| b.metric == MetricId::MaxConditionOps)
    {
        "The condition asks readers to evaluate too many boolean parts at once."
    } else {
        "The main path is hard to identify because control flow has too many branches or nested levels."
    }
}
