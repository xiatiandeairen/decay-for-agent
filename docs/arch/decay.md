# decay 架构

## 架构目标

架构只服务一个价值目标：

> 把 Rust 项目的一次代码状态转成函数级复杂度证据，并支持 baseline diff 判断本次改动是否退化。

因此系统边界必须清楚：扫描、解析、metric、存储、diff、CLI 展示各自负责一件事，避免产品承诺和实现事实再次漂移。

## 模块边界

```text
CLI
  解析命令，渲染终端输出
  |
  v
Pipeline
  组织 walk、parser、metric、fingerprint
  |
  +--> Store
  |     保存和读取命名 baseline
  |
  +--> Diff
        比较函数 metric，输出退化结果
```

| 模块 | 职责 | 边界 |
|---|---|---|
| CLI | 解析用户意图，输出人可读报告 | 不定义 metric 真相 |
| Pipeline | 从项目根目录产出函数、metric 和扫描诊断 | 不隐藏 partial scan |
| Parser | 用 tree-sitter 提取 Rust 函数和上下文 | 不计算 metric，不做存储决策 |
| Metric registry | 定义 active metric、阈值、分组、格式化和计算器 | metric 变更必须先改这里 |
| Diff | 判断函数级退化 | 不关心 CLI 文案和 SQLite 细节 |
| Store | 保存命名 baseline 和 active metric 值 | v0.1.0 不保留旧 schema 兼容 |

## 数据流

```text
filesystem
  -> walk 找到 Rust 文件
  -> parser 提取函数
  -> metric registry 计算 active metrics
  -> pipeline 产出 FunctionSet 和 diagnostics
  -> store 保存 baseline
  -> diff 比较 baseline/current
  -> CLI 输出报告
```

关键约束：

- metric 列表只能以 `src/metric/mod.rs` 的 registry 为准。
- README、PRD、架构文档里的 active metric 必须和 registry 一致。
- `diff` 是主价值路径；`doctor` 只是查看当前风险。
- parser 失败时继续扫描，但必须暴露 diagnostics，并在 baseline 中记录 partial 状态。

## 关键设计决策

| 决策 | 当前选择 | 原因 |
|---|---|---|
| Metric 归属 | 集中在 registry | 防止 CLI、diff、store、docs 各自维护一套 metric 列表 |
| 阈值语义 | `value > threshold` 才算超阈值 | 避免 `doctor` 和 `diff` 边界不一致 |
| 默认命令 | `decay` 打印简洁命令列表，`--help` 打印详细帮助 | 让入口可发现，同时避免把辅助体检误认为默认 gate |
| 扫描失败 | 继续扫描并标记 partial | 保持工具可用，同时不伪装证据完整 |
| schema 兼容 | v0.1.0 不兼容旧 schema | 当前没有外部用户，清晰事实比兼容旧实验更重要 |

## 当前架构债

`decay doctor` 在当前代码中发现 8 个 finding：

- `src/cli/diff_cmd.rs:232 build_report`：`cognitive=20`，超过 15。
- `src/store.rs:273 row_to_function`：`cyclomatic=14`，超过 10。
- `src/cli/diff_cmd.rs:12 run`：`cyclomatic=11`，超过 10。
- `src/cli/diff_cmd.rs:126 print_verbose`：`statement_count=34`，超过 25。
- `src/cli/doctor_cmd.rs:75 print_verbose`：`statement_count=31`，超过 25。
- `src/store.rs:93 save_baseline`：`params=7`，超过 5。
- `src/parser.rs:102 visit`：`params=7`，超过 5。
- `src/store.rs:142 insert_baseline`：`params=6`，超过 5。

这些是当前真实风险。是否马上修，取决于它们是否影响 v0.1.x 的产品验证；不应为了追求零 finding 而偏离 `diff` 价值验证。
