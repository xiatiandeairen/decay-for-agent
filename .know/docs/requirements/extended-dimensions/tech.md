# 扩展维度 技术方案

## 1. 背景

PRD 要求新增 5 个维度（maintainability / observability / quality_assurance / reliability / performance），均基于文件内容分析，复用 files 表路径 + 直接读文件系统，不新增 collector 和 DB 表。

### 技术约束

- 数据源: 从 DB files 表获取路径 + 直接读文件系统，不新增 collector 和 DB 表
- 检测方式: 行级哈希、正则匹配、关键词 grep（语言无关）
- 评分范围: 0-100，扣分制
- 所有维度实现 Dimension trait，通过注册表统一调度

### 前置依赖

- dimension-trait（M1） — 已完成
- collector-plugin（M2） — 已完成

## 2. 方案

### 文件/模块结构

| 文件 | 职责 |
|------|------|
| `src/dimension/maintainability.rs` | Maintainability 维度（重复代码、长文件/函数、TODO/FIXME） |
| `src/dimension/observability.rs` | Observability 维度（日志/错误处理密度、panic/unwrap、硬编码配置） |
| `src/dimension/quality.rs` | QualityAssurance 维度（测试比例、测试/源码行数比、断言密度） |
| `src/dimension/reliability.rs` | Reliability 维度（unsafe/eval、SQL/shell 注入、硬编码密钥、依赖数量） |
| `src/dimension/performance.rs` | Performance 维度（嵌套循环、clone/copy、同步阻塞调用） |
| `src/dimension/mod.rs` | 注册所有 5 个新维度 |
| `src/profile.rs` | ScoreProfile 添加 5 个新维度的权重 |

### 核心流程

所有 5 个维度遵循相同模式：
1. `Xxx::score()` 从 DB files 表读取文件列表 → 还原绝对路径
2. 读取文件内容 → 按维度特定方式检测（详见下文） → 产出各指标值
3. 基于扣分制（100 分起）计算最终分数 → 产出 `Option<i32>` 评分和 `Vec<Issue>` 诊断

**maintainability 检测:**
- 行级哈希检测重复块（连续 6 行块指纹）
- 统计行数检测长文件（>300 行）和长函数（>50 行）
- grep TODO/FIXME 关键词

**observability 检测:**
- grep 日志调用（log/logger/println/console.log）
- grep 错误处理（try/catch/Result）
- grep panic/unwrap 调用
- grep 硬编码配置模式

**quality_assurance 检测:**
- 文件名模式匹配（*_test.* / test_* / *_spec.* / tests/）分类测试和源码
- 统计测试文件比例和测试/源码行数比
- grep 断言关键词（assert/expect/should/toBe）计算断言密度

**reliability 检测:**
- grep unsafe/eval 调用
- 正则匹配 SQL/shell 注入模式和硬编码密钥模式
- 解析依赖文件（Cargo.toml / package.json 等）统计直接依赖数

**performance 检测:**
- 缩进/括号跟踪检测嵌套循环（for/while 嵌套 >=3 层）
- grep clone/copy 调用
- grep 同步阻塞调用（sleep/sync HTTP）

### 数据结构

**maintainability 阈值:**

| 指标 | 阈值 | 扣分 |
|------|------|------|
| 重复代码比例 >5% 文件有显著重复块 | 5% | -15 |
| 重复代码比例 | >15% | -35（替代） |
| 长文件比例（>300行） | >20% | -15 |
| 长文件比例（>300行） | >40% | -30（替代） |
| 长函数比例（>50行） | >10% | -10 |
| 长函数比例（>50行） | >25% | -20（替代） |
| TODO/FIXME 密度 | >20/10K 行 | -5 |

**observability 阈值:**

| 指标 | 阈值 | 扣分 |
|------|------|------|
| unwrap/panic 密度 | >5 per 1K lines | -15 |
| unwrap/panic 密度 | >15 per 1K lines | -30（替代） |
| 无日志框架引用 | 源文件中无 log/logger 调用 | -20 |
| 错误吞没比例 | catch/except 块中无处理 >20% | -15 |
| 硬编码配置 | >5 处 | -10 |

**quality_assurance 阈值:**

| 指标 | 阈值 | 扣分 |
|------|------|------|
| 测试文件比例 | 0% | -40 |
| 测试文件比例 | <10% | -25 |
| 测试文件比例 | <20% | -10 |
| 测试/源码行数比 | <0.1 | -20 |
| 测试/源码行数比 | <0.3 | -10 |
| 断言密度 | <1 per 20 test lines | -10 |

**reliability 阈值:**

| 指标 | 阈值 | 扣分 |
|------|------|------|
| unsafe/eval 密度 | >2 per 1K lines | -15 |
| unsafe/eval 密度 | >8 per 1K lines | -30（替代） |
| SQL/shell 注入模式 | any | -20 per occurrence (max -40) |
| 硬编码密钥/密码模式 | any | -15 per occurrence (max -30) |
| 依赖数量 | >50 直接依赖 | -10 |
| 依赖数量 | >100 直接依赖 | -20（替代） |

**performance 阈值:**

| 指标 | 阈值 | 扣分 |
|------|------|------|
| 深层嵌套循环（>=3层） | >3 occurrences | -15 |
| 深层嵌套循环（>=3层） | >10 occurrences | -30（替代） |
| clone/copy 密度 | >10 per 1K lines | -10 |
| clone/copy 密度 | >25 per 1K lines | -20（替代） |
| 同步阻塞调用 | >5 occurrences | -10 |
| 同步阻塞调用 | >15 occurrences | -20（替代） |

## 3. 关键决策

| 决策 | 选择 | 为什么 |
|------|------|--------|
| 是否新增 collector | 否，直接读文件内容 | 分析轻量级，无需持久化中间结果 |
| 重复检测算法 | 行级哈希（连续 6 行块指纹） | 简单可靠、语言无关；备选"AST 级检测"需要语言解析器 |
| 函数长度检测 | 正则匹配函数定义 | 启发式足够，支持多语言 |
| 最小重复块大小 | 6 行 | 平衡精度和误报 |
| 日志检测范围 | log/logger/println/console.log 等通用关键词 | 覆盖主流语言 |
| 测试文件识别 | 文件名模式匹配 | 覆盖主流语言约定（_test / test_ / _spec / tests/） |
| 质量指标 | 断言密度 | 简单有效衡量测试质量 |
| 安全检测方式 | 关键词 grep + 正则 | 语言无关、实现简单 |
| 依赖分析 | 解析依赖文件统计数量 | 静态可行、无需网络 |
| SQL 注入检测 | 字符串拼接模式匹配 | 覆盖常见模式 |
| 嵌套循环检测 | 缩进/括号跟踪 | 不需要 AST，适度准确 |
| clone/copy 检测 | 关键词 grep | 简单有效覆盖 .clone()/.copy()/.deepcopy() |

## 4. 迭代记录

### 2026-04-14

- maintainability 维度方案
- observability 维度方案
- quality_assurance 维度方案
- reliability 维度方案
- performance 维度方案
