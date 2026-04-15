# 先写 PRD + tech 再编码

每个里程碑/功能必须先产出需求文档再开始写代码：

1. `.know/docs/requirements/{name}/prd.md` — 问题、方案、验收标准、排除项
2. `.know/docs/requirements/{name}/tech.md` — 技术方案、文件变更清单、关键决策
3. 更新 `CLAUDE.md` 索引 + roadmap 需求链接
4. 开新 sprint 实现代码

## 为什么

v3 M1 开发时建立的模式。当 tech doc 就绪时，plan 阶段从分钟级降到秒级（M2 只用 10 分钟完成全流程）。文档迫使在编码前想清楚方案，减少返工。
