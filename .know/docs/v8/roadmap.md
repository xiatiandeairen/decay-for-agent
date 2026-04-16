# decay-for-agent 产品路线图

## 1. 产品愿景

|            |                                       |
| ---------- | ------------------------------------- |
| **解决什么问题** | 项目在持续迭代中悄无声息地积累结构性债务，等到问题显现时清理成本已经很高  |
| **给谁用**    | 使用 Claude Code 的开发者和 AI agent         |
| **核心差异**   | 不是静态 lint，而是跨快照的趋势追踪 + agent 可消费的重构处方 |

## 2. 版本规划

### v1-v7 ✅

| 版本 | 主题 | 能力 |
|------|------|------|
| v1 | CLI 核心闭环 | 单命令健康检查 |
| v2 | Claude Code 集成 | MCP + skill |
| v3 | 多维度评估 | 8 维度 + 自适应打分 |
| v4 | 可执行处方 | 结构化 action + 精确位置 |
| v5 | 时序智能 | velocity + 回归 + 预警 + 相关性 |
| v6 | 智能诊断 | 8 类问题分类 + 模式聚合 + patch + 预防 |
| v7 | 闭环打磨 | 评分校准 + 降噪 + 处方深化 + summary + compare |

### v8 — Impact-Driven Decay（影响驱动的衰退治理）

| 维度     | 标准                                                         |
| ------ | ---------------------------------------------------------- |
| **功能** | 每个问题附带开发影响评估，处方执行后自动追踪效果，基于历史生成改善计划，报告用叙事而非表格 |
| **质量** | 影响评估有实际项目回测，处方效果追踪有 before/after 验证，改善计划有优先级排序验证 |
| **用户** | 不再只看到"哪里不好"，而是看到"这样改会带来什么好处"、"改完后确实变好了"、"下一步该做什么" |


#### 核心理念

**从"发现问题"到"感受改善"**

v1-v7 建立了完整的检测→诊断→处方→趋势管线。但用户的核心体验仍然是"被告知哪里不好"。v8 翻转叙事：不是"你有 87 个问题"，而是"修这 3 个问题能让你每次改代码省 15 分钟"。

三个关键转变：
1. **从指标到影响**：不只说"函数 100 行"，说"这个函数每次 debug 你要花 10 分钟理解上下文"
2. **从建议到验证**：不只说"拆分这个文件"，而是拆完后告诉你"改动影响范围从 5 个文件降到 2 个"
3. **从快照到演进**：不只看当前状态，而是"过去 5 次迭代你的可维护性提升了 25 分，主要因为..."


#### 里程碑

| #   | 里程碑                                                    | 验证点                                              | 进度 |
| --- | -------------------------------------------------------- | --------------------------------------------------- | -- |
| M1  | **影响度量化** — 每个 issue 附带开发影响评估                            | "这个 1000 行文件让每次改动平均影响 3 个其他文件" 等量化影响语句           | ✅ |
| M2  | **处方效果追踪** — 记录执行了哪些处方，对比前后维度变化                        | `--compare last` 输出 "执行 3 个处方后：maintainability +15" |    |
| M3  | **演进守护** — 自适应阈值 + 回退检测：分数下降时自动标记是哪次变更引入的               | "snapshot #15 的 reliability 下降因为 commit abc123 新增了 5 个 unwrap" |    |
| M4  | **改善计划** — 基于优先级和影响度生成分阶段改善路线图                          | "Phase 1: 修 3 个 A 类 (2h), Phase 2: 重构 1 个 B 类 (4h)" |    |
| M5  | **报告叙事化** — 从指标表格到可读叙事："你的项目正在 X 方面变好，因为 Y"            | 报告首段是自然语言摘要，不是数字表格                              | ✅ |
| M6  | **输出层升级** — terminal/markdown 展示 diagnostic report + summary | 用户看到的是分类视图而非 issue 列表                            | ✅ |
| M7  | **MCP 摘要优先** — 返回 summary 在最前面，agent 按需展开              | agent 单次调用获得分层信息                                  | ✅ |
| M8  | **自动对比** — `--compare last` 自动对比上一个快照                    | 无需记 snapshot ID                                    | ✅ |
| M9  | **skill 升级** — `/decay` 输出 summary + top actions + 改善趋势  | 用户一目了然                                            | ✅ |


## 3. 当前版本

### 核心价值

**从"发现问题"到"感受改善"** — 每个问题都有开发影响评估（"这个问题让你多花 X 时间"），每个处方执行后都能看到效果（"改了之后影响范围从 5 文件降到 2 文件"），基于历史生成连贯的改善计划而非散点建议。

### 包含

