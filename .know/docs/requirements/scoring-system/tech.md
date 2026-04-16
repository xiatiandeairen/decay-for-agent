# 评分体系 技术方案

## 1. 背景

PRD 要求基于采集数据计算多维度 0-100 分数并合成 composite 分，同时将维度抽象为统一 trait 支持扩展，并按项目类型差异化权重。

### 技术约束

- 评分范围: 0-100，扣分制
- structural/complexity 数据源: files 表（file-scan 模块产出）
- fragility 数据源: git_changes 表（git-analysis 模块产出）
- 无 git 数据时 fragility 返回 None（不报错）
- composite 合成方式: 加权平均，N/A 维度不参与
- Dimension trait: 必须 `Send + Sync`，支持并发
- DB: 新增 key-value 表存储动态维度分数
- 项目类型检测: 只读文件系统，不执行命令
- profile 影响范围: v3 仅影响 composite 权重，不改各维度内部阈值

### 前置依赖

- file-scan — 已完成
- git-analysis — 已完成

## 2. 方案

### 文件/模块结构

**评分函数:**

| Action | File | Responsibility |
|--------|------|---------------|
| create | `src/score.rs` | structural / complexity / fragility / composite 评分函数 |
| modify | `src/main.rs` | 集成输出 Health 行 |

**Dimension trait 重构后:**

```
src/dimension/
├── mod.rs              # trait 定义 + DimensionResult + all_dimensions() 注册表
├── structural.rs       # structural 评分 + 诊断 + 阈值常量 + 测试
├── complexity.rs       # complexity 评分 + 诊断 + 阈值常量 + 测试
└── fragility.rs        # fragility 评分 + 诊断 + 阈值常量 + 测试
```

| 文件 | 职责 |
|------|------|
| `src/dimension/mod.rs` | Dimension trait + DimensionResult + all_dimensions() |
| `src/dimension/structural.rs` | structural 评分 + 诊断 + 阈值常量 + 测试 |
| `src/dimension/complexity.rs` | complexity 评分 + 诊断 + 阈值常量 + 测试 |
| `src/dimension/fragility.rs` | fragility 评分 + 诊断 + 阈值常量 + 测试 |
| `src/diagnose.rs` | Category → String，删除 run()，保留 Level/Issue/print_issues |
| `src/run.rs` | Scores → HashMap，用注册表循环替代硬编码 |
| `src/db.rs` | 新增 dimension_scores 表 + 动态读写函数 |
| `src/trend.rs` | Trend → HashMap，动态维度对比 |
| `src/score.rs` | 仅保留 composite()，删除 3 个维度函数 |

**分层打分:**

| 文件 | 职责 |
|------|------|
| `src/profile.rs` | ProjectType enum + detect() 检测函数 + ScoreProfile 数据结构 |
| `src/run.rs` | 调用 detect()，composite 改用加权 |

### 核心流程

**structural 评分:**
1. 查询 files 表 → 聚合文件数/最大深度/顶层目录数 → 结构指标
2. 逐项比对阈值 → 超阈值扣分 → 累计扣分
3. 100 - 累计扣分 → clamp 到 0-100 → structural 分数

**complexity 评分:**
1. 查询 files 表 → 计算大文件占比/平均大小/最大大小 → 复杂度指标
2. 逐项比对阈值 → 超阈值扣分 → 累计扣分
3. 100 - 累计扣分 → clamp 到 0-100 → complexity 分数

**fragility 评分:**
1. 查询 git_changes 表 → 计算 churn 集中度和 top churn 文件 → 脆弱性指标
2. 逐项比对阈值 → 超阈值扣分 → 累计扣分
3. 100 - 累计扣分 → clamp 到 0-100 → fragility 分数（或 None）

**composite 合成:**
1. 收集所有维度分数 → 过滤 N/A 维度 → 有效分数列表
2. 等权平均（或 ScoreProfile 加权） → composite 分数
3. 格式化输出 → `Health: X/100 (structural: Y, ...)` → 终端展示

**Dimension trait 调度:**
1. `run.rs` 调用 `all_dimensions()` 获取所有注册维度列表
2. 遍历维度列表，对每个维度调用 `evaluate()` → 产出 `DimensionResult`
3. `db.rs` 将动态维度分数写入 `dimension_scores` 表（key-value 模式）
4. 输出层从 `Vec<DimensionResult>` 动态生成

