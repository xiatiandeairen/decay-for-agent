//! Pipeline orchestrator: walk → parse → metrics → fingerprint.
//!
//! Single public entry: [`scan`] takes a project root and returns the full
//! list of functions with metrics + fingerprints filled in. Per-file parse
//! failures are logged at `warn` and skipped — the run still produces a
//! valid (partial) snapshot, matching §2.7 strategy.

use std::path::Path;

use crate::error::Result;
use crate::fingerprint;
use crate::metric;
use crate::parser;
use crate::types::{Function, Metrics};
use crate::walk;

/// Walk `project_root`, parse every `.rs` file, compute metrics + fingerprint
/// for each function, return the flat list.
///
/// Single-file parse errors (read error / tree-sitter syntax error) are logged
/// via `log::warn!` and the file is skipped — the overall scan does not abort.
/// This matches §2.7: "single file parse failure → pipeline catches, log warn,
/// skip continue".
///
/// IO errors at the directory-walk level (e.g. `project_root` does not exist)
/// propagate as `DecayError::Io` and abort the scan.
pub fn scan(project_root: &Path) -> Result<Vec<Function>> {
    let files = walk::walk_rust_files(project_root)?;
    let mut out: Vec<Function> = Vec::new();

    for file in files {
        let parsed = match parser::parse_file(&file, project_root) {
            Ok(p) => p,
            Err(e) => {
                log::warn!("skipping {}: {e}", file.display());
                continue;
            }
        };

        for pf in parsed.funcs {
            let metrics = Metrics {
                nesting: metric::nesting::compute(&parsed.tree, &parsed.source, pf.body_range),
                cyclomatic: metric::cyclomatic::compute(
                    &parsed.tree,
                    &parsed.source,
                    pf.body_range,
                ),
                cognitive: metric::cognitive::compute(&parsed.tree, &parsed.source, pf.body_range),
                params: metric::params::compute(&parsed.tree, &parsed.source, pf.body_range),
            };

            let signature_hash = fingerprint::compute(
                &pf.function.file,
                &pf.function.name,
                &pf.function.param_types,
            );

            out.push(Function {
                metrics,
                signature_hash,
                ..pf.function
            });
        }
    }

    Ok(out)
}
