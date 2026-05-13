# decay Product Roadmap

## 1. Product Vision

### Product Essence

- **Positioning**: `decay` 是面向 AI 协作编程的 Rust 项目复杂度与变更影响证据 CLI。
- **Motivation**: AI 或人完成一波代码修改后，作者需要知道“本次 diff 对项目复杂度、稳定性和质量造成了哪些可观察影响”；人工读 diff 不稳定，AI 自评不可靠，传统复杂度工具通常只报告当前状态，不解释本次变更和项目历史上下文。
- **Long-term vision**: 作者在 AI 协作开发中拥有一个轻量、客观、可重复的证据层工具，能把“感觉这个 PR 风险大”或“这个项目哪里复杂”变成可追踪、可复查、可 dogfood 的项目证据。

### Value System

| Tier | Value | Metric |
|------|-------|--------|
| **Immediate value** | 每次修改后，作者能看到相对 baseline 的函数级退化证据，而不是只看当前复杂函数列表。 | `decay diff` fixed case 数：目标 ≥1（target value, pending validation） |
| **Evidence value** | 任意 PR 过来时，作者能看到本次 diff 改到了哪些函数、边界、历史高波动区域和测试缺口。 | `diff` useful evidence 数：目标 ≥2/5-10 个真实 PR（target value, pending validation）；action change 数：目标 ≥1（target value, pending validation） |
| **Planning value** | 做项目质量治理时，作者能看到当前复杂度债务集中在哪里，哪些区域最值得优先看。 | `doctor` top debt accepted rate：目标 top 3 中 ≥2 个被认可（target value, pending validation）；实际治理动作 ≥1（target value, pending validation） |
| **Cumulative value** | 连续 dogfood 后，阈值、scope、数据源和输出语义能基于真实命中收敛。 | dogfood evidence 分类完整率：目标 100%（target value, pending validation）；当前真实数据：no data（dogfood 记录尚未形成） |
| **Strategic value** | AI 协作从“事后人工感觉 review”转为“review 前有可信证据层”。 | review / test / split / refactor 决策改变次数：目标 ≥1（target value, pending validation）；当前真实数据：no data |

### Core Problem

| Problem | Occurrence Frequency | Per-Occurrence Cost | Reach | Existing Workaround |
|---------|----------------------|---------------------|-------|---------------------|
| AI 或人修改 Rust 函数后，作者难以稳定判断本次改动是否引入局部复杂度退化。 | 每次 AI coding / refactor session 都可能发生（estimated，基于本项目使用场景） | no data（尚未量化每次人工 review 成本） | 当前只覆盖作者本人维护 Rust 项目的场景 | 人工读 diff；问题是耗时、主观，且容易漏掉局部复杂化 |
| 任意 PR 过来时，作者难以快速确认本次 diff 对复杂度、稳定性和质量的真实影响。 | 每次 PR review 都会发生（estimated，来自目标使用场景） | no data（尚未量化 review 搜索成本） | 当前只覆盖本地 git 工作区和作者本人项目 | 人工扫 diff、git log、测试变更和项目文档；问题是证据分散且不可重复 |
| 做项目质量治理时，作者难以判断当前复杂度债务优先级。 | 每次规划重构、补测或质量治理时发生（estimated） | no data（尚未量化治理选择成本） | 当前只覆盖 Rust 项目 | 运行复杂度工具或人工经验判断；问题是普通复杂函数列表无法区分“复杂但稳定”和“复杂且高波动/边界敏感” |
| 当前风险体检和本次退化裁决容易混淆。 | 每次运行 `doctor` 或裸命令理解 CLI 语义时可能发生（estimated，来自本轮命令语义讨论） | no data（尚未量化误用成本） | 当前只覆盖本地 CLI 使用者 | 读 README / `--help`；问题是旧文档曾把命令语义写得不一致 |

### Target Users

| Role | Typical Scenario | Before | After | Estimated Efficiency Gain |
|------|------------------|--------|-------|--------------------------|
| 作者本人 | 在 Rust 项目中让 AI 完成一波修改，commit 前检查是否产生函数级退化 | 只能人工读 diff 或运行当前风险体检；无法稳定区分旧债和本次退化 | 保存 baseline 后运行 `decay diff`，只看新增或变糟的函数级风险 | no data（尚无连续 dogfood 数据）；目标是至少产生 1 个 fixed case |
| 作者本人 | review 任意 PR，想知道本次 diff 影响了哪些复杂度、稳定性和质量证据 | 人工扫 diff、git history、测试变更和文档约束；容易漏掉 touched risky function、边界变化或测试缺口 | 运行 `decay diff <git-ref>`，获得带 source 的 complexity / stability / quality / history evidence | no data；目标是产生人工初看遗漏的 useful evidence |
| 作者本人 | 规划项目质量治理，想知道当前复杂度债务优先看哪里 | 只能看复杂函数列表或凭经验选模块；无法结合历史波动和边界敏感性 | planned：增强 `decay doctor`，获得 hot complexity debt 排名和证据 | no data；目标是改变或强化治理优先级 |

