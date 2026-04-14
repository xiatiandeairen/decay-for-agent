# JSON 输出

## 1. 问题

当前 terminal 输出是人可读格式，程序无法消费。后续 MCP 集成需要机器可读的结构化输出。

## 2. 目标用户

使用 decay 的 AI agent 和自动化工具。通过 `--json` 获取结构化数据。

## 3. 核心假设

**提供 --json flag → 程序可直接解析输出，为后续 MCP/skill 集成铺路。**

验证方式：`decay --json` 输出合法 JSON。

## 4. 方案

- **Before**: 只有人可读输出 → **After**: `decay --json` 输出包含 scores/issues/trend 的 JSON

### 任务

| 任务 | 文档 | 进度 |
|------|------|------|
| JSON 输出 tech | [tech](tech.md) | 0/0 |

## 5. 验收标准

- `decay --json` → 输出合法 JSON，退出码 0
- JSON 包含 scores、issues、trend、scan、git 字段
- `decay`（无 --json）→ 输出不变

## 6. 排除项

- 不包含 YAML/XML 等其他格式
- 不包含 --json 与 --help 的组合处理
