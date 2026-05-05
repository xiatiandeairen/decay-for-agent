[English](README.md) · **中文**

# decay

一个面向"用 AI 协作写代码"项目的函数级复杂度退化检测器。

> **状态：v0.1 — 仅支持 Rust，作者本人 dogfood 中，粗糙之处在所难免。**
> 暂无外部用户。接口、阈值、输出格式都可能变化。

## 为什么做这个

AI 编程助手有个倾向：修 bug 就是再加一个 `if`。问题是，当你回头问同一个助手"代码是不是变复杂了"，它有结构性的"说没问题"偏见（sycophancy）。结果就是——复杂度悄悄爬升，没人察觉。

`decay` 是一个小而克制的"局外人"：

- 它不参与代码生产，所以没有动机替已经写出来的代码辩护。
- 它看的是 **delta**，不是绝对值。`lizard`、Clippy、ESLint 的复杂度规则告诉你"这个函数现在很复杂"；`decay` 告诉你"*这次改动*让它变更糟了"。
- 函数粒度，跨快照持久化——多个 session 累积下来的慢性退化也能看见。

## 它做什么

在 Rust 项目里跑 `decay`。它用 tree-sitter 解析每个 `.rs` 文件，给每个函数算 4 个 metric，存一个快照。之后再跑一次，`decay diff` 告诉你哪些函数退化了。

下面是 v0.1 开发期间在本仓库自查的真实输出——工具识别出 3 个 AI 刚写完、自己没意识到已经超阈值的函数：

```
decay v0.1.0
Scanned 225 files, 896 functions in 0.27s
Snapshot #2 saved

34 functions exceed threshold:

  src/cli/diff_cmd.rs:92  collect_metric_lines
    cognitive: 23 ⚠ (>15)

  src/cli/scan.rs:92  print_exceeded
    cognitive: 16 ⚠ (>15)

  src/metric/cognitive.rs:131  score_match
    nesting: 5 ⚠ (>4)
  ...
```

这三个都是 v0.1 实施期间 AI subagent 写出、人工 review 没拦住、被工具首次自查抓出来的真实退化。后续都已重构。

## 安装

目前仅源码安装（暂未发布到 crates.io）：

```bash
git clone <本仓库>
cd decay
cargo install --path .
```

## 快速开始

```bash
cd /path/to/your/rust/project

decay         # 扫描 + 保存快照 + 列出超阈值函数
# ... 自己改 / 让 AI 助手改 ...
decay         # 再来一次快照
decay diff    # 对比上一次快照
```

`decay diff` 只会报真正变糟的函数：新增并超阈值、首次跨过阈值、已超阈值且更高。下降和未变化的函数静默。

## 状态与边界

下面这份清单是诚实的。在你依赖工具输出之前请先读一遍。

- **仅支持 Rust。** 不支持 TypeScript / Python / 其他。多语言在 roadmap 上，未实现。
- **函数重命名 / 跨文件移动会被识别为"删除 + 新增"。** 指纹是 `xxh3(file + name + param_types)`，重命名或换文件就追不上了。
- **Closure 不独立计入。** 它的复杂度算在外层函数头上。一个长 `query_map` 闭包会让外层函数 metric 暴增。
- **退出码不区分"有退化 / 无退化"。** 两种情况都返回 0。靠退出码做 agent 集成 gate 暂不可靠。
- **不读 `.gitignore`。** 排除目录只有 `target/` 和 `.git/`。扫描前可能需要先清掉构建产物或 vendored 副本。
- **阈值硬编码**（`nesting 4`、`cyclomatic 10`、`cognitive 15`、`params 5`）。v0.1 不支持配置。
- **暂无外部验证。** 用户只有作者一人。阈值和认知复杂度公式在 Rust 惯用语法（`?` 链、match arm）上的表现是按直觉校准的，不是在语料上验证过的。
- **同一 `impl` 块外，同名同参的函数共享指纹。** 实际项目里很少遇到，v0.1 接受这个偏差。

如果上面任何一条挡住了你的使用场景，`decay` 暂时还没准备好给你用。

## 工作原理

- **解析** — tree-sitter-rust 提取每个 `function_item`（含 `impl` 方法、trait 默认实现）。无函数体的签名、closure、宏生成的函数不提取。
- **度量** — 每个函数 4 个 metric：
  - **Nesting** — 最大代码块嵌套深度。
  - **Cyclomatic** — McCabe 圈复杂度（分支数 + 1）。
  - **Cognitive** — SonarSource 公式，带 nesting bonus，深嵌套权重高于浅分支。
  - **Params** — 签名参数数。
- **指纹** — `xxh3_64(file ⊕ name ⊕ param_types)`，参数类型归一化（去掉 lifetime、去空白）。跨进程稳定。
- **持久化** — SQLite，路径 `dirs::data_dir()/decay/snapshots.db`，两张表：`snapshots`、`functions`。
- **对比** — 按指纹对齐两快照，把每个函数分为 `Added` / `CrossedThreshold` / `Worsened`，按 `max(value − threshold)` 降序输出。

更多细节与设计依据：[`docs/plans/v0.1.md`](docs/plans/v0.1.md)、[`docs/audit.md`](docs/audit.md)。

## License

尚未指定。在添加 license 文件之前，按 all rights reserved 处理。
