# 趋势分析 技术方案

## 1. 背景

完整的时间维度分析体系：从 2 点对比到时间序列查询，到变化率计算，到回归检测，到阈值预警，到维度相关性分析，到统一轨迹报告。

### 技术约束

- 对比目标: 同 project_path 的上一个快照（自动匹配）
- dimension_scores 表: 已有快照维度分数存储，复用即可，无需新建表
- 线性回归: 使用序号（0,1,2...）而非 snapshot_id 作为 x 轴，避免 ID 间隔不均导致斜率失真
- 统计方法: 使用总体标准差（除以 N），适合小样本快照序列
- 阈值: 默认 60 分，硬编码
- Pearson 系数: 要求两维度在同一快照中都有值，缺失值的快照跳过
- Report: 保留原有零散字段向后兼容，trajectory 为 `Option<Trajectory>`

### 前置依赖

- snapshot-store — 已完成
- composite-score — 已完成

## 2. 方案

### 文件/模块结构

| 文件 | 职责 |
|------|------|
| `src/trend.rs` | compare 函数 + Trend/Delta 类型 + Velocity/Direction + linear_regression_slope() + Regression/RegressionSeverity + std_dev() + Forecast + r_squared() + forecast_breaches() + Correlation/CorrelationStrength + pearson_correlation() + Trajectory + build_trajectory() |
| `src/db.rs` | get_previous_scores 查询 + SnapshotScores 类型 + get_dimension_time_series() |
| `src/run.rs` | Report 集成：trend/time_series/velocities/regressions/forecasts/correlations/trajectory 字段 |
| `src/render.rs` | terminal + markdown 渲染：趋势变化、velocity、回归警告、预警信息、相关性、统一 "Health Trajectory" 段落 |

### 核心流程

**2 点对比**:
1. `db::get_previous_scores()` → 查询同 project_path 上一个快照 → Option\<Scores\>
2. `trend::compare()` → 逐维度计算 current - previous → Delta (Up/Down/Unchanged/NA)

**时间序列**:
1. `get_dimension_time_series()` → 查询最近 N 个快照及其维度分数 → `Vec<SnapshotScores>`（按 snapshot_id 升序）
2. `dimension_series()` → 从 SnapshotScores 序列提取单维度 (snapshot_id, score) 对 → 跳过 None 值

**衰退速度**:
1. `linear_regression_slope()` → 最小二乘法回归 → 斜率（<2 点返回 None，全 x 相同返回 0.0）
2. `calculate_velocities()` → 遍历所有维度调用回归 → `Vec<Velocity>`（<3 数据点跳过，按字母序排列）
3. 方向映射 → 斜率 > 1.0 为 Improving(↑)，< -1.0 为 Declining(↓)，其余为 Stable(→)

**回归检测**:
1. `std_dev()` → 计算数值序列总体标准差 → σ（空序列返回 0.0）
2. `detect_regressions()` → 对每个维度提取分数序列，计算相邻差值的 σ → 最新差值超过 k×σ 标记回归
3. 严重度判定 → |last diff| > 2k×σ → Severe，否则 Moderate → σ=0 时任何下降都标记回归

**阈值预警**:
1. `r_squared()` → 线性回归决定系数 R² → 0.0~1.0（<2 点返回 None，SS_tot=0 返回 1.0）
2. `forecast_breaches()` → ≥5 点 + slope < 0 + R² > 0.7 + 当前未跌破 → 计算 snapshots_until_breach
3. 外推公式 → `ceil((threshold - current_score) / slope).abs()` → 按 snapshots_until_breach 升序

**维度相关性**:
1. `pearson_correlation()` → 两个分数数组 Pearson r → 相关系数（<2 点返回 None，方差为 0 返回 0.0）
2. `analyze_correlations()` → 所有有序维度对 (a < b) 提取共同快照分数 → 过滤 <5 点和 |r| ≤ 0.4
3. 按 |r| 降序排列 → 最强相关性优先

**轨迹报告**:
1. `build_trajectory()` → 调用 calculate_velocities/detect_regressions/forecast_breaches/analyze_correlations → Trajectory 聚合
2. overall_direction → composite 维度的 velocity direction → 无 composite 则默认 Stable
3. Report → snapshot_count ≥ 3 时填充 trajectory → markdown 合并为统一段落

### 数据结构

**SnapshotScores**:

| 字段 | 类型 | 用途 |
|------|------|------|
| snapshot_id | i64 | 快照唯一标识 |
| created_at | String | 快照创建时间 |
| scores | HashMap<String, Option<i32>> | 各维度分数 |

**Velocity**:

