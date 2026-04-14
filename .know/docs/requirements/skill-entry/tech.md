# skill 入口 技术方案

## 1. 背景

PRD 要求 `/decay` slash command 触发健康检查。

## 2. 方案

skills/decay/SKILL.md 定义 skill。AI 用 Bash tool 调用 `decay` CLI，直接显示 terminal 输出。plugin.json 注册 plugin 元数据。

### 文件结构

| Action | File | Responsibility |
|--------|------|---------------|
| create | `skills/decay/SKILL.md` | skill 定义 |
| create | `.claude-plugin/plugin.json` | plugin 元数据 |

## 3. 关键决策

| 决策 | 结论 | 理由 |
|------|------|------|
| 调用方式 | Bash tool 调用 CLI | 不依赖 MCP 配置 |
| 输出 | terminal 原始输出 | v1 输出已可读 |

## 4. 迭代记录

- 2026-04-14: 初始方案
