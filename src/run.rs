use std::env;

use anyhow::Result;
use log::debug;
use serde::Serialize;

use crate::{db, diagnose, git, scan, score, trend};

#[derive(Serialize)]
pub struct Report {
    pub snapshot_id: i64,
    pub scores: Scores,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trend: Option<trend::Trend>,
    pub issues: Vec<diagnose::Issue>,
    pub scan: scan::ScanSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<git::GitSummary>,
}

#[derive(Serialize)]
pub struct Scores {
    pub structural: i32,
    pub complexity: i32,
    pub fragility: Option<i32>,
    pub composite: i32,
}

pub const HEALTH_REPORT_TEMPLATE: &str = include_str!("../templates/health-report.md");

pub struct MarkdownCtx<'a> {
    pub snapshot_id: i64,
    pub project_path: &'a str,
    pub s: i32,
    pub c: i32,
    pub f: Option<i32>,
    pub comp: i32,
    pub trend_data: &'a Option<trend::Trend>,
    pub scan_summary: &'a scan::ScanSummary,
    pub git_summary: &'a Option<git::GitSummary>,
    pub issues: &'a [diagnose::Issue],
}

pub fn render_markdown(ctx: &MarkdownCtx<'_>) -> String {
    let MarkdownCtx {
        snapshot_id,
        project_path,
        s,
        c,
        f,
        comp,
        trend_data,
        scan_summary,
        git_summary,
        issues,
    } = ctx;
    let f_str = f.map_or("N/A".to_string(), |v| v.to_string());

    let (s_trend, c_trend, f_trend, comp_trend) = match trend_data {
        Some(t) => (
            format!("{}", t.structural),
            format!("{}", t.complexity),
            format!("{}", t.fragility),
            format!("{}", t.composite),
        ),
        None => (
            "—".to_string(),
            "—".to_string(),
            "—".to_string(),
            "—".to_string(),
        ),
    };

    let (total_commits, files_analyzed) = match git_summary {
        Some(g) => (g.total_commits.to_string(), g.files_analyzed.to_string()),
        None => ("N/A".to_string(), "N/A".to_string()),
    };

    let critical_count = issues
        .iter()
        .filter(|i| i.level == diagnose::Level::Critical)
        .count();
    let warning_count = issues
        .iter()
        .filter(|i| i.level == diagnose::Level::Warning)
        .count();
    let info_count = issues
        .iter()
        .filter(|i| i.level == diagnose::Level::Info)
        .count();

    let issue_summary = if issues.is_empty() {
        "none".to_string()
    } else {
        let mut parts = Vec::new();
        if critical_count > 0 {
            parts.push(format!("{critical_count} critical"));
        }
        if warning_count > 0 {
            parts.push(format!("{warning_count} warning"));
        }
        if info_count > 0 {
            parts.push(format!("{info_count} info"));
        }
        parts.join(", ")
    };

    let issues_section = if issues.is_empty() {
        "No issues found.".to_string()
    } else {
        let mut sections = Vec::new();
        for (level, label) in [
            (diagnose::Level::Critical, "Critical"),
            (diagnose::Level::Warning, "Warning"),
            (diagnose::Level::Info, "Info"),
        ] {
            let level_issues: Vec<&diagnose::Issue> =
                issues.iter().filter(|i| i.level == level).collect();
            if !level_issues.is_empty() {
                sections.push(format!("### {label}\n"));
                for issue in level_issues {
                    let line = match &issue.prescription {
                        Some(rx) => {
                            format!("- **{}**: {} — *{}*", issue.category, issue.message, rx)
                        }
                        None => format!("- **{}**: {}", issue.category, issue.message),
                    };
                    sections.push(line);
                }
                sections.push(String::new());
            }
        }
        sections.join("\n")
    };

    let project_name = std::path::Path::new(project_path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();

    let timestamp = chrono_now();

    HEALTH_REPORT_TEMPLATE
        .replace("{{project_name}}", &project_name)
        .replace("{{version}}", env!("CARGO_PKG_VERSION"))
        .replace("{{timestamp}}", &timestamp)
        .replace("{{snapshot_id}}", &snapshot_id.to_string())
        .replace("{{structural}}", &s.to_string())
        .replace("{{complexity}}", &c.to_string())
        .replace("{{fragility}}", &f_str)
        .replace("{{composite}}", &comp.to_string())
        .replace("{{structural_trend}}", &s_trend)
        .replace("{{complexity_trend}}", &c_trend)
        .replace("{{fragility_trend}}", &f_trend)
        .replace("{{composite_trend}}", &comp_trend)
        .replace("{{file_count}}", &scan_summary.file_count.to_string())
        .replace("{{dir_count}}", &scan_summary.dir_count.to_string())
        .replace("{{max_depth}}", &scan_summary.max_depth.to_string())
        .replace("{{total_commits}}", &total_commits)
        .replace("{{files_analyzed}}", &files_analyzed)
        .replace("{{issue_summary}}", &issue_summary)
        .replace("{{issues_section}}", &issues_section)
}

fn is_leap(y: u64) -> bool {
    y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400))
}

