# 位置精度提升 — Action 包含精确文件位置

## 1. 问题

M2 完成后所有维度输出结构化 Action，但 `target.line_range` 和 `target.symbol` 全部为 None。agent 消费 action 后仍需搜索代码才能定位具体位置。

## 2. 目标用户

- AI agent：读取 action 后可直接定位到代码行，无需二次搜索
- 开发者：action 输出包含行号和函数名，可直接跳转

## 3. 核心假设

**在已有采集数据中提取行号/函数名 → action 的位置信息足够精确，agent 可直接定位。**

验证方式：performance/maintainability/observability/reliability 维度的 action 包含 line_range 或 symbol。

## 4. 方案

分 3 层按数据可用性提升精度：

| 层级 | 策略 | 维度 |
|------|------|------|
| A 直接填充 | 已有行号数据 | performance（nest line_no）、maintainability（func start_line + symbol） |
| B 扩展采集 | analyze 函数增加行号收集 | observability（unwrap 行号列表）、reliability（injection/secret 行号） |
| C 保持现状 | 无行级数据可提升 | structural、complexity、fragility、quality |

### 任务

| 任务 | 文档 | 进度 |
|------|------|------|
| location-precision tech | [tech](tech.md) | 0/0 |

## 5. 验收标准

- performance: nested loop action 有 `line_range: (line, line)`
- maintainability: long function action 有 `line_range: (start, end)` + `symbol`
- observability: unwrap action 有 `line_range: (first, last)`
- reliability: injection/secret action 有 `line_range: (line, line)`
- structural/complexity/fragility/quality 保持 `line_range: None`（无行级数据）
- `cargo test` 75 tests 全部通过

## 6. 排除项

- 不添加 AST 解析（超出 decay 当前能力边界）
- 不修改 Action Schema 定义
- 不为文件级/项目级数据强行生成行号
