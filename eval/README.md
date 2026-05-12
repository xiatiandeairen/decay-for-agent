# External PR Validation Harness

## 目标

这个目录用于验证 `decay diff <git-ref>` 在真实开源 PR 上是否有产品价值。

验证目标不是证明命令能运行，而是回答：

- `decay` 是否能指出 reviewer 值得优先看的文件或函数？
- evidence 是否和真实 review、CI、测试补充、后续修复有关？
- negative control（docs/test/deps-only PR）是否足够安静？

## 目录

- `corpus/pr-cases.toml`：预注册 PR 样本。先固定样本，再运行工具，避免事后挑选好看的结果。
- `results/`：runner 生成的原始输出和 metadata。
- `judgements/`：人工判定。每个 PR 一个 TOML 文件。
- `reports/`：汇总报告输出。
- `scripts/run-pr-case.sh`：运行单个 PR case。
- `scripts/summarize.py`：汇总 results + judgements。

## 样本规则

第一轮只做 20 个 PR：

- 2 个仓库，每个 10 个 PR。
- 每个仓库包含 3 个 bug fix、3 个 feature、2 个 refactor、2 个 negative control。
- negative control 包括 docs-only、test-only、dependency-only、CI-only 等低风险变更。

样本必须先写入 `corpus/pr-cases.toml`，再运行 `decay`。

## Judgement Rubric

每条 evidence 只能归入一种：

- `useful`：指向真实 review 关注点、CI/测试问题、后续修复点，或让 reviewer 明确改变 review / test / split / refactor 动作。
- `plausible`：风险合理，但没有外部事实确认。
- `noise`：与本次 PR 风险无关、定位太泛、来自 docs/test/generated 噪声，或重复无行动价值信息。
- `missed`：真实 review / CI / 后续修复指出了风险，但 `decay` 没有给出对应 evidence。

限制：

- history evidence 不能单独算 `useful`，除非它和本次 diff、review、CI、测试缺口或后续修复结合。
- “工具输出看起来合理”最多算 `plausible`。
- 运行失败、clone 失败、依赖缺失必须进入报告分母，不能丢弃。

## 指标

- `precision@top3`：每个 PR 前 3 条 evidence 中 `useful / (useful + plausible + noise)`。
- `actionable_pr_rate`：至少有 1 条 `useful` evidence 的 PR 占比。
- `negative_control_silence_rate`：negative control PR 中 evidence 数为 0 的占比。
- `noise_rate`：`noise / (useful + plausible + noise)`。
- `missed_high_risk_rate`：存在 `missed` 的 PR 占比。

## 第一轮通过门槛

20 个 PR 跑完后再判断：

- `negative_control_silence_rate >= 80%`
- `precision@top3 >= 50%`
- `actionable_pr_rate >= 40%`
- 至少 3 条 `useful` evidence
- 所有 top false positives 都有明确修复方向

达不到门槛时，不进入风险评分或 merge recommendation；先修 evidence source、scope、identity 和输出语义。

## 运行

准备：

```bash
cargo install --path .
gh auth status
```

运行单个 case：

```bash
eval/scripts/run-pr-case.sh ripgrep-3210
```

runner 会在 PR head 上运行：

```bash
decay diff <base-sha>
```

也就是验证 v0.2 evidence-first git-ref diff 路径，而不是旧的 named baseline diff 路径。

汇总：

```bash
python3 eval/scripts/summarize.py
```