fn chrono_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    // Simple UTC date-time formatting
    let secs_per_day: u64 = 86400;
    let days = now / secs_per_day;
    let remaining = now % secs_per_day;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;

    let mut y: u64 = 1970;
    let mut d = days;
    loop {
        let diy = if is_leap(y) { 366 } else { 365 };
        if d < diy {
            break;
        }
        d -= diy;
        y += 1;
    }
    let leap = is_leap(y);
    let mdays: &[u64] = if leap {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m: u64 = 1;
    for &md in mdays {
        if d < md {
            break;
        }
        d -= md;
        m += 1;
    }
    format!("{y:04}-{m:02}-{:02} {hours:02}:{minutes:02} UTC", d + 1)
}

/// Run the core decay logic.
///
/// Returns `Ok(true)` if critical issues exist, `Ok(false)` otherwise.
/// Does not call `process::exit` — caller is responsible for exit codes.
pub fn run(json: bool, markdown: bool, quiet: bool) -> Result<bool> {
    debug!("decay starting");

    let conn = db::init()?;
    let project_path = env::current_dir()?;
    let project_path_str = project_path.to_string_lossy().to_string();
    let snapshot_id = db::create_snapshot(&conn, &project_path_str)?;
    debug!("snapshot {snapshot_id} created for {project_path_str}");

    let scan_summary = scan::collect(&conn, snapshot_id, &project_path)?;
    debug!(
        "scan complete: {} files, {} dirs",
        scan_summary.file_count, scan_summary.dir_count
    );

    let git_summary = match git::collect(&conn, snapshot_id, &project_path, 90) {
        Ok(summary) => {
            debug!(
                "git analysis complete: {} commits, {} files",
                summary.total_commits, summary.files_analyzed
            );
            Some(summary)
        }
        Err(e) => {
            debug!("git analysis skipped: {e}");
            if !json && !markdown && !quiet {
                eprintln!("Git analysis skipped: {e}");
            }
            None
        }
    };

    let s = score::structural(&conn, snapshot_id)?;
    let c = score::complexity(&conn, snapshot_id)?;
    let f = if git_summary.is_some() {
        score::fragility(&conn, snapshot_id)?
    } else {
        None
    };
    let comp = score::composite(s, c, f);
    debug!("scores: structural={s} complexity={c} fragility={f:?} composite={comp}");

    db::insert_scores(&conn, snapshot_id, s, c, f, comp)?;

    let trend_data = db::get_previous_scores(&conn, &project_path_str, snapshot_id)?
        .map(|prev| trend::Trend::compare(s, c, f, comp, &prev));

    let issues = diagnose::run(&conn, snapshot_id)?;
    debug!("diagnosis complete: {} issues", issues.len());

    let critical_count = issues
        .iter()
        .filter(|i| i.level == diagnose::Level::Critical)
        .count();

    if json {
        let report = Report {
            snapshot_id,
            scores: Scores {
                structural: s,
                complexity: c,
                fragility: f,
                composite: comp,
            },
            trend: trend_data,
            issues,
            scan: scan_summary,
            git: git_summary,
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else if markdown {
        let md = render_markdown(&MarkdownCtx {
            snapshot_id,
            project_path: &project_path_str,
            s,
            c,
            f,
            comp,
            trend_data: &trend_data,
            scan_summary: &scan_summary,
            git_summary: &git_summary,
            issues: &issues,
        });
        println!("{md}");
    } else if quiet {
        println!("Health: {comp}/100 ({critical_count} critical)");
    } else {
        // Default terminal output
        println!(
            "Scanned: {} files, {} dirs, max depth {}",
            scan_summary.file_count, scan_summary.dir_count, scan_summary.max_depth
        );

        if let Some(ref git) = git_summary {
            println!(
                "Git: {} commits, {} files changed (last 90 days)",
                git.total_commits, git.files_analyzed
            );
        }

        match &trend_data {
            Some(t) => println!("{}", trend::format_health_with_trend(comp, s, c, f, t)),
            None => {
                let f_display = match f {
                    Some(v) => format!("{v}"),
                    None => "N/A".to_string(),
                };
                println!(
                    "Health: {comp}/100 structural: {s} complexity: {c} fragility: {f_display}"
                );
            }
        }

        diagnose::print_issues(&issues);

        println!(
            "Snapshot #{snapshot_id} created for {}",
            project_path.display()
        );
    }

    Ok(critical_count > 0)
}