| 能力              | 解决什么问题                                                 |
| --------------- | ------------------------------------------------------ |
| **影响度量化**      | "87 个 issues" 看不出哪个该先修 → 每个 issue 附带 "影响 N 个文件/每次改动多 X 分钟" |
| **处方效果追踪**    | 修了不知道有没有用 → 自动对比执行前后的维度变化                          |
| **演进守护**        | 改好了又回退 → 自适应阈值 + 回退检测 + 归因到具体 commit                |
| **改善计划**        | 每次独立建议 → 分阶段路线图 "先做高 ROI 的，再做架构的"                  |
| **叙事化报告**      | 数字表格 → "你的项目在可维护性上持续改善，主要因为拆分了 3 个大文件"              |
| **输出层升级**      | issue 列表 → 分类视图 + 摘要 + 自动对比                          |

### 排除

- 自动执行修复（生成处方和计划，执行仍由用户/agent 控制）
- 多项目聚合（Portfolio Health — v9 方向）
- 自定义规则引擎（内置规则 + 影响评估已够用）
- GUI/Dashboard

## 4. 技术方向

### M1 影响度量化

每个 issue 新增 `impact` 字段：

```rust
pub struct Impact {
    pub affected_files: usize,     // 改这个文件通常影响多少其他文件（来自 git 共变分析）
    pub avg_review_burden: String, // "这个函数需要 ~N 分钟理解上下文"（基于行数+复杂度）
    pub change_risk: String,       // "改动影响范围：高/中/低"（基于 churn + coupling）
}
```

数据源：
- `affected_files`：git 共变分析 — 过去改了文件 A 时同时改了哪些文件
- `avg_review_burden`：函数行数 × 嵌套深度 → 估算理解时间
- `change_risk`：churn 密度 + 被依赖数 → 变更风险等级

### M2 处方效果追踪

在 snapshot DB 中记录已执行的 action：

```sql
CREATE TABLE action_log (
    id INTEGER PRIMARY KEY,
    snapshot_id INTEGER,      -- 在哪个快照后执行
    action_hash TEXT,         -- action 的唯一标识
    executed_at TEXT
);
```

`--compare` 增强：不只对比分数，还关联 "这次改善是因为执行了哪些处方"。

### M3 演进守护

自适应阈值：基于项目自身历史的均值 ± 1σ，而非全局固定阈值。

回退归因：当分数下降时，对比 git diff 找到新增的问题代码。

### M4 改善计划

```rust
pub struct ImprovementPlan {
    pub phases: Vec<Phase>,
    pub estimated_total_effort: String,
    pub expected_composite_gain: i32,
}

pub struct Phase {
    pub name: String,           // "Phase 1: Quick Wins"
    pub actions: Vec<Action>,
    pub estimated_effort: String,
    pub expected_gain: i32,
}
```

分阶段策略：
1. Quick Wins — A 类机械修复（effort=S, impact 高）
2. Pattern Fix — B 类共性问题（effort=M, 消除根因）
3. Structural — C 类架构决策（effort=L, 长期收益）

### M5 叙事化报告

从 summary 的一行扩展为段落叙事：

```
Your project health is 92/100 and improving (↑3 since last scan).

Key improvements: maintainability rose from 45 to 70 after splitting trend.rs into 
7 focused modules. Performance improved to 85 after adjusting detection thresholds.

Remaining concerns: 13 files share duplicated test patterns (category B). 
This costs ~5 minutes per test change because fixes must be replicated across files.
Extracting shared test helpers would eliminate this.

Recommended next step: Extract test helpers into a shared module (effort: S, ~30 min).
This alone would improve maintainability by ~5 points.
```

## 5. 风险与依赖

| 风险               | 影响             | 缓解                              |
| ---------------- | -------------- | ------------------------------- |
| 影响评估不准确         | 用户不信任 "节省 X 分钟" | 保守估算，标注 "estimated"，可关闭        |
| git 共变分析慢        | 大项目扫描时间长        | 缓存 + 增量分析                        |
| 叙事生成质量           | 模板化文本感觉机械       | 条件丰富的模板 + 关键数据填充               |
| 改善计划过于激进         | 用户感觉压力大         | 计划分阶段，Phase 1 控制在 2 小时内        |

## 6. 后续版本方向（存档）

### v9+ — Portfolio Health + Auto-execute

- 多项目健康聚合：组织级仪表盘
- 跨项目模式发现：所有项目的共性衰退趋势
- Auto-execute：A 类 patch 自动 apply → cargo check → 提 PR
- 团队级预警：组织范围内的健康阈值管理
- 自定义规则引擎：项目级自定义检测规则和阈值