| 字段 | 类型 | 用途 |
|------|------|------|
| dimension | String | 维度名称 |
| slope | f64 | 线性回归斜率（分/快照） |
| direction | Direction | Improving(↑)/Declining(↓)/Stable(→) |
| data_points | usize | 参与计算的数据点数 |

**Regression**:

| 字段 | 类型 | 用途 |
|------|------|------|
| dimension | String | 回归维度名称 |
| previous_score | i32 | 前一快照分数 |
| current_score | i32 | 当前快照分数 |
| drop | i32 | 下降幅度 |
| threshold | f64 | 触发阈值（k×σ） |
| severity | RegressionSeverity | Moderate 或 Severe |

**Forecast**:

| 字段 | 类型 | 用途 |
|------|------|------|
| dimension | String | 维度名称 |
| current_score | i32 | 当前分数 |
| slope | f64 | 线性回归斜率 |
| r_squared | f64 | 决定系数 |
| threshold | i32 | 健康阈值 |
| snapshots_until_breach | u32 | 预计多少个快照后跌破 |

**Correlation**:

| 字段 | 类型 | 用途 |
|------|------|------|
| dim_a | String | 维度 A 名称 |
| dim_b | String | 维度 B 名称 |
| coefficient | f64 | Pearson 相关系数 |
| strength | CorrelationStrength | Strong(\|r\|>0.6) 或 Moderate(\|r\|>0.4) |
| data_points | usize | 共同数据点数 |

**Trajectory**:

| 字段 | 类型 | 用途 |
|------|------|------|
| overall_direction | Direction | 项目整体健康方向 |
| snapshot_count | usize | 参与分析的快照数 |
| velocities | Vec\<Velocity\> | 各维度变化率 |
| regressions | Vec\<Regression\> | 回归事件 |
| forecasts | Vec\<Forecast\> | 阈值预警 |
| correlations | Vec\<Correlation\> | 维度相关性 |

## 3. 关键决策

| 决策 | 选择 | 为什么 |
|------|------|--------|
| 对比目标 | 同 project_path 上一个快照 | 自动匹配，无需手动指定 |
| 存储方案 | 复用 dimension_scores 表 | 数据已存在，新建表导致冗余 |
| 排序方向 | 查询 DESC + 结果翻转为 ASC | 数据库按 DESC 取最近 N 条效率最高 |
| API 粒度 | 返回全维度 SnapshotScores | 一次查询全返回减少 IO |
| x 轴取值 | 序号（0,1,2...） | snapshot_id 间隔不均匀会导致斜率失真 |
| 方向阈值 | ±1.0 分/快照 | 低于 1 分的变化属正常波动 |
| 最少数据点（velocity） | ≥3 个快照 | 2 点回归永远完美拟合无统计意义 |
| 统计方法（回归） | 差值序列标准差 | 直接衡量"正常波动幅度"，语义清晰 |
| k 值 | 固定 k=2 | 2σ 是统计学常用异常检测阈值 |
| σ=0 处理 | 任何下降都标记回归 | 历史完全稳定时任何下降都异常 |
| 置信度过滤 | R² > 0.7 | 过滤线性拟合差的维度避免噪声预警 |
| 最少数据点（预警） | ≥5 个快照 | 少于 5 点的 R² 统计意义不足 |
| 外推方法 | 线性外推 | 简单可解释，足以提供有用预警 |
| 相关性方法 | Pearson 相关系数 | 线性相关检测，简单高效 |
| 强度阈值 | \|r\| > 0.4 输出，> 0.6 标注强 | 统计学常用分级标准 |
| 最少共同点 | ≥5 个共同快照 | 少于 5 点容易偶然高相关 |
| 维度对遍历 | 有序对 (a < b) | 避免重复计算 |
| 兼容策略 | 保留原有零散字段 + 新增 trajectory | 不破坏已有 JSON 消费方 |
| overall direction 来源 | composite 维度的 velocity | composite 是加权综合分，最能代表整体 |
| trajectory 生成条件 | ≥3 个快照 | 少于 3 个快照统计意义不足 |

## 4. 迭代记录

### 2026-04-14

- 初始方案：2 点对比，compare 函数 + Delta 类型

### 2026-04-15

- 趋势引擎：复用 dimension_scores 表，新增时间序列查询 API + Report 集成
- 衰退速度：线性回归斜率 + Improving/Declining/Stable 方向标签 + 渲染集成
- 回归检测：差值标准差 + k=2 + Moderate/Severe 两级严重度
- 阈值预警：R² > 0.7 过滤 + 线性外推预测阈值突破
- 维度相关性：Pearson 相关 + Strong/Moderate 强度分级
- 轨迹报告：Trajectory 聚合 + overall direction + 统一渲染
