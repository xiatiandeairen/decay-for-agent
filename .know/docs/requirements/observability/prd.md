# 可观测性与恢复能力维度

## 1. 问题
项目缺乏日志、监控、错误处理时，线上问题难以定位和恢复。现有维度不检测这类腐败信号。

## 2. 核心假设
基于文件内容模式匹配检测日志/错误处理密度 → 用户能量化"出问题时能否快速发现和恢复"。

## 3. 指标
| 指标 | 检测方式 |
|------|---------|
| 日志调用密度 | grep log/logger/println/print/console.log 等 |
| 错误处理覆盖率 | grep try/catch/Result/unwrap/expect/rescue/except |
| panic/unwrap 密度 | grep panic!/unwrap()/force unwrap |
| 配置硬编码 | grep 硬编码 URL/端口/密钥模式 |

## 4. 验收标准
- observability 维度输出 0-100 评分 + 诊断 + 处方
- 无日志/错误处理的项目 → 分数低
- 大量 unwrap/panic → 诊断指出具体文件

## 5. 排除项
- 运行时可观测性检查（需要部署环境）
- APM/监控工具集成检测
