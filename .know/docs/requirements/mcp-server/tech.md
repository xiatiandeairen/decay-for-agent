# MCP server 技术方案

## 1. 背景

PRD 要求用 MCP stdio 协议暴露 decay_check tool，让 Claude Code agent 能程序化调用 decay。

## 2. 方案

TypeScript + @modelcontextprotocol/sdk，放在项目内 mcp/ 目录。stdio transport，1 个 tool，内部调用 decay --json。

### 文件结构

| Action | File | Responsibility |
|--------|------|---------------|
| create | `mcp/package.json` | npm 配置 + 依赖 |
| create | `mcp/tsconfig.json` | TypeScript 编译配置 |
| create | `mcp/src/index.ts` | server 入口，tool 注册，CLI 调用 |

### CLI 路径发现

优先级：`../target/release/decay` → `../target/debug/decay` → 系统 PATH `decay`

### Tool 定义

- name: `decay_check`
- 参数: `path` (string, 可选, 默认 cwd)
- 返回: decay --json 的完整 JSON

## 3. 关键决策

| 决策 | 结论 | 理由 |
|------|------|------|
| 语言 | TypeScript | MCP 官方 SDK 最成熟 |
| 位置 | mcp/ 目录 | 同仓库维护 |
| CLI 发现 | 同仓库 → PATH | 开发和生产都能工作 |

## 4. 迭代记录

- 2026-04-14: 初始方案
