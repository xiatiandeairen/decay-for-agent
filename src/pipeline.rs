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
use crate::scope::ScanScope;
use crate::types::Function;
use crate::walk;

pub struct ScanOutput {
    pub funcs: Vec<Function>,
    pub diagnostics: Vec<ScanDiagnostic>,
}

pub struct ScanDiagnostic {
    pub path: String,
    pub message: String,
}

/// Walk `project_root`, parse every `.rs` file, compute metrics + fingerprint
/// for each function, and return functions plus scan diagnostics.
pub fn scan_with_excludes(
    project_root: &Path,
    excludes: &[String],
    scope: ScanScope,
) -> Result<ScanOutput> {
    let files = walk::walk_rust_files_with_excludes(project_root, excludes)?
        .into_iter()
        .filter(|path| scope.includes_path(project_root, path))
        .collect::<Vec<_>>();
    let mut out: Vec<Function> = Vec::new();
    let mut diagnostics = Vec::new();

    for file in files {
        let parsed = match parser::parse_file(&file, project_root) {
            Ok(p) => p,
            Err(e) => {
                log::warn!("skipping {}: {e}", file.display());
                diagnostics.push(ScanDiagnostic {
                    path: file.display().to_string(),
                    message: e.to_string(),
                });
                continue;
            }
        };

        for pf in parsed.funcs {
            if !scope.includes_function(project_root, &file, &pf) {
                continue;
            }
            let metrics = metric::compute(&parsed.tree, &parsed.source, pf.body_range);

            let signature_hash = fingerprint::compute(
                &pf.function.file,
                &pf.function.impl_context,
                &pf.function.cfg_context,
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

    Ok(ScanOutput {
        funcs: out,
        diagnostics,
    })
}
