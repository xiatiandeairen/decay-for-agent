# 诊断处方 技术方案

## 1. 背景

从评分和采集数据中自动识别具体问题、生成分级诊断、输出结构化可执行处方。涵盖诊断规则、文本处方、Action Schema 类型系统、全维度迁移、位置精度提升、排序去重。

### 技术约束

- 诊断规则: 硬编码，约 10 条诊断规则，分级 critical / warning / info 三级
- 处方生成: 规则驱动模板（排除 LLM 生成），仅 critical/warning 级别
- Action 类型系统: 必须 derive `Serialize`，JSON 输出格式稳定，不可破坏现有 CLI 文本输出
- 向后兼容: Issue 新增字段用 `skip_serializing_if` 保证无 action 时 JSON 输出不变；prescription 字段保留双写过渡
- 构造函数: `Issue::new()` 和 `Issue::with_actions()` 必须覆盖所有 Issue 创建场景
- 位置数据来源: 只能从现有 analyze 函数的采集数据中提取行号，不引入 AST 解析
- Effort 枚举: 必须 derive `PartialOrd + Ord`，变体定义顺序决定排序结果（Small < Medium < Large）
- dedup_by: 要求输入已排序，相邻同类 action 才能正确去重

### 前置依赖

- file-scan — 已完成
- git-analysis — 已完成
- structural-score — 已完成
- complexity-score — 已完成
- fragility-score — 已完成

## 2. 方案

### 文件/模块结构

| 文件 | 职责 |
|------|------|
| `src/diagnose.rs` | 诊断规则 + 处方模板 + Issue 构造函数（`new()` + `with_actions()`） |
| `src/action.rs` | Action/ActionType/Priority/Effort/Target 定义 + Serialize + Display |
| `src/main.rs` | 集成诊断输出 + `mod action` |
| `src/run.rs` | Report 新增 `actions`，收集+排序+去重，markdown 渲染 |
| `src/dimension/structural.rs` | Action POC（3 个 Issue 附带 Action）+ 构造函数迁移 |
| `src/dimension/complexity.rs` | 添加 Action + 构造函数迁移 |
| `src/dimension/fragility.rs` | 添加 Action + 构造函数迁移 |
| `src/dimension/maintainability.rs` | 添加 Action + 构造函数 + long_func_details 扩展 start_line → line_range + symbol |
| `src/dimension/observability.rs` | 添加 Action + 构造函数 + unwrap_details 扩展 Vec<u32> → line_range |
| `src/dimension/reliability.rs` | 添加 Action + 构造函数 + injection/secret_details 扩展 line_no → line_range |
| `src/dimension/performance.rs` | 添加 Action + 构造函数 + nest line_no 直接填入 line_range |
| `src/dimension/quality.rs` | 添加 Action + 构造函数 |
| `tests/action_schema.rs` | Action 序列化 roundtrip 测试 |

### 核心流程

**诊断**:
1. 遍历规则集 → 逐条查询 files/git_changes 表 → 匹配结果
2. 匹配成功 → 生成 Issue（含 level/category/message/prescription/actions）
3. 按 level 排序 → critical > warning > info → 分级输出

**处方**:
1. 诊断规则匹配 → 确定问题类型和级别
2. critical/warning → 查找处方模板 → 填充具体文件名等上下文
3. 输出 Issue.prescription → 用户可读的重构建议

**Action Schema**:
1. `action.rs` 定义 ActionType(7 种)、Priority(4 级)、Effort(3 级)、Target、Action → 完整类型系统
2. 8 个维度文件将 struct literal 改为构造函数调用，Warning/Critical 级使用 `with_actions()` → 附带结构化 Action
3. `run.rs` 从 all_issues 收集 actions、排序+去重 → Report 顶层 actions 数组供 agent 消费

**位置精度分层**:
1. A 层（直接填充）: performance `nest_details` 已有 `(path, line_no, depth)` → 直接填入 line_range
2. A 层: maintainability `long_func_details` 扩展为 `(path, func_name, func_len, start_line)` → line_range + symbol
3. B 层（扩展采集）: observability `unwrap_details` 扩展为 `(path, count, Vec<u32>)` → line_range
4. B 层: reliability `injection/secret_details` 扩展为 `(path, pattern, line_no)` → line_range
5. C 层（保持现状）: structural/complexity/fragility/quality → line_range: None

**排序去重**:
1. Effort 枚举添加 `PartialOrd + Ord` derive → Small < Medium < Large
2. `sort_by(priority.cmp.then(effort.cmp))` → Critical+Small 在最前
3. `dedup_by(dimension + file + action_type)` → 相邻同类 action 去重

### 数据结构

**Issue**:

| 字段 | 类型 | 用途 |
|------|------|------|
| level | Level (enum) | critical/warning/info |
| category | Category (enum) | structural/complexity/fragility 等 |
| message | String | 问题描述 |
| prescription | Option\<String\> | 重构建议（人可读） |
| actions | Vec\<Action\> | 结构化处方 |

**Action**:

