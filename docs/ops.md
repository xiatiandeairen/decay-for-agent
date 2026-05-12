# decay 操作记录规范

## 目的

`ops` 文档只记录如何验证产品价值。

v0.1.x 的关键问题是：

> `decay diff` 是否能在真实 AI 编程任务中改变作者的 commit 决策？

因此操作记录不能堆运行日志，只记录能回答这个问题的信息。

## 价值验证算法

验证目标不是收集“工具看起来有用”的认可数据，而是判断：

> 在真实项目的 commit 前，`decay diff` 是否比作者原本的 review 更早、更稳定地指出值得处理的函数级退化？

### 验证单元

一次验证单元是一轮真实开发任务，而不是一次命令运行。

纳入条件：

- 任务来自真实项目的正常开发、修复或重构，不为测试 `decay` 而刻意制造复杂代码。
- 任务开始前能保存 baseline。
- 任务结束时存在准备提交或准备交给用户验收的一组改动。
- 作者在看 `decay diff` 输出前，先写下自己的人工预判。

排除条件：

- demo、toy project、专门构造的坏代码。
- 已经知道某个函数明显变坏，只是用 `decay` 补截图。
- baseline 缺失、scan scope 明显错误，导致 diff 不能解释本次改动。

### 每轮验证流程

1. 任务开始前保存 baseline：

```bash
decay baseline <version>
```

2. 记录任务预登记：

```text
task:
project:
baseline:
change_intent:
expected_risky_functions:
manual_review_plan:
```

`expected_risky_functions` 可以为空，但必须在运行 `decay diff` 前填写。它用于判断 `decay` 是否真的提供新增信息。

3. 完成一波 AI 或手动修改后，先人工 review，再记录人工预判：

```text
manual_findings:
manual_commit_decision: commit | revise | unsure
```

4. 运行：

```bash
decay diff <version>
```

5. 对每个 evidence 做人工判断，并记录是否改变决策。v0.2.0 的 git-ref diff 会输出 complexity / stability / quality / history sections，并在本地 SQLite 记录 changeset 与 evidence；这些记录可以辅助复盘，但不能替代人工分类。

6. 如果没有任何命中，也要记录一条 session 结果。无命中是重要分母，不能丢弃。

### 命中分类

每个 `decay diff` 命中只能归为以下一种：

- `fixed`：命中有效，作者在看到输出后改变了 commit 决策并返工。
- `reviewed`：命中有效，作者没有立即返工，但因此做了额外 review、补测试或留下明确 follow-up。
- `ignored`：命中真实，但作者明确决定本轮不处理。
- `noise`：命中无行动价值，或与本次改动无关。

只有 `fixed` 能证明强产品价值。`reviewed` 只能证明有辅助 review 价值，不能等同于 fixed。`ignored` 和 `noise` 用来校准阈值、scope 和输出语义。

对于 v0.2.0 evidence-first 输出，分类对象可以是一条 evidence，而不一定是单个函数退化。分类时必须保留 evidence 的 `category`、`kind`、`source` 和 `file`。

### 反作弊规则

以下情况不能算 `fixed`：

- 作者在运行 `decay diff` 前已经准备返工同一函数。
- AI 或测试已经明确指出同一问题，`decay` 只是重复提醒。
- 命中来自历史旧债，和本次改动无关。
- 作者只是为了让记录好看而做轻微重排、拆行、改名。
- 返工后没有让函数级退化消失或显著降低。

以下情况必须记录为负反馈：

- 输出无法定位具体问题。
- 命中过多，作者不愿逐个判断。
- 命中虽真实但不影响当前提交决策。
- baseline、scope、阈值或报告文案导致误解。

### 汇总指标

每累计 5 轮真实验证，更新一次汇总：

```text
sessions:
sessions_with_findings:
findings_total:
fixed:
reviewed:
ignored:
noise:
no_finding_sessions:
decision_changes:
abandoned_due_to_friction:
top_noise_reasons:
top_fixed_patterns:
```

核心判断看三个比率：

- `decision_change_rate = decision_changes / sessions`
- `actionable_rate = (fixed + reviewed + ignored) / findings_total`
- `noise_rate = noise / findings_total`

`findings_total = 0` 时不计算 actionable/noise，只记录无命中会话数量。

### 演进门槛

每 5 轮验证后做一次判断：

| 结果 | 判断 | 下一步 |
|---|---|---|
| `fixed >= 1` 且作者认为值得继续运行 | 初步证明 commit 前退化裁决有价值 | 优先优化 baseline 摩擦、输出解释和阈值 |
| `reviewed >= 2` 但 `fixed = 0` | 可能更适合作为 review 辅助，而不是 commit gate | 调整定位和文案，降低“裁决”承诺 |
| `noise_rate > 50%` | 当前阈值、scope 或 metric 组合不可信 | 先分析噪音原因，不新增 metric |
| 连续 5 轮 `findings_total = 0` | 可能太钝、scope 太窄，或目标场景不对 | 检查 scan scope、阈值和真实任务类型 |
| `abandoned_due_to_friction > 0` | 使用摩擦正在破坏验证 | 优先修 workflow，而不是继续收集数据 |
| 连续 10 轮无 `fixed` 且无稳定 `reviewed` | v0.1.0 产品假设未通过 | 暂停新增功能，重新评估产品切口 |

### 数据质量要求