### Competitive Comparison

| Solution | Positioning | Target Users | Core Features | Strengths | Limitations |
|----------|-------------|--------------|---------------|-----------|-------------|
| **decay** | Rust 项目复杂度与变更影响证据 CLI | 使用 AI 协作维护 Rust 项目的作者 | baseline、diff、doctor、prod scope、partial scan diagnostics；v0.2.0 目标增加 evidence database、git history、diff hunk mapping | 聚焦“本次 diff 影响了什么”和“当前复杂度债务优先级”；输出可解释、可追溯的证据 | 仅 Rust；产品价值未验证；baseline 和 evidence workflow 仍需 dogfood |
| 人工读 diff | 人基于经验 review 本次改动 | 所有代码作者 | 读 patch、判断风险、决定返工 | 上下文完整；能理解业务语义 | 主观、不稳定、容易漏掉局部复杂度变化 |
| 传统 linter / complexity 工具 | 当前代码质量或复杂度检查 | 需要静态检查的开发者 | 规则检查、复杂度阈值、报告当前问题 | 成熟、客观、易接入 CI | 通常不回答“相对某个 baseline 本次是否变糟” |
| AI 自评 | 让 AI 解释或审查自己生成的 diff | 使用 AI 编程的作者 | 总结风险、给出建议、解释代码 | 快、上下文对话成本低 | 结论可能漂移；存在迎合倾向；缺少稳定可重复裁决 |

## 2. Version Plan

### Version Summary Table

| Version | Core Direction | Core-Metric Delta | Status | Period | Milestones |
|---------|----------------|-------------------|--------|--------|------------|
| v0.1.0 | 跑通 Rust 函数扫描、baseline、diff、doctor 的本地闭环 | ↑ active metrics 0→6（measured: `src/metric/mod.rs`）；↑ integration tests 0→16（measured: `cargo test` 2026-05-06）；↓ product certainty no data→unproven（no data: 尚无 fixed case） | released | TBD - 2026.05.05 | M1-M1 |
| v0.1.x | 用 dogfood 验证 `diff` 是否改变 commit 决策，并修复 baseline identity / moved function 噪音 | ↑ dogfood classified cases 0→TBD（target value, pending validation）；↑ fixed cases 0→≥1（target value, pending validation）；↓ path-move identity noise initial fix（measured: local mxcore dogfood 2026-05-11） | in development | 2026.05.05 - TBD | M2-M2 |
| v0.2.0 | 在事实证据层之上建设语义筛选层，把静态扫描事实收敛成高价值 review 证据 | ↑ semantic-filtered top risk baseline 65% precision@top3→≥75%（target，基于 10-case 外部 PR benchmark）；↑ actionable_pr_rate 60%→≥70%（target）；↓ missed_high_risk_count 2/10→≤1/20（target）；↓ weak-only top risk 3/10→≤2/20（target） | planning | 2026.05.13 - TBD | M3-M3 |

### Version Details

#### v0.1.0 — functional PoC

- **Strategic intent**: 证明本地 Rust 函数级复杂度证据链能跑通；不证明产品价值已经成立。
- **Input/output**: input no data（未记录实际工时）；output 是可运行 CLI、SQLite baseline、diff 裁决和 doctor 体检。
- **Priority rationale**: 没有基础扫描、baseline 和 diff，就无法验证 commit 前退化裁决这个产品假设。
- **Risks and dependencies**: 风险是把测试通过误认为产品成功；依赖 Rust parser、SQLite baseline、metric registry 与文档一致。
- **Success metric**: 功能成功以 `cargo test` 和 `cargo clippy -- -D warnings` 通过为准；产品成功不在 v0.1.0 内宣称。
- **Core value**: 作者可以在本地保存 baseline，并比较当前工作区或两个 baseline 的函数级退化。
- **User coverage**: 作者本人 dogfood；无外部用户。
- **Core metric** (N/A → v0.1.0):

| Metric | N/A | v0.1.0 | Delta |
|--------|-----|--------|-------|
| Active metrics | 0 | 6（measured: `src/metric/mod.rs`） | ↑ 6 |
| Integration tests | 0 | 16（measured: `cargo test` 2026-05-06） | ↑ 16 |
| Current doctor findings in this repo | N/A | 8（measured: `decay doctor` 2026-05-05） | N/A |
| Real fixed cases from `decay diff` | 0 | no data（尚未形成 dogfood 记录） | no data |

#### v0.1.x — dogfood validation

- **Strategic intent**: 验证 `decay diff` 是否真的能改变作者的 commit 决策。
- **Input/output**: input 是作者持续在真实 AI 编程任务中运行 baseline/diff；output 是 classified dogfood cases。
- **Priority rationale**: 在没有 fixed case 前，新增 metric、JSON/agent protocol 或更多语言都会放大未验证假设；基础 CI 只作为工程质量门禁保留。
- **Risks and dependencies**: 风险是命中主要来自旧债或噪音；依赖作者按 `docs/ops.md` 记录 fixed / reviewed / ignored / noise。
- **Success metric**: 至少 1 个 `fixed` case（target value, pending validation），且噪音没有导致作者停止运行 `decay diff`。
- **Core value**: 把 `diff` 从“功能可用”推进到“对 commit 决策有证据价值”。
- **User coverage**: 作者本人 dogfood。
- **Core metric** (v0.1.0 → v0.1.x):

