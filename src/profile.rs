use std::collections::HashMap;
use std::path::Path;

use log::debug;

/// Detected project type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectType {
    Cli,
    WebService,
    Library,
    MobileApp,
    Monorepo,
    Generic,
}

/// Scoring profile: dimension weights for composite score.
pub struct ScoreProfile {
    pub project_type: ProjectType,
    pub weights: HashMap<String, f64>,
}

impl ScoreProfile {
    /// Build a profile for the given project type.
    pub fn for_type(pt: ProjectType) -> Self {
        let weights = match pt {
            ProjectType::Cli => vec![
                ("structural", 0.25),
                ("complexity", 0.30),
                ("fragility", 0.25),
                ("maintainability", 0.20),
            ],
            ProjectType::WebService => vec![
                ("structural", 0.20),
                ("complexity", 0.25),
                ("fragility", 0.35),
                ("maintainability", 0.20),
            ],
            ProjectType::Library => vec![
                ("structural", 0.20),
                ("complexity", 0.30),
                ("fragility", 0.20),
                ("maintainability", 0.30),
            ],
            ProjectType::MobileApp => vec![
                ("structural", 0.20),
                ("complexity", 0.30),
                ("fragility", 0.30),
                ("maintainability", 0.20),
            ],
            ProjectType::Monorepo => vec![
                ("structural", 0.30),
                ("complexity", 0.25),
                ("fragility", 0.25),
                ("maintainability", 0.20),
            ],
            ProjectType::Generic => vec![
                ("structural", 0.25),
                ("complexity", 0.25),
                ("fragility", 0.25),
                ("maintainability", 0.25),
            ],
        };
        ScoreProfile {
            project_type: pt,
            weights: weights
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        }
    }

    /// Compute weighted composite score from dimension scores.
    ///
    /// Dimensions with None score are skipped. Weights are renormalized
    /// over available dimensions.
    pub fn weighted_composite(&self, scores: &HashMap<String, Option<i32>>) -> i32 {
        let mut weighted_sum = 0.0;
        let mut weight_sum = 0.0;

        for (name, score) in scores {
            if name == "composite" {
                continue;
            }
            if let Some(s) = score {
                let w = self.weights.get(name).copied().unwrap_or(1.0);
                weighted_sum += *s as f64 * w;
                weight_sum += w;
            }
        }

        if weight_sum == 0.0 {
            return 0;
        }

        (weighted_sum / weight_sum).round() as i32
    }
}

/// Detect project type from file system signals.
pub fn detect(project_path: &Path) -> ProjectType {
    // MobileApp: iOS or Android
    if project_path.join("Info.plist").exists()
        || project_path.join("AndroidManifest.xml").exists()
        || has_file_recursive(project_path, "Info.plist", 2)
        || has_file_recursive(project_path, "AndroidManifest.xml", 3)
    {
        debug!("detected: mobile_app");
        return ProjectType::MobileApp;
    }

    // Monorepo: multiple Cargo.toml or workspace
    if is_monorepo(project_path) {
        debug!("detected: monorepo");
        return ProjectType::Monorepo;
    }

    // WebService: web framework dependency
    if is_web_service(project_path) {
        debug!("detected: web_service");
        return ProjectType::WebService;
    }

    // Library: lib entry without main
    if is_library(project_path) {
        debug!("detected: library");
        return ProjectType::Library;
    }

    // CLI: has main + CLI framework dependency
    if is_cli(project_path) {
        debug!("detected: cli");
        return ProjectType::Cli;
    }

    debug!("detected: generic");
    ProjectType::Generic
}

fn has_file_recursive(base: &Path, name: &str, max_depth: usize) -> bool {
    if max_depth == 0 {
        return false;
    }
    let Ok(entries) = std::fs::read_dir(base) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.file_name().is_some_and(|n| n == name) {
            return true;
        }
        if path.is_dir() && has_file_recursive(&path, name, max_depth - 1) {
            return true;
        }
    }
    false
}

fn is_monorepo(project_path: &Path) -> bool {
    // Check Cargo workspace
    let cargo_toml = project_path.join("Cargo.toml");
    if cargo_toml.exists() {
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            if content.contains("[workspace]") {
                return true;
            }
        }
    }

    // Check package.json workspaces
    let pkg_json = project_path.join("package.json");
    if pkg_json.exists() {
        if let Ok(content) = std::fs::read_to_string(&pkg_json) {
            if content.contains("\"workspaces\"") {
                return true;
            }
        }
    }

    // Check pnpm-workspace.yaml
    if project_path.join("pnpm-workspace.yaml").exists() {
        return true;
    }

    false
}

fn is_web_service(project_path: &Path) -> bool {
    let web_signals = [
        "actix", "axum", "rocket", "warp", "tide",         // Rust
        "express", "fastify", "koa", "nest", "next",        // Node
        "flask", "django", "fastapi", "starlette",          // Python
        "gin", "echo", "fiber",                              // Go
        "spring-boot", "quarkus",                            // Java
    ];
    dep_file_contains(project_path, &web_signals)
}