**分层打分:**
1. `run.rs` 调用 `profile::detect()` → 基于文件特征匹配项目类型
2. `ScoreProfile::for_type()` 加载对应类型的权重表 → 产出 `ScoreProfile`
3. composite 使用 `score_profile.weighted_composite(&scores)` → 加权平均

### 数据结构

**structural 阈值:**

| 指标 | 阈值 | 扣分 |
|------|------|------|
| 文件数 | >500 | -20 |
| 文件数 | >1000 | -40（替代） |
| 目录深度 | >5 | -15 |
| 目录深度 | >8 | -30（替代） |
| 顶层目录数 | >15 | -15 |

**complexity 阈值:**

| 指标 | 阈值 | 扣分 |
|------|------|------|
| 大文件（>15KB）占比 | >20% | -25 |
| 大文件占比 | >40% | -45（替代） |
| 平均文件大小 | >10KB | -15 |
| 最大文件 | >50KB | -10 |

**fragility 阈值:**

| 指标 | 阈值 | 扣分 |
|------|------|------|
| top 10% 文件承担 churn | >50% | -25 |
| top 10% 文件承担 churn | >70% | -45（替代） |
| 最高 churn 文件变更行数 | >500 | -15 |

**DimensionResult:**

| 字段 | 类型 | 用途 |
|------|------|------|
| name | String | 维度名称 |
| score | Option\<i32\> | 评分 0-100，None 表示数据源不可用 |
| issues | Vec\<Issue\> | 诊断问题列表 |

**dimension_scores 表:**

| 字段 | 类型 | 用途 |
|------|------|------|
| dimension | TEXT | 维度名（含 "composite"） |
| score | INTEGER | 分数 |

**ScoreProfile:**

| 字段 | 类型 | 用途 |
|------|------|------|
| ProjectType | enum (Cli/WebService/Library/MobileApp/Monorepo/Generic) | 项目类型标识 |
| project_type | ProjectType | 当前检测到的类型 |
| weights | HashMap\<String, f64\> | 维度名 → 权重 0.0-1.0 |

## 3. 关键决策

| 决策 | 选择 | 为什么 |
|------|------|--------|
| 算法 | 扣分制 | 简单直观，阈值可解释；备选加权评分需要归一化且不易理解 |
| 阈值来源 | 经验值 | v1 先用固定阈值，后续可调；备选统计分布需大量项目样本 |
| 大文件定义 | >15KB | 约等于 500 行代码；备选 10KB 过于严格，20KB 过于宽松 |
| 不用 AST | 启发式 | PRD 排除语言级分析；备选 tree-sitter 增加依赖且需多语言支持 |
| 无 git 数据 | 返回 None | 不报错，composite 跳过该维度；备选返回 0 会错误惩罚无 git 项目 |
| churn 定义 | lines_added + lines_deleted | 标准 churn 指标；备选只算 commit 数忽略变更规模 |
| 加权方式 | 等权（默认）/ ScoreProfile 加权 | 简单起步，v3 按项目类型差异化 |
| N/A 处理 | 不参与合成 | 不因缺失数据惩罚；备选设默认值会扭曲结果 |
| trait 方法签名 | score + diagnose 分离 | 允许只取分数而不运行诊断；备选"单一 evaluate()"会强制执行诊断 |
| Category 类型 | String（非 enum） | enum 无法在不修改定义处的情况下扩展 |
| DB schema | 新增 key-value 表 | 固定列无法支持动态维度数量 |
| 旧 scores 表 | 保留不删 | 向后兼容历史数据读取 |
| evaluate 默认实现 | trait 提供 default impl | 减少每个维度的样板代码 |
| 检测方式 | 文件特征，不执行命令 | 快速、无副作用 |
| profile 影响范围 | 仅 composite 权重 | 最小改动，避免破坏所有维度实现 |
| Dimension trait | 不扩展签名 | 避免破坏所有维度实现 |
| 权重配置 | 代码硬编码 | v3 不做用户可配，先用合理默认值 |
| 检测 fallback | Generic（等权重） | 误检测比不检测安全 |

## 4. 迭代记录

### 2026-04-14

- 分层打分系统方案
- Dimension trait 统一注册方案
- composite 合成方案
- fragility 评分方案
- complexity 评分方案
- structural 评分方案