| Metric | v0.1.0 | v0.1.x | Delta |
|--------|--------|--------|-------|
| Real fixed cases from `decay diff` | no data | ≥1（target value, pending validation） | ↑ ≥1 |
| Classified dogfood cases | 0 | TBD（pending dogfood） | no data |
| Noise share | no data | no target yet（先分类再判断） | no data |
| Manual baseline friction | identified qualitatively | TBD（pending dogfood evidence） | no data |

#### v0.2.0 — semantic-filtered evidence layer

- **Strategic intent**: 把 `decay` 从“静态扫描事实列表”推进到“语义筛选后的 PR review 证据”。0.1.1 已验证纯扫描能发现默认值/配置/显式危险构造/机械 refactor 静默等场景，也暴露出 `mixed_concerns`、`blast_radius`、`touched_existing_risky` 单独出现时价值弱；0.2.0 的重点不是继续堆事实 detector，而是在事实层之上建立语义层，筛掉弱证据、组合强证据、识别高价值漏报。
- **Input/output**: input 是 0.1.1 已有事实证据（complexity / stability / quality / semantic pattern / history / local DB）加可选 PR/issue/review/CI/decision docs 上下文；output 是语义筛选后的 top review evidence，包含 claim、supporting facts、confidence、known limitation 和 reviewer action。
- **Priority rationale**: 当前 10-case 外部 PR benchmark 显示 `precision_at_top3=65%`、`actionable_pr_rate=60%`、`missed_high_risk_count=2`、`noise_top3_count=3`。这说明事实层已能产生价值，但 top risk 仍会被弱扫描信号污染，也会漏掉 API/protocol 行为类高价值风险。0.2.0 需要让 Top risk 从“扫描命中排序”变成“语义价值排序”。
- **Risks and dependencies**: 风险是语义层变成不可复查的主观判断或 AI 自评；依赖 fact schema 稳定、evidence source 可追溯、外部 PR benchmark、bad-case regression tests、local evidence DB 记录语义裁决过程。
- **Success metric**: 第一轮以 20 个高质量 PR case 为验证集：`precision_at_top3 >= 75%`、`actionable_pr_rate >= 70%`、`negative_control_silence_rate >= 90%`、`missed_high_risk_count <= 1`、`weak_only_top_risk <= 2`；每个语义结论必须能回链到事实 evidence 或外部上下文。
- **Core value**: `diff` 不再只回答“扫描到了什么”，而是回答“哪些扫描事实组合后真的值得 reviewer 先看、为什么可信、下一步该检查什么”；`doctor` 暂时降级为后续场景，不阻塞 0.2.0 diff 价值验证。
- **User coverage**: 作者本人 dogfood；外部数据源默认 opt-in。
- **Core metric** (v0.1.x → v0.2.0):

| Metric | v0.1.x | v0.2.0 | Delta |
|--------|--------|--------|-------|
| Fact evidence baseline | 10-case benchmark: precision@top3 65%, actionable 60%, missed high risk 2, noise top3 3 | 20-case benchmark: precision@top3 ≥75%, actionable ≥70%, missed high risk ≤1, weak-only top risk ≤2（target） | ↑ top value, ↓ weak noise |
| Semantic claim traceability | no dedicated layer | every semantic top finding links to supporting facts and source context（target 100%） | ↑ explainability |
| Weak signal handling | weak signals can enter Top risk directly | `mixed_concerns` / `blast_radius` / history / touched old debt default to supporting context unless paired with direct evidence | ↓ misleading top risk |
| High-value missed risk | API/protocol behavior cases can be missed | add bad-case driven semantic scenarios for API surface, protocol behavior, behavior gate/config boundary | ↓ missed risk |
| Negative controls | 100% silence on current 10-case judged set | keep ≥90% silence while adding semantic layer | maintain trust |
| Doctor output shape | function-level current findings | deferred until diff semantic layer passes first benchmark | scope reduction |

## 3. Milestones

| # | Core Direction | Goal Achievement | Status | Completion Date |
|---|----------------|------------------|--------|-----------------|
| [M1](milestones/m1.md) | 交付 v0.1.0 本地功能闭环 | 功能闭环完成，`cargo test` 和 clippy 通过；产品 fixed case 仍无数据，因此不声明产品成功 | done | 2026-05-05 |
| [M2](milestones/m2.md) | 验证 `diff` 是否改变 commit 决策 | 尚未开始；需要按 `docs/ops.md` 记录 fixed / reviewed / ignored / noise | not started | — |
| [M3](milestones/m3.md) | v0.2.0 语义筛选证据层 | planning；0.1.1 已完成事实层和 10-case benchmark，下一步建设 semantic-filtered top review evidence | planning | — |
