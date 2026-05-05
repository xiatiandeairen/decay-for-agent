[English](README.md) · **中文**

# decay

`decay` 是一个 Rust 函数级复杂度退化检测 CLI，面向 AI 协作编程场景。

它当前只回答一个窄问题:

> 相对某个 baseline，这次改动有没有让某个函数在局部结构上明显变糟？

它不是通用代码质量平台，也不是另一个复杂函数排行榜。v0.1.0 的核心价值还在 dogfood 验证中。

## 状态

v0.1.0:

- 仅支持 Rust
- 作者本人 dogfood 中
- 没有外部用户
- CLI 输出、SQLite schema、阈值都可能破坏式变化
- 功能闭环已实现，但产品假设尚未被真实案例证明

## 快速开始

```bash
cd /path/to/your/rust/project

decay doctor             # 诊断当前代码风险；不依赖 baseline，不作为 gate
decay baseline v1.0.0    # 保存当前代码为命名 baseline
# ... 修改代码 / 让 AI 修改代码 ...
decay diff v1.0.0        # 当前工作区 vs v1.0.0 baseline
decay baseline v1.1.0    # 保存新的命名 baseline
decay diff v1.0.0 v1.1.0 # 两个 baseline 互比
```

裸 `decay` 只展示简洁命令列表，不扫描、不写存储。
详细参数说明使用 `decay --help`。

## 命令语义

| 命令 | 作用 | 退出码 |
|---|---|---|
| `decay` | 展示简洁命令列表 | 0 |
| `decay doctor` | 查看当前代码中已存在的风险 | 总是 0，除非运行错误 |
| `decay baseline <version>` | 保存命名 baseline | 成功 0；同名不同内容且未 `--replace` 返回 1 |
| `decay diff <version>` | 当前工作区相对 baseline 的退化裁决 | 无退化 0；有退化 1 |
| `decay diff <from> <to>` | 两个 baseline 之间的退化裁决 | 无退化 0；有退化 1 |

`doctor` 是体检。`diff` 才是 commit 前裁决。

## Diff 报告什么

`decay diff` 只报告退化:

- 新增函数已经超阈值
- 已有函数从未超阈值变为超阈值
- 已有函数原本已超阈值，并继续变糟

不报告:

- 删除函数
- 指标下降
- 指标不变
- 仍在阈值内的小幅上升

示例:

```text
status=degraded from=v1.0.0 to=current degradations=2

[functions that crossed a risk boundary]
- src/store.rs:130 save_baseline
  problem=Function body grew beyond a focused size.
  change=Function size changed from 22 statements to 31 statements; recommended limit is 25 statements.
```

## Metrics

当前 active metrics:

| Metric | 阈值 | 含义 |
|---|---:|---|
| `nesting` | 4 | 最大控制流嵌套深度 |
| `cyclomatic` | 10 | McCabe 分支复杂度 |
| `cognitive` | 15 | 更贴近阅读负担的分支复杂度 |
| `params` | 5 | 函数参数数 |
| `statement_count` | 25 | 函数体可执行步骤数量 |
| `max_condition_ops` | 4 | 单个条件表达式中的布尔操作符最大数量 |

阈值语义:

```text
value > threshold => breach
```

## 扫描范围

默认 `--scope prod` 关注主维护面 Rust 代码，会排除常见测试/示例/fixture 噪音。

需要完整视图时使用:

```bash
decay doctor --scope all
decay diff v1.0.0 --scope all
```

工具读取项目根 `.gitignore`，同时支持 `--exclude <pattern>`。

## 边界

- 不支持 Rust 之外语言。
- 不支持 JSON 输出。
- 不支持阈值配置。
- 不支持语义级 rename/move 追踪；改名或移动可能表现为删除 + 新增。
- Closure 不独立计入，它的复杂度算入外层函数。
- 单文件 parse 失败不会中断扫描，但结果会标记 partial。
- 不做 DB migration；v0.1.0 没有外部兼容承诺。

## 文档

- [docs/roadmap.md](docs/roadmap.md): 产品路线和当前状态
- [docs/requirements/function-complexity-detection/prd.md](docs/requirements/function-complexity-detection/prd.md): v0.1.0 PRD
- [docs/arch/decay.md](docs/arch/decay.md): 当前架构
- [docs/ops.md](docs/ops.md): dogfood / 运维闭环
- [docs/decision/v0.1.0-closeout.md](docs/decision/v0.1.0-closeout.md): v0.1.0 收尾决策

## License

尚未指定。在添加 license 文件之前，按 all rights reserved 处理。
