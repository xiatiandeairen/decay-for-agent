use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub file: String,             // 项目根相对路径, 正斜杠
    pub impl_context: String,     // impl 块上下文 (eg "Foo" / "Display for Foo"); 自由 fn 为空
    pub cfg_context: String,      // 连续 #[cfg(...)] attribute 规范化文本; 无 cfg 时为空
    pub name: String,             // fn 名, 不含 impl receiver
    pub start_line: u32,          // 1-indexed
    pub end_line: u32,            // 1-indexed
    pub param_types: Vec<String>, // 归一化后, 用于指纹
    pub signature_hash: u64,      // fingerprint::compute() 结果
    pub metrics: Metrics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metrics {
    pub nesting: u32,
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub params: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: i64,
    pub project_id: String, // 项目根绝对路径
    pub created_at: i64,    // unix timestamp seconds
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffEntry {
    pub function: Function,        // 当前快照里的函数
    pub previous: Option<Metrics>, // None = 新增
    pub kind: DiffKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffKind {
    Added,
    Worsened,
    CrossedThreshold,
}
