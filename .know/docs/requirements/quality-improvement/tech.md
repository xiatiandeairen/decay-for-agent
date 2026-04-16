# 质量提升 技术方案

## 1. 背景

提升 decay 自身代码质量和检测准确性：测试加固 + 调试能力、检测精度（消除误报）、问题分类引擎。

### 技术约束

- CLI 输出: `--debug` 日志必须输出到 stderr，避免污染 stdout（尤其 `--json` 模式）
- 日志框架: 零配置启用，不增加用户认知负担
- helpers.rs: 已有文件解析辅助函数，test block 检测逻辑在此实现
- count_pattern_matches: 已有 pattern 匹配函数，需新增 `test_mask` 参数支持跳过测试行
- diagnose.rs: 已有 Issue 结构体，新增 `classification` 字段
- classify.rs: 新模块，规则引擎基于 (dimension, message pattern, action_type, level) 匹配

### 前置依赖

- cli-framework — 已完成
- structural-score / complexity-score / fragility-score — 已完成
- v3 observability / reliability 维度 — 已完成
- v4 action-schema / prescription-engine — 已完成

## 2. 方案

### 文件/模块结构

| 文件 | 职责 |
|------|------|
| `src/cli.rs` | 新增 `--debug` flag，设置 `RUST_LOG=debug` |
| `src/main.rs` | `env_logger::init()` 初始化 + `mod classify` 注册 |
| `src/score/*.rs` | 关键路径加 `log::debug!` |
| `tests/` | 补充边界测试用例 |
| `src/dimension/helpers.rs` | `mark_test_lines()` 函数 + tests |
| `src/dimension/observability.rs` | 传入 test mask 跳过测试代码 |
| `src/dimension/reliability.rs` | SQL injection 假阳性过滤 + test |
| `src/diagnose.rs` | IssueCategory 枚举 + Issue.classification 字段 |
| `src/classify.rs` | `classify_issues()` + `classify()` 规则函数 + 9 tests |
| `src/run.rs` | 调用 `classify_issues()` |
| `src/render.rs` | markdown 输出分类标签 |

### 核心流程

**测试加固 + Debug**:
1. 用户传入 `--debug` → clap 解析后设置 `RUST_LOG=debug`
2. `env_logger::init()` → 根据环境变量决定日志级别
3. 各模块 `log::debug!` → db init、scan 进度、git 分析、评分计算、诊断规则 → stderr

**检测精度**:
1. `mark_test_lines(lines)` → 追踪 `#[cfg(test)]` mod 和 `#[test]` fn 的大括号范围 → `Vec<bool>`
2. observability `count_pattern_matches` → 新增 `test_mask: Option<&[bool]>` → 测试行跳过 unwrap 计数
3. reliability SQL injection → 检查行是否包含 `bail!/anyhow!/panic!/eprintln!/error!/warn!` 或 `failed/error/unable/could not` → 匹配则排除

**问题分类**:
1. `classify()` → 按优先级匹配规则（SecurityCritical → MechanicalFix → ContextualException → Prevention → PatternProblem → ChronicDecay → ConventionDrift → ArchitecturalDecision）→ 返回 IssueCategory
2. `classify_issues()` → 遍历所有 issue 调用 `classify()` → 填充 classification 字段
3. 渲染集成 → JSON serde 自动序列化 + markdown 展示分类标签

### 数据结构

**补充测试覆盖**:

| 模块 | 边界用例 |
|------|----------|
| score | 空快照、满分、零分 |
| diagnose | 无文件、无 git、全健康 |
| trend | 无历史、有历史、分数不变 |

**分类规则优先级**:

| 优先级 | 类别 | 匹配条件 |
|--------|------|----------|
| 1 | D: SecurityCritical | injection/credential 关键词 |
| 2 | A: MechanicalFix | per-file unwrap/catch/hardcoded |
| 3 | G: ContextualException | unsafe/eval |
| 4 | H: Prevention | dependencies/blocking |
| 5 | B: PatternProblem | duplicate/clone density |
| 6 | F: ChronicDecay | TODO/ratio/Info-level |
| 7 | E: ConventionDrift | no logging/no tests/low ratio |
| 8 | C: ArchitecturalDecision | Split/Refactor/structural |
| 9 | Default | C（most conservative） |

## 3. 关键决策

| 决策 | 选择 | 为什么 |
|------|------|--------|
| 日志框架 | log + env_logger | 轻量适合 CLI；备选 tracing 功能更强但对 CLI 过重 |
| debug 输出目标 | stderr | 不污染 stdout，尤其 `--json` 模式 |
| test block 检测方式 | 大括号计数追踪 | 不依赖语法树，轻量且对格式规范的 Rust 代码足够准确；备选 syn 解析引入重依赖 |
| test mask 集成方式 | Optional 参数 | 只有 observability 需要，其他维度传 None 零成本 |
| SQL 误报处理 | 关键词排除 | 错误消息有明显词汇特征，规则简单有效 |
| 分类方法 | 规则引擎（关键词 + 优先级） | issue 数量有限（55 种），规则可解释可调试；备选 ML 分类器需训练数据且黑箱 |
| 分类数量 | 8 类 | 覆盖从 MechanicalFix 到 Prevention 的完整频谱 |
| 默认分类 | ArchitecturalDecision | 最保守选择，未匹配规则的 issue 需要人工判断 |

## 4. 迭代记录

### 2026-04-14

- 测试加固：log + env_logger，`--debug` 设置环境变量，关键路径加 debug 日志，边界测试补充

### 2026-04-16

- 检测精度：test block 大括号追踪 + Optional test mask + SQL 关键词排除
- 问题分类引擎：8 类 IssueCategory 枚举 + 优先级规则引擎 + 渲染集成