| 字段 | 类型 | 用途 |
|------|------|------|
| dimension | String | 来源维度 |
| action_type | ActionType (enum) | Split/Extract/Add/Remove/Replace/Move/Refactor |
| target | Target (struct) | file(必填) + line_range(可选) + symbol(可选) |
| reason | String | 包含具体数值的操作原因 |
| priority | Priority (enum) | Critical/High/Medium/Low，derive Ord |
| effort | Effort (enum) | Small/Medium/Large |

**处方模板映射**:

| 问题类型 | 处方模板 |
|---------|----------|
| 文件过多 | 按职责拆分为子模块 |
| 目录过深 | 扁平化嵌套目录 |
| 大文件 | 提取独立逻辑到新文件 |
| churn 热点 | 拆分高频变更文件，隔离不稳定逻辑 |

**全维度 Action 映射**:

| 维度 | Issue 类型 | ActionType | Priority | Effort |
|------|-----------|-----------|----------|--------|
| structural | file_count > CRIT | Split | Critical | Large |
| structural | depth > WARN | Move | Medium | Medium |
| complexity | size > 50KB | Split | Critical | Large |
| complexity | size > 15KB | Extract | High | Medium |
| fragility | concentration > WARN | Refactor | High | Large |
| fragility | high churn | Split | Critical | Medium |
| maintainability | duplicates | Extract | High | Medium |
| maintainability | long function | Extract | High | Small |
| observability | unwrap/panic | Replace | High | Medium |
| observability | no logging | Add | High | Medium |
| quality | no tests | Add | Critical | Large |
| quality | low test ratio | Add | High | Large |
| reliability | unsafe code | Replace | High | Medium |
| reliability | injection | Replace | Critical | Small |
| reliability | secrets | Replace | Critical | Small |
| performance | nested loops | Extract | Critical/High | Small |
| performance | excess clones | Refactor | Medium | Medium |

**位置精度扩展**:

| 维度 | 原 tuple | 扩展后 tuple | 新增字段 |
|------|----------|-------------|---------|
| performance | (path, line_no, depth) | 不变 | — |
| maintainability | (path, func_name, func_len) | (path, func_name, func_len, start_line) | start_line: u32 |
| observability | (path, count) | (path, count, Vec<u32>) | line_numbers: Vec<u32> |
| reliability | (path, pattern) | (path, pattern, line_no) | line_no: u32 |

## 3. 关键决策

| 决策 | 选择 | 为什么 |
|------|------|--------|
| 规则驱动 | 硬编码 | v1 规则少，不需要注册表；备选 DSL/配置文件增加解析复杂度 |
| 输出结构 | Vec\<Issue\> | 不需要跨快照查询；备选写入 DB 增加无意义 schema |
| 诊断+处方 | 同文件 | 紧密耦合，分开冗余；备选拆模块需跨文件传递上下文 |
| 生成方式 | 规则驱动模板 | PRD 排除 LLM 生成；备选 LLM 需网络和 API 依赖 |
| Info 无处方 | 仅 critical/warning | 信息量太低不值得建议；备选全级别处方增加噪音 |
| actions 容器类型 | `Vec<Action>` | 不阻塞 1:N 映射，JSON schema 稳定；备选 `Option<Action>` 限制一个 issue 多个 action |
| prescription 保留 | 双写过渡 | CLI 输出不中断，M3 移除；备选立即删除会破坏现有用户输出 |
| skip_serializing_if | actions 空时跳过 | JSON 向后兼容，无 action 的 issue 输出不变 |
| Priority 排序 | derive PartialOrd + Ord | Critical < High < Medium < Low，sort_by 自然升序 |
| Target.file | 必填 String | M1 只有目录/文件级，M3 补充行级；备选 Option<String> 增加无意义的空值处理 |
| Effort 定义 | Small/Medium/Large 枚举 | 比数值估算更实用，agent 可按 effort 过滤 |
| 构造函数 vs builder | 两个构造函数 `new()` + `with_actions()` | 场景只有两种（有/无 action），builder pattern 过度设计 |
| reason 字段 | 包含具体数值和操作建议 | agent 需要上下文判断优先级合理性 |
| 分层提升策略 | A(直接填充) + B(扩展采集) + C(保持现状) | 按数据可用性最小化改动量 |
| 不提升 C 层维度 | structural/complexity/fragility/quality 保持 None | 只有项目级/文件级指标，强行生成行号是虚假精度 |
| tuple 扩展 vs struct | 继续用 tuple | 4 个维度内部数据结构保持一致风格 |
| 排序策略 | Priority → Effort 双键 | Critical+Small 排最前是最佳修复路径 |
| Effort 排序方式 | derive Ord（按定义顺序） | 零代码量，Small < Medium < Large 自然匹配 |
| 去重策略 | dedup_by（排序后相邻去重） | 标准库方法，O(n) 复杂度 |
| 去重维度 | dimension + file + action_type | 同维度同文件同类型才算重复 |

## 4. 迭代记录

### 2026-04-14

- 初始方案：诊断规则 + 处方模板，与诊断同模块

### 2026-04-15

- Action Schema 类型系统：新增 action.rs，7 种 ActionType，structural POC
- Prescription 引擎重构：8 维度全部迁移为构造函数 + Action 生成器
- 位置精度提升：4 维度按 A/B/C 分层提升行号信息
- Action 排序：Priority→Effort 双键排序 + dedup_by 去重