- 记录所有纳入验证的 session，包括 no finding 和 noise。
- 每个 fixed 必须有返工前后的 `decay diff` 结果或简短说明。
- 每个 noise 必须写原因，不能只写“误报”。
- 不把“测试通过”“工具能运行”“作者觉得不错”记为产品价值。
- 不跨项目合并结论前，先保留项目名和任务类型，避免不同场景混在一起。

## 外部 PR 验证

v0.2.0 增加外部开源 PR 验证，用来避免只在本项目 dogfood 中自我证明。

验证位置：

- `eval/corpus/pr-cases.toml`：预注册 PR 样本。
- `eval/results/`：每个 PR 的原始 `decay diff` 输出、changed files 和 GitHub metadata。
- `eval/judgements/`：人工判定 useful / plausible / noise / missed。
- `eval/reports/`：汇总指标。

外部 PR 验证的基本规则：

- 先固定样本，再运行 `decay`，不允许跑完后挑选结果好看的 PR。
- negative control 必须进入分母；docs-only / test-only / deps-only PR 输出越安静越好。
- 每条 useful evidence 必须能连接到外部事实：review comment、CI failure、测试补充、linked issue 或后续 fix。
- history evidence 不能单独算 useful；必须和本次 diff 或外部事实结合。
- missed risk 必须记录；不能只统计命中的 evidence。

第一轮目标是 20 个 PR：

- 2 个仓库，每个 10 个 PR。
- 每个仓库包含 bug fix、feature、refactor 和 negative control。
- 通过门槛：`negative_control_silence_rate >= 80%`、`precision@top3 >= 50%`、`actionable_pr_rate >= 40%`，且至少 3 条 useful evidence。

运行方式：

```bash
cargo install --path .
eval/scripts/run-pr-case.sh ripgrep-3210
python3 eval/scripts/summarize.py
```

外部 PR runner 必须验证 git-ref diff 路径，即在 PR head 上运行 `decay diff <base-sha>`；不要用 named baseline diff 代替，否则会测到旧输出形态。

## 日常使用流程

1. 在任务开始前保存 baseline：

```bash
decay baseline <version>
```

2. 完成一波 AI 或手动修改后运行：

```bash
decay diff <version>
```

3. 对每个命中做人工判断：

- `fixed`：命中有效，作者因此返工。
- `reviewed`：命中有效，作者因此做了额外 review、补测试或留下明确 follow-up。
- `ignored`：命中真实，但作者决定暂不处理。
- `noise`：命中无行动价值，或与本次改动无关。

4. 没有命中也要记录 session 结果。

5. 只有 `fixed` 能证明强产品价值。`reviewed` 是弱信号，不能替代 fixed。`ignored` 和 `noise` 用来校准阈值、scope 和输出。

## Dogfood 记录格式

每轮验证先记录 session：

```text
date:
project:
task:
baseline:
change_intent:
expected_risky_functions:
manual_findings:
manual_commit_decision: commit | revise | unsure
command:
diff_summary:
recorded:
session_result: no_finding | has_findings | invalid
friction:
notes:
```

每个命中只记录必要信息：

```text
date:
project:
task:
baseline:
command:
result:
category:
kind:
source:
file:
case_type: fixed | reviewed | ignored | noise
function:
metric:
human_judgment:
action_taken:
decision_changed: yes | no
notes:
```

字段含义：

| 字段 | 含义 |
|---|---|
| `project` | 被验证的真实项目名 |
| `task` | 本次 AI 或手动修改的目标 |
| `baseline` | 对比使用的 baseline 名称 |
| `expected_risky_functions` | 运行 `decay diff` 前作者已经怀疑的函数 |
| `manual_findings` | 运行 `decay diff` 前人工 review 已发现的问题 |
| `manual_commit_decision` | 运行 `decay diff` 前作者准备 commit、返工还是不确定 |
| `diff_summary` | `decay diff` 的汇总结果；无命中也要写 |
| `recorded` | `decay diff` 是否成功写入本地 evidence DB，例如 `recorded=true changeset=... evidence_saved=...` |
| `session_result` | 本轮是否有命中，或是否因 baseline/scope 等问题作废 |
| `friction` | baseline 命名、命令理解、输出噪音等使用摩擦 |
| `result` | `decay diff` 的核心输出 |
| `category` | evidence 类别，例如 complexity / stability / quality / history |
| `kind` | evidence 类型，例如 worsened / test_gap / churn_hotspot |
| `source` | evidence 来源，例如 diff_hunk+ast_snapshot / stability / quality / git_history |
| `file` | evidence 指向的文件 |
| `case_type` | 本次命中的产品价值分类 |
| `human_judgment` | 作者为什么认为它该修、可忽略或是噪音 |
| `action_taken` | 实际采取的动作 |
| `decision_changed` | 是否因为 `decay diff` 改变 commit 决策 |

## 判断规则

有效命中：

- 指向本次改动引入或放大的复杂度。
- 作者原本没有明确意识到。
- 作者看完后认为应该返工或至少需要 review。

噪音命中：

- 主要是历史旧债。
- 和本次改动没有关系。
- 虽然超阈值，但作者没有任何行动意愿。
- 输出信息不足以定位问题。

## 当前已知风险

当前 `decay doctor` 输出显示项目自身有 8 个 finding。这些 finding 可以作为架构债记录，但不能直接算 dogfood 成功。

dogfood 成功只能来自 `decay diff` 的真实 fixed 案例。
