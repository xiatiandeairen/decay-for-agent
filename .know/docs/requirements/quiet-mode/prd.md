# quiet 模式

## 1. 问题

外部工具（sprint/CI/git hook）需要简洁的健康状态判断，不需要完整输出。当前只有完整输出和 JSON 两种模式。

## 2. 目标用户

自动化脚本和 hook。通过 exit code 判断健康状态，通过一行摘要获取关键数据。

## 3. 核心假设

**`--quiet` + 语义化 exit code → 任何工具都能集成 decay 健康检查。**

验证方式：`decay --quiet` 输出一行摘要，exit code 0=健康/1=有 critical。

## 4. 方案

- **Before**: 只有完整输出，脚本需要解析 → **After**: `--quiet` 一行摘要 + exit code

### 任务

| 任务 | 文档 | 进度 |
|------|------|------|
| quiet 模式 tech | [tech](tech.md) | 0/0 |

## 5. 验收标准

- `decay --quiet` → 输出一行：`Health: 81/100 (0 critical)`
- exit code 0 → 无 critical issues
- exit code 1 → 有 critical issues
- `decay --quiet` 可与 `&&` / `||` 配合使用

## 6. 排除项

- 不包含自定义 exit code 阈值
