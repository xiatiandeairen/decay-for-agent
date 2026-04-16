# 诊断处方

## 1. 问题

### 痛点

- **分数无法指导行动**: M3 给出了三维度健康分数，但不告诉"具体哪里有问题"。用户看到 structural: 65 后还需要自己分析哪些目录过深、哪些文件过大。
- **诊断到修复断层**: 诊断出具体问题后，用户仍然需要自己设计解决方案。对 AI agent 来说，"src/git.rs 太大"不够可操作——agent 需要知道"怎么拆、拆成什么"。
- **处方非结构化**: 现有处方（prescription）是纯文本字符串，agent 无法解析文本建议来自动执行重构——不知道操作哪个文件、什么变更类型、优先级如何。
- **维度覆盖不全**: Action Schema 在 structural 维度做了 POC，但剩余 7 个维度仍用空 actions 数组，处方信息只在文本字段中，约 87% 的可操作处方 agent 无法消费。
- **位置精度不足**: 所有 action 的 `target.line_range` 和 `target.symbol` 全部为 None，agent 消费 action 后仍需搜索代码才能定位具体位置。
- **排序和去重缺失**: 顶层 actions 数组只按 Priority 单维排序，同优先级内无序，多个维度可能对同一文件产生重复 action。

### 影响范围

影响所有分数低于健康阈值的项目。从分数到定位到修复到精确位置的全链路，每次 decay 运行都涉及。8 个维度的 Warning/Critical 级 issue 均需要结构化 action 支持。

### 为什么现在做

M3 评分已就绪，分数无法指导行动是当前最大的可用性瓶颈。从诊断到处方到结构化 Action 到位置精度到排序去重，是让 decay 输出从"人可读"到"agent 可消费可执行"的完整链路。

## 2. 目标用户

| 角色 | 场景 | Before | After |
|------|------|--------|-------|
| AI agent | 运行 `decay` 定位问题 | 只有分数，不知道具体问题在哪 | 获得分级问题列表，直接知道哪里有问题、多严重 |
| AI agent | 运行 `decay` 获取修复方案 | 知道问题但不知道怎么修 | 每个问题附带处方，如"建议拆分 src/git.rs：提取 walker 逻辑到 git/walker.rs" |
| AI agent | 读取 decay JSON 输出生成重构计划 | 需要理解自由文本处方 | 直接解析 Action 结构体获取类型、目标、优先级 |
| AI agent | 消费所有维度的处方 | 只有 structural 维度有结构化 action | 8 个维度全部输出结构化 action |
| AI agent | 读取 action 后定位代码 | line_range 全部 None，需二次搜索 | 直接从 action 获取行号和函数名 |
| AI agent | 按顺序消费 actions 执行修复 | 同优先级内无序，有重复 | 按 priority→effort 双键排序，无重复，按数组顺序执行即最优路径 |
| 开发者 | 筛选最值得处理的问题 | 只能逐条阅读文本建议 | 按优先级和变更类型过滤 action |
| decay 维护者 | 构造 Issue 对象 | struct literal 手写，格式不一致 | 统一使用 `Issue::new()` / `Issue::with_actions()` 构造函数 |

## 3. 核心假设

- **假设**: 自动从采集数据中识别具体问题并分级 → agent 能直接知道"哪里有问题、多严重"
- **验证方式**: 运行 `decay`，输出包含分级问题列表（critical/warning/info）和具体文件/目录
- **假设**: 为每个问题生成可执行的重构建议 → agent 能从"发现问题"直接进入"执行修复"
- **验证方式**: 每个 critical/warning 问题附带至少一条具体的重构建议
- **假设**: 将文本处方结构化为 Action（含变更类型、目标文件、原因、优先级、工作量） → agent 可直接消费 decay 输出执行重构
- **验证方式**: structural 维度输出的 Action 可序列化为 JSON，包含所有必要字段
- **假设**: 在已有采集数据中提取行号/函数名 → action 的位置信息足够精确，agent 可直接定位
- **验证方式**: performance/maintainability/observability/reliability 维度的 action 包含 line_range 或 symbol
- **假设**: 按 Priority → Effort 双键排序 + 去重 → agent 按数组顺序执行即为最优修复路径
- **验证方式**: 输出 actions 数组中 Critical+Small 在最前，Low+Large 在最后，无重复项

## 4. 方案

- **Before**: 只有分数，不知道具体问题在哪 → **After**: 输出具体问题列表，如"src/git.rs 是变更热点（1145 行 churn）"
- **Before**: 知道问题但不知道怎么修 → **After**: 每个问题附带处方，如"建议拆分 src/git.rs：提取 walker 逻辑到 git/walker.rs"
- **Before**: `prescription: Option<String>` 纯文本 → **After**: `actions: Vec<Action>` 含 action_type/target/priority/effort/reason
- **Before**: 7 个维度用 `actions: vec![]` → **After**: 所有维度用 `Issue::with_actions()` 或 `Issue::new()`，Warning/Critical 附带 Action
- **Before**: 所有 action 的 target.line_range = None → **After**: 4 个维度按数据可用性分层提升精度（A 直接填充 / B 扩展采集 / C 保持现状）
- **Before**: actions 只按 Priority 单维排序，有重复 → **After**: Priority→Effort 双键排序 + 同 dimension+file+action_type 去重

### 任务追踪

| 任务 | Tech | 状态 | 备注 |
|------|------|------|------|
| 问题诊断 | [tech](tech.md) | 已完成 | 硬编码规则，分级问题列表 |
| 重构处方 | [tech](tech.md) | 已完成 | 规则驱动模板，与诊断同模块 |
| Action Schema 类型系统 | [tech](tech.md) | 已完成 | 7 种 ActionType，structural POC |
| Prescription 引擎 8 维度迁移 | [tech](tech.md) | 已完成 | 全维度构造函数迁移 |
| 位置精度提升 | [tech](tech.md) | 已完成 | 4 维度行号/函数名填充 |
| Action 优先级排序 | [tech](tech.md) | 已完成 | 双键排序 + 去重 |

## 5. 验收标准

- 用户运行 `decay` → 应看到分级问题列表，按级别排序（critical > warning > info）
- 用户查看每个问题 → 应看到级别、类别、具体文件或目录、量化数据
- 用户运行 `decay` → 每个 critical/warning 问题附带重构建议（动作、目标文件、预期效果）
- 用户运行 `decay --json` → 应看到 Action Schema 完整：ActionType(7 种)、Priority(4 级)、Effort(3 级)、Target、Action
- 用户查看 Report JSON → 应看到顶层 `actions` 字段包含结构化 action 数组
- 用户检查 8 个维度源码 → 全部使用构造函数，无 struct literal 构造 Issue
- 用户运行 performance/maintainability 分析 → 应看到 action 有 `line_range` 和 `symbol`
- 用户运行 `decay --json` → 应看到顶层 actions 按 priority→effort 双键排序，无重复
- 用户对无问题的项目运行 → 应看到 "No issues found"

## 6. 排除项

- 不包含自动执行重构 — 风险不可控，需人工确认
- 不包含 LLM 辅助生成建议 — 保持纯规则驱动，结果可预测
- 不包含自定义诊断规则 — 增加配置复杂度，收益不明确
- 不添加 AST 解析 — 超出 decay 当前能力边界
- 不引入复合评分公式（priority × effort 加权）— 当前枚举排序足够
- 不跨维度去重 — 不同维度对同文件的不同 action_type 应保留
