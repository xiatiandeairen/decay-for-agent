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
- `reports/`：汇总报告输出。
- `scripts/run-pr-case.sh`：运行单个 PR case。
- `scripts/select-candidates.py`：从 GitHub PR 搜索生成候选，不直接进入 50 case 分母。
- `scripts/summarize.py`：汇总 results + judgements。

## 样本规则

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
- `comparison.md`：最后填写 matched/noise/missed/actionable 和指标。
- `raw/pr.json`、`raw/files.txt`、`raw/stat.txt`、`raw/diff.patch`、`raw/decay.stdout`、`raw/decay.stderr`。

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

生成候选：

```bash
python3 eval/scripts/select-candidates.py hyperium/hyper --limit 30
```
