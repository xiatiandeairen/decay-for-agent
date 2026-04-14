# Dimension trait 统一注册 技术方案

## 1. 背景

PRD 要求将 3 个维度抽象为统一 trait，通过注册表统一调度。需要重构 score.rs、diagnose.rs、run.rs、db.rs 四个模块。

## 2. 方案

### 2.1 Dimension trait

```rust
// src/dimension/mod.rs

pub struct DimensionResult {
    pub name: String,
    pub score: Option<i32>,  // None = 数据不可用（如无 git 时 fragility）
    pub issues: Vec<Issue>,
}

pub trait Dimension: Send + Sync {
    /// 维度名称，用于输出和持久化
    fn name(&self) -> &'static str;

    /// 计算评分（0-100，扣分制）
    /// 返回 None 表示该维度的数据源不可用
    fn score(&self, conn: &Connection, snapshot_id: i64) -> Result<Option<i32>>;

    /// 诊断问题并生成处方
    fn diagnose(&self, conn: &Connection, snapshot_id: i64) -> Result<Vec<Issue>>;

    /// 一次性执行评分 + 诊断
    fn evaluate(&self, conn: &Connection, snapshot_id: i64) -> Result<DimensionResult> {
        Ok(DimensionResult {
            name: self.name().to_string(),
            score: self.score(conn, snapshot_id)?,
            issues: self.diagnose(conn, snapshot_id)?,
        })
    }
}
```

### 2.2 注册表

```rust
// src/dimension/mod.rs

pub fn all_dimensions() -> Vec<Box<dyn Dimension>> {
    vec![
        Box::new(structural::Structural),
        Box::new(complexity::Complexity),
        Box::new(fragility::Fragility),
    ]
}
```

### 2.3 维度模块结构

每个维度一个文件，包含：
- 零大小 struct（如 `pub struct Structural;`）
- `impl Dimension for Structural`
- 阈值常量（从 score.rs 迁移）
- 评分逻辑（从 score.rs 迁移）
- 诊断逻辑（从 diagnose.rs 迁移）
- 单元测试（从 score.rs + diagnose.rs 迁移）

```
src/dimension/
├── mod.rs              # trait 定义 + 注册表
├── structural.rs       # structural 维度
├── complexity.rs       # complexity 维度
└── fragility.rs        # fragility 维度
```

### 2.4 diagnose.rs 变更

- `Category` enum 改为 `String` 类型（维度名即 category）
- `Issue` struct 的 `category` 字段类型从 `Category` 改为 `String`
- 删除 `run()` 函数（调度逻辑移到 run.rs 通过注册表实现）
- 保留 `Level` enum、`Issue` struct、`print_issues()` 函数

### 2.5 run.rs 变更

- `Scores` struct 从固定字段改为 `HashMap<String, Option<i32>>` + `composite: i32`
- 调度逻辑：遍历 `all_dimensions()`，调用 `evaluate()`，收集结果
- 输出层（terminal / JSON / markdown / quiet）适配动态维度

```rust
let dimensions = dimension::all_dimensions();
let mut results: Vec<DimensionResult> = Vec::new();
for dim in &dimensions {
    results.push(dim.evaluate(&conn, snapshot_id)?);
}
```

### 2.6 db.rs 变更

新增 `dimension_scores` 表（key-value 模式）：

```sql
CREATE TABLE IF NOT EXISTS dimension_scores (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id INTEGER NOT NULL REFERENCES snapshots(id),
    dimension TEXT NOT NULL,
    score INTEGER
);
```

- 新增 `insert_dimension_scores(conn, snapshot_id, results)` 函数
- 新增 `get_previous_dimension_scores(conn, project_path, snapshot_id)` 函数
- 保留旧 `scores` 表用于读取历史数据
- composite 存入 `dimension_scores` 表，dimension 名为 `"composite"`

### 2.7 trend.rs 变更

- `Trend` struct 从固定字段改为 `HashMap<String, TrendDirection>`
- `compare()` 函数接受动态维度的分数 map
- 输出格式适配动态维度

### 2.8 Markdown 模板变更

- 模板中的固定维度占位符改为循环生成
- `render_markdown()` 接受 `Vec<DimensionResult>` 而非固定字段

### 2.9 JSON 输出格式

重构前：
```json
{
  "scores": {
    "structural": 85,
    "complexity": 70,
    "fragility": 60,
    "composite": 71
  }
}
```

重构后：
```json
{
  "scores": {
    "structural": 85,
    "complexity": 70,
    "fragility": 60,
    "composite": 71
  }
}
```

格式不变，但内部实现从固定字段改为动态序列化。

## 3. 文件变更清单

| Action | File | 变更 |
|--------|------|------|
| create | `src/dimension/mod.rs` | Dimension trait + DimensionResult + all_dimensions() |
| create | `src/dimension/structural.rs` | 迁移 structural 评分 + 诊断 |
| create | `src/dimension/complexity.rs` | 迁移 complexity 评分 + 诊断 |
| create | `src/dimension/fragility.rs` | 迁移 fragility 评分 + 诊断 |
| modify | `src/main.rs` | 添加 `mod dimension` |
| modify | `src/diagnose.rs` | Category → String，删除 run()，保留 Level/Issue/print_issues |
| modify | `src/run.rs` | Scores → HashMap，用注册表循环替代硬编码 |
| modify | `src/db.rs` | 新增 dimension_scores 表 + 动态读写函数 |
| modify | `src/trend.rs` | Trend → HashMap，动态维度对比 |
| modify | `src/score.rs` | 仅保留 composite()，删除 3 个维度函数 |
| modify | `templates/health-report.md` | 适配动态维度 |

## 4. 关键决策

| 决策 | 结论 | 理由 |
|------|------|------|
| trait 方法签名 | score + diagnose 分离 | 允许只取分数（如 composite 计算）而不运行诊断 |
| Category 类型 | String（非 enum） | enum 无法在不修改定义处的情况下扩展 |
| DB schema | 新增 key-value 表 | 固定列无法支持动态维度数量 |
| 旧 scores 表 | 保留不删 | 向后兼容历史数据读取 |
| evaluate 默认实现 | trait 提供 default impl | 减少每个维度的样板代码 |

## 5. 迭代记录

- 2026-04-14: 初始方案
