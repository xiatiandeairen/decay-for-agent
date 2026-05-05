# decay Product Roadmap

## 1. Product Vision

### Product Essence

- **Positioning**: `decay` 是面向 AI 协作编程的 Rust 函数级复杂度退化裁决 CLI。
- **Motivation**: AI 或人完成一波代码修改后，作者需要在 commit 前判断“这次改动是否让函数变得更难维护”；人工读 diff 不稳定，AI 自评不可靠，传统复杂度工具不回答“相对 baseline 是否变糟”。
- **Long-term vision**: 作者在 AI 协作开发中拥有一个轻量、客观、可重复的 commit 前退化裁决工具，能把“感觉代码变复杂了”变成可追踪的函数级证据。

### Value System

| Tier | Value | Metric |
|------|-------|--------|
| **Immediate value** | 每次修改后，作者能看到相对 baseline 的函数级退化证据，而不是只看当前复杂函数列表。 | `decay diff` fixed case 数：目标 ≥1（target value, pending validation） |
| **Cumulative value** | 连续 dogfood 后，阈值、scope 和输出语义能基于真实命中收敛。 | dogfood case 分类完整率：目标 100%（target value, pending validation）；当前真实数据：no data（dogfood 记录尚未形成） |
| **Strategic value** | AI 协作从“事后人工感觉 review”转为“commit 前有 baseline 裁决”。 | commit 决策改变次数：目标 ≥1（target value, pending validation）；当前真实数据：no data（尚未记录 fixed case） |

### Core Problem

| Problem | Occurrence Frequency | Per-Occurrence Cost | Reach | Existing Workaround |
|---------|----------------------|---------------------|-------|---------------------|
| AI 或人修改 Rust 函数后，作者难以稳定判断本次改动是否引入局部复杂度退化。 | 每次 AI coding / refactor session 都可能发生（estimated，基于本项目使用场景） | no data（尚未量化每次人工 review 成本） | 当前只覆盖作者本人维护 Rust 项目的场景 | 人工读 diff；问题是耗时、主观，且容易漏掉局部复杂化 |
| 当前风险体检和本次退化裁决容易混淆。 | 每次运行 `doctor` 或裸命令理解 CLI 语义时可能发生（estimated，来自本轮命令语义讨论） | no data（尚未量化误用成本） | 当前只覆盖本地 CLI 使用者 | 读 README / `--help`；问题是旧文档曾把命令语义写得不一致 |

### Target Users

| Role | Typical Scenario | Before | After | Estimated Efficiency Gain |
|------|------------------|--------|-------|--------------------------|
| 作者本人 | 在 Rust 项目中让 AI 完成一波修改，commit 前检查是否产生函数级退化 | 只能人工读 diff 或运行当前风险体检；无法稳定区分旧债和本次退化 | 保存 baseline 后运行 `decay diff`，只看新增或变糟的函数级风险 | no data（尚无连续 dogfood 数据）；目标是至少产生 1 个 fixed case |

### Competitive Comparison

| Solution | Positioning | Target Users | Core Features | Strengths | Limitations |
|----------|-------------|--------------|---------------|-----------|-------------|
| **decay** | Rust 函数级复杂度退化裁决 CLI | 使用 AI 协作维护 Rust 项目的作者 | baseline、diff、doctor、prod scope、partial scan diagnostics | 聚焦“这次是否变糟”；输出可解释的函数级证据 | 仅 Rust；产品价值未验证；baseline 需要手动命名 |
| 人工读 diff | 人基于经验 review 本次改动 | 所有代码作者 | 读 patch、判断风险、决定返工 | 上下文完整；能理解业务语义 | 主观、不稳定、容易漏掉局部复杂度变化 |
| 传统 linter / complexity 工具 | 当前代码质量或复杂度检查 | 需要静态检查的开发者 | 规则检查、复杂度阈值、报告当前问题 | 成熟、客观、易接入 CI | 通常不回答“相对某个 baseline 本次是否变糟” |
| AI 自评 | 让 AI 解释或审查自己生成的 diff | 使用 AI 编程的作者 | 总结风险、给出建议、解释代码 | 快、上下文对话成本低 | 结论可能漂移；存在迎合倾向；缺少稳定可重复裁决 |

## 2. Version Plan

### Version Summary Table

| Version | Core Direction | Core-Metric Delta | Status | Period | Milestones |
|---------|----------------|-------------------|--------|--------|------------|
| v0.1.0 | 跑通 Rust 函数扫描、baseline、diff、doctor 的本地闭环 | ↑ active metrics 0→6（measured: `src/metric/mod.rs`）；↑ integration tests 0→15（measured: `cargo test` 2026-05-05）；↓ product certainty no data→unproven（no data: 尚无 fixed case） | released | TBD - 2026.05.05 | M1-M1 |
| v0.1.x | 用 dogfood 验证 `diff` 是否改变 commit 决策 | ↑ dogfood classified cases 0→TBD（target value, pending validation）；↑ fixed cases 0→≥1（target value, pending validation） | in development | 2026.05.05 - TBD | M2-M2 |
| v0.2 | 如果 `diff` 被证明有价值，降低 baseline 使用摩擦 | ↑ baseline workflow convenience no data→TBD（pending v0.1.x）；↓ manual naming friction no data→TBD（pending v0.1.x） | planned | TBD - TBD | TBD |

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
| Integration tests | 0 | 15（measured: `cargo test` 2026-05-05） | ↑ 15 |
| Current doctor findings in this repo | N/A | 8（measured: `decay doctor` 2026-05-05） | N/A |
| Real fixed cases from `decay diff` | 0 | no data（尚未形成 dogfood 记录） | no data |

#### v0.1.x — dogfood validation

- **Strategic intent**: 验证 `decay diff` 是否真的能改变作者的 commit 决策。
- **Input/output**: input 是作者持续在真实 AI 编程任务中运行 baseline/diff；output 是 classified dogfood cases。
- **Priority rationale**: 在没有 fixed case 前，新增 metric、CI、JSON 或更多语言都会放大未验证假设。
- **Risks and dependencies**: 风险是命中主要来自旧债或噪音；依赖作者按 `docs/ops.md` 记录 fixed / ignored / noise。
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

## 3. Milestones

| # | Core Direction | Goal Achievement | Status | Completion Date |
|---|----------------|------------------|--------|-----------------|
| [M1](milestones/m1.md) | 交付 v0.1.0 本地功能闭环 | 功能闭环完成，`cargo test` 和 clippy 通过；产品 fixed case 仍无数据，因此不声明产品成功 | done | 2026-05-05 |
| [M2](milestones/m2.md) | 验证 `diff` 是否改变 commit 决策 | 尚未开始；需要按 `docs/ops.md` 记录 fixed / ignored / noise | not started | — |
