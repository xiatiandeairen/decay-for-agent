# skill 入口

## 1. 问题

用户需要切换到终端手动运行 `decay` 命令，打断 Claude Code 工作流。需要一个 `/decay` 入口让用户在对话中直接触发健康检查。

## 2. 目标用户

使用 Claude Code 的开发者。在对话中 `/decay` 即可看到健康报告。

## 3. 核心假设

**提供 `/decay` slash command → 用户零学习成本触发健康检查，不打断工作流。**

验证方式：在 Claude Code 中 `/decay` 触发健康检查并看到结果。

## 4. 方案

- **Before**: 需要切换终端运行命令 → **After**: `/decay` 直接在对话中输出

### 任务

| 任务 | 文档 | 进度 |
|------|------|------|
| skill 入口 tech | [tech](tech.md) | 0/0 |

## 5. 验收标准

- `/decay` → 触发健康检查，输出评分+诊断+处方
- `/decay` 在非项目目录 → 输出错误提示，不 panic
- SKILL.md 包含触发条件和使用说明

## 6. 排除项

- 不包含 Markdown 格式化输出（→ M3）
- 不包含参数传递（`/decay --json` 等）
