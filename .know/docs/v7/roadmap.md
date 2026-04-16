# decay-for-agent 产品路线图

## 1. 产品愿景

|            |                                       |
| ---------- | ------------------------------------- |
| **解决什么问题** | 项目在持续迭代中悄无声息地积累结构性债务，等到问题显现时清理成本已经很高  |
| **给谁用**    | 使用 Claude Code 的开发者和 AI agent         |
| **核心差异**   | 不是静态 lint，而是跨快照的趋势追踪 + agent 可消费的重构处方 |

## 2. 版本规划

### v1 — Rust CLI 核心闭环 ✅
### v2 — Claude Code 集成层 ✅
### v3 — 多维度评估 + 智能打分 ✅
### v4 — Actionable Prescriptions ✅
### v5 — Temporal Intelligence ✅
### v6 — Intelligent Diagnosis ✅

### v7 — Core Loop Polish（核心闭环打磨）

| 维度     | 标准                                                         |
| ------ | ---------------------------------------------------------- |
| **功能** | 分数校准可信、检测噪声 <5%、处方包含具体拆分方案、MCP 分层摘要、修复后自动验证改善 |
| **质量** | 阈值有实际项目回测数据支撑、降噪有 before/after 对比验证、agent 摘要有集成测试 |
| **用户** | 分数和直觉一致，处方拿来就能执行，agent 一次调用获得分层信息，修复效果可量化 |


#### 里程碑

| #   | 里程碑                                                    | 验证点                                              | 进度 |
| --- | -------------------------------------------------------- | --------------------------------------------------- | -- |
| M1  | **评分校准** — 审视 8 维度阈值和权重，用实际项目回测校准                      | 自身项目 maintainability 从 45 校准到合理范围（60-75），分数与代码质量直觉一致 | ✅ |
| M2  | **检测降噪** — healthy churn 过滤、测试代码 blocking call 排除       | 自身项目 issues 从 115 降至 <60，噪声率从 20% 降至 <5%          | ✅ |
| M3  | **处方深化** — 从"split X"到"按 Y 职责拆为 A/B/C"，附文件/函数级拆分建议    | 处方包含具体模块拆分方案，agent 可直接执行而非二次推理                   | ✅ |
| M4  | **agent 摘要** — MCP 分层输出：摘要 + top 3 + 详细报告                | 单次 MCP 调用返回 3 层信息，agent 按需消费                      | ✅ |
| M5  | **反馈验证** — before/after 快照对比，输出改善报告                      | 执行处方后 re-scan，输出"X 维度从 Y 提升到 Z"的改善报告             | ✅ |


## 3. 当前版本

### 核心价值

**从"能用"到"好用"** — 不加新功能，打磨现有闭环的每个环节，让分数可信、处方可执行、效果可验证。

v1-v6 建立了完整管线（检测→评分→诊断→分类→处方→趋势→智能报告），但管线质量有 5 个断点：分数偏差、检测噪声、处方模糊、信息过载、无反馈。v7 逐个修补，让闭环真正闭合。

### 包含

| 能力              | 解决什么问题                                                 |
| --------------- | ------------------------------------------------------ |
| **评分校准**        | maintainability=45 但代码不差 → 校准阈值和权重，分数与直觉一致           |
| **检测降噪**        | 20% 噪声（healthy churn、测试代码误报）→ 精准过滤降至 <5%           |
| **处方深化**        | "split src/trend.rs" 太模糊 → "按 velocity/regression/forecast 拆分" |
| **agent 摘要**    | 115 条 JSON 信息过载 → 分层摘要：1 句话 + top 3 + 详情            |
| **反馈验证**        | 修了不知道有没有效 → before/after 对比 + 改善报告                  |

### 排除

- 自动执行修复（v8+ — 有了精准处方 + 反馈验证后再做自动化）
- 多项目聚合（v8+ — Portfolio Health）
- 自定义规则/配置（内置规则先打磨到位）
- UI/Dashboard（CLI + MCP 已满足需求）

## 4. 技术方向

### M1 评分校准

```
当前阈值审计：
  → 用 decay 自身 + 2-3 个已知质量的开源项目回测
  → 调整每个维度的 WARN/CRIT 阈值
  → 调整 ScoreProfile 权重
  → 验证：校准后分数与代码质量直觉一致
```

关键观察：
- maintainability 45 主要因为长函数（dimension evaluate）和重复代码（测试模式），但这些是合理的
- performance 50 主要因为嵌套循环（检测逻辑本身需要遍历）和 clone（测试代码）
- 需要区分"真问题"和"结构性固有复杂度"

### M2 检测降噪

```
噪声源分析：
  1. fragility churn — 快速开发期的 healthy churn（15 issues）
     → 引入 churn 类型判断：项目 age < 30 天或 commit 密度 > 5/天 → 降级
  2. performance blocking call — 测试代码中的模式字符串（7 issues）
     → 扩展 FileContext.Test 过滤到 performance 维度
  3. 其他维度的测试代码污染
     → 统一：所有维度的 analyze() 跳过 test context 文件
```

### M3 处方深化

```
当前：Action { suggestion: "split src/trend.rs to isolate unstable logic" }
目标：Action {
  suggestion: "split by responsibility",
  details: [
    "extract velocity calculation → src/trend/velocity.rs",
    "extract regression detection → src/trend/regression.rs",  
    "extract shared math → src/trend/math.rs",
  ]
}
```

需要 analyze 阶段产出更多结构信息（函数列表、依赖关系），处方引擎基于结构信息生成具体方案。

### M4 agent 摘要

```
MCP 返回结构：
{
  "summary": "Health 83/100, declining (↓3). 2 security issues need immediate attention.",
  "top_actions": [
    { "priority": "critical", "what": "fix SQL injection in db.rs:42", "effort": "small" },
    { "priority": "high", "what": "extract duplicated code in 13 files", "effort": "medium" },
    { "priority": "high", "what": "add cargo-deny for dependency audit", "effort": "small" }
  ],
  "full_report": { ... }  // 现有完整 JSON
}
```

### M5 反馈验证

```
decay --compare <snapshot_id>
  → 对比当前快照与指定快照
  → 输出改善报告：
    "maintainability: 45 → 65 (+20) ✅ improved"
    "performance: 50 → 55 (+5) → improved"
    "composite: 83 → 87 (+4)"
    "Issues resolved: 12 / New issues: 3"
```

## 5. 风险与依赖

| 风险               | 影响             | 缓解                              |
| ---------------- | -------------- | ------------------------------- |
| 校准后分数大幅变动       | 历史快照对比失真       | 新旧阈值并行输出一段时间                   |
| 处方深化依赖代码结构分析    | 分析不准导致建议不对     | 仅在结构信息充分时输出详细建议，否则回退到通用建议    |
| agent 摘要丢失关键信息   | agent 漏掉重要 issue | top_actions 保证覆盖所有 Critical     |
| 反馈验证的时间跨度       | 快照间隔太短看不到改善    | 支持指定任意历史快照对比，不限于相邻              |

## 6. 后续版本方向（存档）

### v8+ — Auto-execute + Portfolio Health

- 自动执行 A 类 patch（生成 → apply → cargo check → 提 PR）
- C 类架构决策辅助（调研行业标准做法，推荐方案）
- 多项目健康聚合：组织级仪表盘
- 跨项目模式发现：所有项目的共性衰退趋势
- 团队级预警：组织范围内的健康阈值管理
