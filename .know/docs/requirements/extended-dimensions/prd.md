# 扩展维度

## 1. 问题

### 痛点

原有三个维度（structural / complexity / fragility）只覆盖结构和变更模式，无法捕捉代码可维护性、可观测性、测试保障、安全可靠性和性能等方面的腐败信号。这些维度是完整代码健康画像不可或缺的部分。

- **可维护性盲区**: 重复代码蔓延、文件/函数过长、死代码积累无法检测
- **可观测性盲区**: 日志缺失、错误吞没、panic/unwrap 滥用无法发现
- **测试保障盲区**: 测试缺失或质量低，修改代码的信心不足
- **安全可靠性盲区**: unsafe 代码、SQL 拼接、硬编码密钥等风险无法识别
- **性能盲区**: 嵌套循环、不必要拷贝、同步阻塞调用无法检测

### 影响范围

影响所有使用 decay 的开发者和 AI agent。这些问题几乎每个项目都会遇到，是技术债的重要组成部分。

### 为什么现在做

v3 扩展维度是核心目标，5 个新维度互补构成完整的代码健康画像：maintainability（好不好改）、observability（能不能发现问题）、quality_assurance（改了有没有保障）、reliability（安不安全）、performance（快不快）。

## 2. 目标用户

| 角色 | 场景 | Before | After |
|------|------|--------|-------|
| 开发者 | 想知道代码是否在变得难维护 | 无法从 decay 获取可维护性信息 | 看到 maintainability 0-100 分 + 具体问题 |
| 开发者 | 想知道项目出问题时能否快速定位 | 无法从 decay 获取可观测性信息 | 看到 observability 0-100 分 + 具体问题 |
| 开发者 | 想知道项目测试保障有多强 | 无法从 decay 获取测试质量信息 | 看到 quality_assurance 0-100 分 + 具体建议 |
| 开发者 | 想知道代码有多少安全隐患 | 无法从 decay 获取安全性信息 | 看到 reliability 0-100 分 + 具体风险点 |
| 开发者 | 想知道代码有多少性能隐患 | 无法从 decay 获取性能反模式信息 | 看到 performance 0-100 分 + 具体问题位置 |
| AI agent | 根据 decay 输出做决策 | 只有 3 维度信息，画像不完整 | 8 维度完整画像，决策更准确 |

## 3. 核心假设

- **假设**: 基于文件内容分析计算 5 个新维度分数 → 用户获得完整代码健康画像
- **验证方式**: 运行 `decay`，输出包含 8 个维度分数 + 诊断 + 处方

## 4. 方案

### maintainability（可维护性）
- **Before**: 无法检测重复代码、长文件/函数、死代码 → **After**: 输出 0-100 分，诊断具体问题，提供重构处方

### observability（可观测性）
- **Before**: 无法检测日志缺失、错误吞没、panic/unwrap 滥用 → **After**: 输出 0-100 分，诊断具体问题，提供改进处方

### quality_assurance（质量保障）
- **Before**: 无法检测测试文件比例、测试/源码行数比、断言密度 → **After**: 输出 0-100 分，诊断测试不足，提供补测试处方

### reliability（可靠性与安全）
- **Before**: 无法检测 unsafe 代码、SQL 拼接、硬编码密钥、依赖膨胀 → **After**: 输出 0-100 分，诊断安全风险，提供修复处方

### performance（性能）
- **Before**: 无法检测嵌套循环、不必要拷贝、同步阻塞调用 → **After**: 输出 0-100 分，诊断性能反模式，提供优化处方

### 任务追踪

| 任务 | Tech | 状态 | 备注 |
|------|------|------|------|
| maintainability | [tech](tech.md) | 已完成 | — |
| observability | [tech](tech.md) | 已完成 | — |
| quality_assurance | [tech](tech.md) | 已完成 | — |
| reliability | [tech](tech.md) | 已完成 | — |
| performance | [tech](tech.md) | 已完成 | — |

## 5. 验收标准

- `decay` 输出包含 maintainability 分数（0-100），重复代码多则分数低
- 长文件/函数多 → maintainability 分数低 + 诊断列出具体文件
- `decay` 输出包含 observability 评分，无日志/错误处理 → 分数低
- 大量 unwrap/panic → observability 诊断指出具体文件
- `decay` 输出包含 quality_assurance 评分，无测试 → 分数极低
- 断言密度低 → 诊断指出测试质量问题
- `decay` 输出包含 reliability 评分，大量 unsafe/eval → 分数低
- SQL 拼接/shell 注入模式 → critical 级诊断
- `decay` 输出包含 performance 评分，大量嵌套循环 → 分数低
- 高 clone 密度 → 建议使用引用

## 6. 排除项

- AST 级重复检测（行级哈希足够）
- 语言特定的死代码分析（需要编译器支持）
- 运行时可观测性检查（需要部署环境）
- APM/监控工具集成检测（需要运行时配置分析）
- 覆盖率百分比（需要运行测试工具）
- 变异测试（需要运行测试框架）
- CVE 数据库查询（需要网络访问）
- 依赖版本过时检测（需要 registry API）
- 运行时性能分析（需要 profiler）
- 算法复杂度分析（需要 AST 级控制流分析）
