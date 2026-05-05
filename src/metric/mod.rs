pub mod cognitive;
pub mod condition_ops;
pub mod cyclomatic;
pub mod nesting;
pub mod params;
pub mod statements;

use crate::types::{MetricId, Metrics};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemGroupId {
    HardToFollowLogic,
    LargeFunctionBody,
    WideInterface,
    CompoundConditions,
}

#[derive(Debug, Clone, Copy)]
pub struct MetricDef {
    pub id: MetricId,
    pub key: &'static str,
    pub measure_name: &'static str,
    pub threshold: u32,
    pub group: ProblemGroupId,
    pub format: fn(u32) -> String,
}

pub const ACTIVE_METRICS: &[MetricDef] = &[
    MetricDef {
        id: MetricId::Nesting,
        key: "nesting",
        measure_name: "Nested control flow depth",
        threshold: 4,
        group: ProblemGroupId::HardToFollowLogic,
        format: format_depth,
    },
    MetricDef {
        id: MetricId::Cyclomatic,
        key: "cyclomatic",
        measure_name: "Branch count",
        threshold: 10,
        group: ProblemGroupId::HardToFollowLogic,
        format: format_plain,
    },
    MetricDef {
        id: MetricId::Cognitive,
        key: "cognitive",
        measure_name: "Branching complexity",
        threshold: 15,
        group: ProblemGroupId::HardToFollowLogic,
        format: format_plain,
    },
    MetricDef {
        id: MetricId::Params,
        key: "params",
        measure_name: "Parameter count",
        threshold: 5,
        group: ProblemGroupId::WideInterface,
        format: format_params,
    },
    MetricDef {
        id: MetricId::StatementCount,
        key: "statement_count",
        measure_name: "Function size",
        threshold: 25,
        group: ProblemGroupId::LargeFunctionBody,
        format: format_statements,
    },
    MetricDef {
        id: MetricId::MaxConditionOps,
        key: "max_condition_ops",
        measure_name: "Boolean condition complexity",
        threshold: 4,
        group: ProblemGroupId::CompoundConditions,
        format: format_boolean_ops,
    },
];

pub fn compute(tree: &tree_sitter::Tree, source: &str, body_range: tree_sitter::Range) -> Metrics {
    Metrics {
        nesting: nesting::compute(tree, source, body_range),
        cyclomatic: cyclomatic::compute(tree, source, body_range),
        cognitive: cognitive::compute(tree, source, body_range),
        params: params::compute(tree, source, body_range),
        statement_count: statements::compute(tree, source, body_range),
        max_condition_ops: condition_ops::compute(tree, source, body_range),
    }
}

pub fn def(id: MetricId) -> &'static MetricDef {
    ACTIVE_METRICS
        .iter()
        .find(|metric| metric.id == id)
        .expect("active metric definition")
}

pub fn active_values(metrics: Metrics) -> impl Iterator<Item = (&'static MetricDef, u32)> {
    ACTIVE_METRICS
        .iter()
        .map(move |metric| (metric, metrics.value(metric.id)))
}

pub fn breaches_threshold(value: u32, threshold: u32) -> bool {
    value > threshold
}

pub fn crossed_threshold(previous: u32, current: u32, threshold: u32) -> bool {
    !breaches_threshold(previous, threshold) && breaches_threshold(current, threshold)
}

pub fn worsened_over_threshold(previous: u32, current: u32, threshold: u32) -> bool {
    breaches_threshold(previous, threshold) && current > previous
}

fn format_plain(value: u32) -> String {
    value.to_string()
}

fn format_depth(value: u32) -> String {
    format!("depth {value}")
}

fn format_params(value: u32) -> String {
    format!("{value} parameters")
}

fn format_statements(value: u32) -> String {
    format!("{value} statements")
}

fn format_boolean_ops(value: u32) -> String {
    format!("{value} boolean operators")
}
