# 可维护性维度 技术方案

## 1. 背景

PRD 要求新增 maintainability 维度，基于文件内容分析检测重复代码、长文件/函数、TODO/FIXME。

## 2. 方案

### 2.1 新增 Collector: ContentAnalysis

需要一个新的 collector 读取文件内容并写入分析结果。但为了简化 M4，直接在 dimension 的 score/diagnose 中读取文件内容（从 DB 的 files 表获取路径，然后读文件系统）。不新增 collector 和 DB 表。

理由：maintainability 的分析是轻量级的（行数统计、哈希、grep），不需要持久化中间结果。

### 2.2 Dimension 实现

```rust
// src/dimension/maintainability.rs

pub struct Maintainability;

impl Dimension for Maintainability {
    fn name(&self) -> &'static str { "maintainability" }
    fn score(&self, conn, snapshot_id) -> Result<Option<i32>> { ... }
    fn diagnose(&self, conn, snapshot_id) -> Result<Vec<Issue>> { ... }
}
```

### 2.3 评分逻辑（扣分制，100 分起）

| 指标 | 阈值 | 扣分 |
|------|------|------|
| 重复代码比例 | >5% 文件有显著重复块 | -15 |
| 重复代码比例 | >15% 文件有显著重复块 | -35（替代） |
| 长文件比例（>300行） | >20% | -15 |
| 长文件比例（>300行） | >40% | -30（替代） |
| 长函数比例（>50行） | >10% of functions | -10 |
| 长函数比例（>50行） | >25% of functions | -20（替代） |
| TODO/FIXME 密度 | >20 per 10K lines | -5 |

### 2.4 重复代码检测算法

行级哈希：
1. 读取所有源文件，对每行（trim + 跳过空行/注释行）计算哈希
2. 连续 N 行（N=6）的哈希序列作为"块指纹"
3. 块指纹在多个文件中出现 → 标记为重复
4. 统计有重复块的文件比例

### 2.5 函数长度检测

正则匹配常见函数定义模式：
- Rust: `fn \w+`
- Python: `def \w+`
- JavaScript/TypeScript: `function \w+` / `\w+\s*=.*=>`
- Go: `func \w+`
- Java/Kotlin: `(public|private|protected).*\w+\s*\(`

计算两个函数定义之间的行数作为函数体长度估算。

### 2.6 项目路径

从 DB files 表读取项目文件路径列表。需要还原绝对路径来读取文件内容。`files` 表存储的是相对路径，需要从 `snapshots` 表获取 `project_path` 拼接。

## 3. 文件变更清单

| Action | File | 变更 |
|--------|------|------|
| create | `src/dimension/maintainability.rs` | Maintainability 维度实现 |
| modify | `src/dimension/mod.rs` | 注册 Maintainability |
| modify | `src/profile.rs` | ScoreProfile 添加 maintainability 权重 |

## 4. 关键决策

| 决策 | 结论 | 理由 |
|------|------|------|
| 是否新增 collector | 否 | 直接读文件内容，无需持久化中间结果 |
| 重复检测算法 | 行级哈希 | 简单可靠，语言无关 |
| 函数长度检测 | 正则匹配 | 启发式足够，不需要解析器 |
| 最小重复块大小 | 6 行 | 平衡精度和误报 |

## 5. 迭代记录

- 2026-04-14: 初始方案
