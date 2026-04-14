# 可靠性与安全维度

## 1. 问题
unsafe 代码、缺少输入验证、过时依赖等问题影响系统可靠性和安全性。现有维度不检测这些信号。

## 2. 核心假设
基于文件内容和依赖文件分析检测安全/可靠性信号 → 用户能量化"代码有多可靠和安全"。

## 3. 指标
| 指标 | 检测方式 |
|------|---------|
| unsafe 代码密度 | grep unsafe 块（Rust）/ eval/exec（Python/JS） |
| 输入验证密度 | grep validate/sanitize/escape/parameterize |
| 依赖数量 | 解析依赖文件统计直接依赖数 |
| 已知风险模式 | grep SQL 拼接/shell exec/硬编码密钥模式 |

## 4. 验收标准
- reliability 维度输出 0-100 评分 + 诊断 + 处方
- 大量 unsafe/eval → 分数低 + 诊断列出具体位置
- SQL 拼接/shell 注入模式 → critical 级诊断

## 5. 排除项
- CVE 数据库查询（需要网络）
- 依赖版本过时检测（需要 registry API）
