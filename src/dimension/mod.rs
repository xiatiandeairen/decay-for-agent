pub mod complexity;
pub mod fragility;
pub mod maintainability;
pub mod observability;
pub mod performance;
pub mod quality;
pub mod reliability;
pub mod structural;

use anyhow::Result;

use crate::data_store::DataStore;
use crate::diagnose::Issue;

/// Result of evaluating a single dimension.
pub struct DimensionResult {
    pub name: String,
    /// None = data source unavailable (e.g. no git history for fragility).
    pub score: Option<i32>,
    pub issues: Vec<Issue>,
}

/// A measurable dimension of code health.
///
/// Dimensions pull data from DataStore (lazy-loaded, cached).
/// DB-only dimensions use store.conn(); file-based dimensions use store.source_files().
pub trait Dimension: Send + Sync {
    fn name(&self) -> &'static str;

    fn score(&self, store: &DataStore) -> Result<Option<i32>>;

    fn diagnose(&self, store: &DataStore) -> Result<Vec<Issue>>;

    fn evaluate(&self, store: &DataStore) -> Result<DimensionResult> {
        Ok(DimensionResult {
            name: self.name().to_string(),
            score: self.score(store)?,
            issues: self.diagnose(store)?,
        })
    }
}

/// Registry of all enabled dimensions.
pub fn all_dimensions() -> Vec<Box<dyn Dimension>> {
    vec![
        Box::new(structural::Structural),
        Box::new(complexity::Complexity),
        Box::new(fragility::Fragility),
        Box::new(maintainability::Maintainability),
        Box::new(observability::Observability),
        Box::new(quality::QualityAssurance),
        Box::new(reliability::Reliability),
        Box::new(performance::Performance),
    ]
}
