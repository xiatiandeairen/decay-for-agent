use std::path::Path;

use clap::ValueEnum;

use crate::parser::ParsedFunc;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum ScanScope {
    #[default]
    Prod,
    All,
}

impl ScanScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Prod => "prod",
            Self::All => "all",
        }
    }

    pub fn includes_path(self, project_root: &Path, path: &Path) -> bool {
        match self {
            Self::All => true,
            Self::Prod => is_prod_path(project_root, path),
        }
    }

    pub fn includes_function(
        self,
        project_root: &Path,
        path: &Path,
        parsed_func: &ParsedFunc,
    ) -> bool {
        match self {
            Self::All => true,
            Self::Prod => {
                is_prod_path(project_root, path)
                    && !is_test_support_path(project_root, path)
                    && !parsed_func.is_test_like
            }
        }
    }
}

fn is_prod_path(project_root: &Path, path: &Path) -> bool {
    let rel = path.strip_prefix(project_root).unwrap_or(path);
    let rel = rel.to_string_lossy().replace('\\', "/");
    if rel == "build.rs" || rel.ends_with("/build.rs") {
        return true;
    }

    !rel.split('/')
        .any(|part| matches!(part, "tests" | "examples" | "benches" | "fixtures"))
}

fn is_test_support_path(project_root: &Path, path: &Path) -> bool {
    let rel = path.strip_prefix(project_root).unwrap_or(path);
    let file_stem = rel.file_stem().and_then(|name| name.to_str()).unwrap_or("");
    file_stem == "testutil"
}
