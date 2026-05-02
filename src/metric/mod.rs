pub mod cognitive;
pub mod cyclomatic;
pub mod nesting;
pub mod params;

use crate::types::Metrics;

pub fn compute(_tree: &tree_sitter::Tree, _source: &str, _body_range: tree_sitter::Range) -> Metrics {
    todo!()
}
