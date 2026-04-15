# 文本匹配仅做粗筛，精确判断用结构化解析

## 原则

代码分析中，字符串/正则匹配适合快速过滤（pass 1），但不适合语义判断（pass 2）。

## 已知误判案例

| 场景 | 匹配方式 | 误判 |
|------|---------|------|
| package.json 依赖计数 | `count("\":")` | JSON 所有字段都命中 |
| catch 块检测 | `starts_with("catch")` | 变量名 catching_errors |
| 循环嵌套 | 数 `{` | 一行多个 brace 全标记为 loop |

## 修复策略

- package.json → `serde_json::Value` 精确解析（已有依赖，零额外成本）
- 语法结构 → 至少用 brace-depth tracking + line-level state machine
- 每个检测方法附带 false positive test case
