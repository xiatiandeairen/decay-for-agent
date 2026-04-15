# Action Schema 技术方案

## 1. 背景

PRD 要求将文本处方升级为结构化 Action 类型系统。需要新增 action 模块，修改 Issue/Report 结构，并在 structural 维度做 POC 验证。

## 2. 方案

### 2.1 Action Schema（新增 src/action.rs）

```rust
use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    Split,     // 拆分过大的文件/目录/函数
    Extract,   // 提取模块/接口/公共逻辑
    Add,       // 添加缺失的测试/日志/错误处理
    Remove,    // 删除死代码/无用依赖
    Replace,   // 替换 unsafe/panic/硬编码
    Move,      // 移动文件到更合适的位置
    Refactor,  // 通用重构
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Effort {
    Small,   // < 30 min, 1 file
    Medium,  // 30 min - 2 hr, 2-5 files
    Large,   // > 2 hr, 5+ files or cross-module
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Target {
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_range: Option<(u32, u32)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Action {
    pub dimension: String,
    pub action_type: ActionType,
    pub target: Target,
    pub reason: String,
    pub priority: Priority,
    pub effort: Effort,
}
```

Display trait 实现：`[PRIORITY] ACTION_TYPE target.file — reason`

### 2.2 diagnose.rs 变更

```rust
// before
pub struct Issue {
    pub level: Level,
    pub category: String,
    pub message: String,
    pub prescription: Option<String>,
}

// after
pub struct Issue {
    pub level: Level,
    pub category: String,
    pub message: String,
    pub prescription: Option<String>,           // 保留，双写过渡
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<action::Action>,           // 新增
}
```

- Issue::Display 不变（只显示 prescription 文本）
- actions 序列化时空数组跳过，JSON 向后兼容

### 2.3 run.rs 变更

Report 新增 actions 字段：

```rust
pub struct Report {
    // ...existing fields...
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<action::Action>,
}
```

run() 函数在收集完 all_issues 后，额外收集 actions：

```rust
let mut all_actions: Vec<Action> = all_issues
    .iter()
    .flat_map(|i| i.actions.iter().cloned())
    .collect();
all_actions.sort_by(|a, b| a.priority.cmp(&b.priority));
```

Markdown 输出在 issues 之后新增 Actions 段落（仅当 actions 非空时）。

### 2.4 structural.rs POC 变更

3 个 Issue 附带 Action：

| Issue | Action |
|-------|--------|
| file_count > CRIT (1000+) | `Split, file: "src/", priority: Critical, effort: Large` |
| file_count > WARN (500+) | `Refactor, file: "src/", priority: High, effort: Medium` |
| max_depth > WARN (5+) | `Move, file: "(deepest path)", priority: Medium, effort: Medium` |

top_dirs issue 不附带 action（info 级，无明确操作目标）。

### 2.5 main.rs 变更

添加 `mod action;` 声明。

### 2.6 JSON 输出格式

重构前：
```json
{
  "issues": [
    { "level": "warning", "category": "structural", "message": "600 files", "prescription": "review..." }
  ]
}
```

重构后：
```json
{
  "issues": [
    {
      "level": "warning", "category": "structural", "message": "600 files",
      "prescription": "review...",
      "actions": [{
        "dimension": "structural",
        "action_type": "refactor",
        "target": { "file": "src/" },
        "reason": "600 files exceed 500 threshold, review for extractable modules",
        "priority": "high",
        "effort": "medium"
      }]
    }
  ],
  "actions": [
    { "dimension": "structural", "action_type": "refactor", "target": { "file": "src/" }, ... }
  ]
}
```

- 无 action 的 issue 不输出 actions 字段（skip_serializing_if）
- 顶层 actions 为空时不输出

## 3. 文件变更清单

| Action | File | 变更 |
|--------|------|------|
| create | `src/action.rs` | Action/ActionType/Priority/Effort/Target + Serialize + Display |
| modify | `src/main.rs` | 添加 `mod action` |
| modify | `src/diagnose.rs` | Issue 新增 `actions: Vec<Action>` 字段 |
| modify | `src/run.rs` | Report 新增 `actions`，run() 收集+排序，markdown 渲染 |
| modify | `src/dimension/structural.rs` | 3 个 Issue 附带 Action |
| create | `tests/action_schema.rs` | Action 序列化 roundtrip 测试 |

## 4. 关键决策

| 决策 | 结论 | 理由 |
|------|------|------|
| actions 容器类型 | `Vec<Action>` 而非 `Option<Action>` | 不阻塞 1:N，JSON schema 稳定 |
| prescription 保留 | 双写过渡 | CLI 输出不中断，M3 移除 |
| skip_serializing_if | actions 空时跳过 | JSON 向后兼容，无 action 的 issue 输出不变 |
| Priority 排序 | derive PartialOrd + Ord | Critical < High < Medium < Low，sort_by 自然升序 = 高优先级在前 |
| Target.file | 必填 String | M1 只有目录/文件级，M3 补充行级 |
| Effort 定义 | Small/Medium/Large | 比数值估算更实用，agent 可按 effort 过滤 |

## 5. 迭代记录

- 2026-04-15: 初始方案
