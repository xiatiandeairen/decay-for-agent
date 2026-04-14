# Markdown 输出 技术方案

## 1. 背景

`--markdown` flag 输出格式化健康报告。

## 2. 方案

templates/health-report.md 模板，include_str! 编译嵌入，str::replace 填充占位符。

### 模板占位符

scores: {{structural}}, {{complexity}}, {{fragility}}, {{composite}} + trend 后缀
scan: {{file_count}}, {{dir_count}}, {{max_depth}}
git: {{total_commits}}, {{files_analyzed}}
issues: {{issues_section}} 按级别分组生成

## 3. 关键决策

| 决策 | 结论 | 理由 |
|------|------|------|
| 模板引擎 | str::replace | 零依赖，占位符少 |
| 模板嵌入 | include_str! | 编译时嵌入，不需要运行时文件 |

## 4. 迭代记录

- 2026-04-14: 初始方案