fn is_library(project_path: &Path) -> bool {
    // Rust: lib.rs without main.rs in src/
    let src = project_path.join("src");
    if src.join("lib.rs").exists() && !src.join("main.rs").exists() {
        return true;
    }

    // Python: __init__.py without __main__.py
    // Check top-level dirs for Python package pattern
    if let Ok(entries) = std::fs::read_dir(project_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir()
                && path.join("__init__.py").exists()
                && !path.join("__main__.py").exists()
                && !project_path.join("setup.py").exists()
            {
                // Has package but no main entry — could be library
                // but also needs no obvious CLI/web indicators
                return true;
            }
        }
    }

    false
}

fn is_cli(project_path: &Path) -> bool {
    let cli_signals = [
        "clap", "structopt", "argh",        // Rust
        "argparse", "click", "typer",        // Python
        "cobra", "urfave/cli",               // Go
        "commander", "yargs", "meow",        // Node
    ];

    // Must have a main entry point
    let has_main = project_path.join("src/main.rs").exists()
        || project_path.join("main.py").exists()
        || project_path.join("main.go").exists()
        || project_path.join("cmd").is_dir();

    has_main && dep_file_contains(project_path, &cli_signals)
}

/// Check if any dependency file contains one of the given signals.
fn dep_file_contains(project_path: &Path, signals: &[&str]) -> bool {
    let dep_files = [
        "Cargo.toml",
        "package.json",
        "requirements.txt",
        "pyproject.toml",
        "go.mod",
        "pom.xml",
        "build.gradle",
    ];

    for dep_file in &dep_files {
        let path = project_path.join(dep_file);
        if let Ok(content) = std::fs::read_to_string(&path) {
            let content_lower = content.to_lowercase();
            for signal in signals {
                if content_lower.contains(&signal.to_lowercase()) {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_detect_cli_rust() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[dependencies]\nclap = \"4\"",
        )
        .unwrap();
        assert_eq!(detect(dir.path()), ProjectType::Cli);
    }

    #[test]
    fn test_detect_web_service() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[dependencies]\naxum = \"0.7\"",
        )
        .unwrap();
        assert_eq!(detect(dir.path()), ProjectType::WebService);
    }

    #[test]
    fn test_detect_library() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/lib.rs"), "pub fn hello() {}").unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"mylib\"").unwrap();
        assert_eq!(detect(dir.path()), ProjectType::Library);
    }

    #[test]
    fn test_detect_monorepo() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\", \"b\"]",
        )
        .unwrap();
        assert_eq!(detect(dir.path()), ProjectType::Monorepo);
    }

    #[test]
    fn test_detect_generic() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("README.md"), "# hello").unwrap();
        assert_eq!(detect(dir.path()), ProjectType::Generic);
    }

    #[test]
    fn test_weighted_composite_equal() {
        let profile = ScoreProfile::for_type(ProjectType::Generic);
        let mut scores = HashMap::new();
        scores.insert("structural".to_string(), Some(100));
        scores.insert("complexity".to_string(), Some(100));
        scores.insert("fragility".to_string(), Some(100));
        assert_eq!(profile.weighted_composite(&scores), 100);
    }

    #[test]
    fn test_weighted_composite_web_service() {
        let profile = ScoreProfile::for_type(ProjectType::WebService);
        let mut scores = HashMap::new();
        scores.insert("structural".to_string(), Some(100));
        scores.insert("complexity".to_string(), Some(100));
        scores.insert("fragility".to_string(), Some(0));
        scores.insert("maintainability".to_string(), Some(100));
        // 100*0.20 + 100*0.25 + 0*0.35 + 100*0.20 = 65
        let comp = profile.weighted_composite(&scores);
        assert_eq!(comp, 65);
    }

    #[test]
    fn test_weighted_composite_skips_none() {
        let profile = ScoreProfile::for_type(ProjectType::Cli);
        let mut scores = HashMap::new();
        scores.insert("structural".to_string(), Some(80));
        scores.insert("complexity".to_string(), Some(60));
        scores.insert("fragility".to_string(), None);
        scores.insert("maintainability".to_string(), Some(100));
        // (80*0.25 + 60*0.30 + 100*0.20) / (0.25+0.30+0.20) = (20+18+20)/0.75 ≈ 77
        let comp = profile.weighted_composite(&scores);
        assert_eq!(comp, 77);
    }

    #[test]
    fn test_detect_current_project() {
        // This project has src/main.rs + clap in Cargo.toml → CLI
        let project_path = Path::new(env!("CARGO_MANIFEST_DIR"));
        let pt = detect(project_path);
        assert_eq!(pt, ProjectType::Cli);
    }
}
