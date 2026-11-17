# External PR Validation Harness

## 目标

这个目录用于验证 `decay diff <git-ref>` 在真实开源 PR 上是否有产品价值。

验证目标不是证明命令能运行，而是回答：

- `decay` 是否能指出 reviewer 值得优先看的文件或函数？
- evidence 是否和真实 review、CI、测试补充、后续修复有关？
- negative control（docs/test/deps-only PR）是否足够安静？

## 目录

- `corpus/pr-cases.toml`：预注册 PR 样本。先固定样本，再运行工具，避免事后挑选好看的结果。
- `cases/<case-id>/`：runner 生成的可复盘 case，包括 raw context、ground truth、decay evidence 和 comparison。
- `results/`：旧 runner 输出目录，仅作兼容读取；新 case 不再写这里。
- `judgements/`：旧人工判定 TOML，仅作兼容读取；新 case 使用 `comparison.md`。
- `risk-signals/holdout-v1.toml`：阶段 gate 保留集；只用于阶段验收，不用于同一 sprint 内调规则。
- `reports/`：汇总报告输出。
- `scripts/run-pr-case.sh`：运行单个 PR case。
- `scripts/validate-decay-evidence.py`：校验 `decay.stdout` 是否满足 `decay-evidence.v1` contract，并可导出 normalized JSON。
- `scripts/run-error-propagation-gate.py`：运行 `error_propagation_change` 单风险项 gate，输出 precision/recall/overclaim/duplicate 指标。
- `scripts/run-concurrency-ordering-gate.py`：运行 `concurrency_ordering_change` 单风险项 gate，输出 precision/recall/overclaim/duplicate 指标。
- `scripts/run-external-contract-gate.py`：运行 `external_contract_change` 单风险项 gate，输出 precision/recall/overclaim/duplicate 指标。
- `scripts/run-lifecycle-ordering-gate.py`：运行 `lifecycle_ordering_change` 单风险项 gate，输出 precision/recall/overclaim/duplicate 指标。
- `scripts/run-state-transition-gate.py`：运行 `state_transition_change` 单风险项 gate，输出 precision/recall/overclaim/duplicate 指标。
- `scripts/run-semantic-case.sh`：在单个 PR case 上运行 fact evidence 和 codex-cli semantic filtering。
- `scripts/run-frozen-benchmark.py`：批量编排 frozen v1 corpus，生成 run manifest 和 judgement 模板。
- `scripts/summarize-frozen-benchmark.py`：汇总 frozen v1 judgement，比较 fact-only 与 semantic-filtered 指标。
- `scripts/select-candidates.py`：从 GitHub PR 搜索生成候选，不直接进入 50 case 分母。
- `scripts/summarize.py`：汇总 results + judgements。

## 样本规则

三层数据集分工：

- `dev-cases.toml`：开发回归集，可以高频使用。
- `holdout-v1.toml`：阶段 gate 保留集，只在阶段收尾时使用。
- `frozen-v1.toml`：版本级产品效果验证集，不能为提分替换样本。

目标是 50 个高价值 diff，但分三批执行：

- Batch 1：10 个 case，验证脚本、模板、指标和 judgement 流程。
- Batch 2：20 个 case，扩大仓库和风险类型。
- Batch 3：20 个 case，补齐到 50 并输出最终报告。

样本必须先写入 `corpus/pr-cases.toml`，再运行 `decay`。

每个进入 50 的 case 至少满足一项：

- linked issue / bug report。
- review risk discussion。
- CI/test failure。
- follow-up fix。
- public API / protocol / storage / security / config / dependency sensitive change。
- negative control，用来验证应该安静时是否安静。

`select-candidates.py` 只产生候选，不算入 50；必须人工审核 `context.md` 后才算入。

## Case 产物

每个新 case 固定生成：

- `metadata.toml`：base/head、repo、PR、运行 exit code。
- `context.md`：选择理由和 high-value gate。
- `ground-truth.md`：只看外部事实写，禁止参考 decay 输出。
- `decay-evidence.md`：从 decay stdout 自动摘录，只记录不评价。
- `raw/decay.stdout`：原始 CLI 输出；必须先通过 `validate-decay-evidence.py`，否则 case 不入库。
- `comparison.md`：最后填写 matched/noise/missed/actionable 和指标。
- `raw/pr.json`、`raw/files.txt`、`raw/stat.txt`、`raw/diff.patch`、`raw/decay.stdout`、`raw/decay.stderr`。
- semantic case 还会生成 `raw/semantic-prompt.md` 和 `raw/semantic-findings.json`。

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

## 50 Case 通过门槛

50 个 case 跑完后再判断：

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

运行 frozen v1 benchmark dry-run：

```bash
python3 eval/scripts/run-frozen-benchmark.py --dry-run
```

校验 risk-signal pack 与 holdout contract：

```bash
python3 eval/scripts/validate-risk-signal-pack.py
```

校验阶段 judgement contract：

```bash
python3 eval/scripts/validate-risk-eval-judgement.py path/to/judgement.toml
```

校验 `decay-evidence.v1` contract，并导出 normalized JSON：

```bash
python3 eval/scripts/validate-decay-evidence.py path/to/decay.stdout path/to/decay-evidence.json
```

运行完整 frozen v1 benchmark：

```bash
python3 eval/scripts/run-frozen-benchmark.py
```

这会按 `eval/corpus/risk-signal-frozen-v1.toml` 运行 fact-only 和
semantic-filtered 两条路径，并在 `eval/benchmarks/frozen-v1/judgements/`
生成每个 case 的判分模板。判分完成后汇总：

```bash
python3 eval/scripts/summarize-frozen-benchmark.py
```

运行 `error_propagation_change` 单风险项 gate：

```bash
python3 eval/scripts/run-error-propagation-gate.py \
  --json-out eval/reports/error-propagation-gate-v1.json \
  --md-out eval/reports/error-propagation-gate-v1.md
```

运行 `concurrency_ordering_change` 单风险项 gate：

```bash
python3 eval/scripts/run-concurrency-ordering-gate.py \
  --json-out eval/reports/concurrency-ordering-gate-v1.json \
  --md-out eval/reports/concurrency-ordering-gate-v1.md
```

运行 `external_contract_change` 单风险项 gate：

```bash
python3 eval/scripts/run-external-contract-gate.py \
  --json-out eval/reports/external-contract-gate-v1.json \
  --md-out eval/reports/external-contract-gate-v1.md
```

运行 `state_transition_change` 单风险项 gate：

```bash
python3 eval/scripts/run-state-transition-gate.py \
  --json-out eval/reports/state-transition-gate-v1.json \
  --md-out eval/reports/state-transition-gate-v1.md
```

运行 `lifecycle_ordering_change` 单风险项 gate：

```bash
python3 eval/scripts/run-lifecycle-ordering-gate.py \
  --json-out eval/reports/lifecycle-ordering-gate-v1.json \
  --md-out eval/reports/lifecycle-ordering-gate-v1.md
```

生成候选：

```bash
python3 eval/scripts/select-candidates.py hyperium/hyper --limit 30
```
