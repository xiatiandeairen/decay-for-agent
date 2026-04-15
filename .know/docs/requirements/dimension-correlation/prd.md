# 维度相关性 — 跨维度联动模式发现

## 1. 问题

各维度独立分析，错过联动模式。例如 complexity 上升时 maintainability 通常下降，但当前系统无法发现这种关联，导致 agent 可能修复症状而忽略根因维度。

## 2. 目标用户

- AI agent：利用相关性优先处理根因维度，避免修复症状
- 开发者：了解维度间的联动关系
- v5 M6 轨迹报告：相关性作为轨迹报告的组成部分

## 3. 核心假设

**Pearson 相关系数 + 强度分级 → 准确发现维度间联动模式。**

## 4. 方案

- 对所有维度对计算 Pearson 相关系数
- |r| > 0.6 标注强相关，|r| > 0.4 标注中等，其余弱相关（不输出）
- 需要 ≥5 个共同数据点
- Report JSON 新增 `correlations` 字段

### 任务

| 任务 | 文档 | 进度 |
|------|------|------|
| dimension-correlation tech | [tech](tech.md) | 0/0 |

## 5. 验收标准

- `pearson_correlation()` 正确计算相关系数，< 2 个点返回 None
- `analyze_correlations()` 仅输出 |r| > 0.4 且 ≥5 共同数据点的维度对
- 正相关和负相关都能检测
- 不输出维度自身的相关性（r=1.0）
- `--json` 输出包含 `correlations`（有数据时）
- terminal/markdown 展示相关性信息
- 新增测试：Pearson 计算、完美正/负相关、弱相关不输出、不足数据
- `cargo test` 全部通过

## 6. 排除项

- 不做因果推断（相关不等于因果）
- 不做滞后相关（lag correlation，v6+ 考虑）
- 不做偏相关分析（partial correlation）
