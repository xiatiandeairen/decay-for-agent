# decay 操作记录规范

## 目的

`ops` 文档只记录如何验证产品价值。

v0.1.x 的关键问题是：

> `decay diff` 是否能在真实 AI 编程任务中改变作者的 commit 决策？

因此操作记录不能堆运行日志，只记录能回答这个问题的信息。

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
- `ignored`：命中真实，但作者决定暂不处理。
- `noise`：命中无行动价值，或与本次改动无关。

4. 只有 `fixed` 能证明产品价值。`ignored` 和 `noise` 用来校准阈值、scope 和输出。

## Dogfood 记录格式

每个命中只记录必要信息：

```text
date:
task:
baseline:
command:
result:
case_type: fixed | ignored | noise
function:
metric:
human_judgment:
action_taken:
notes:
```

字段含义：

| 字段 | 含义 |
|---|---|
| `task` | 本次 AI 或手动修改的目标 |
| `baseline` | 对比使用的 baseline 名称 |
| `result` | `decay diff` 的核心输出 |
| `case_type` | 本次命中的产品价值分类 |
| `human_judgment` | 作者为什么认为它该修、可忽略或是噪音 |
| `action_taken` | 实际采取的动作 |

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
