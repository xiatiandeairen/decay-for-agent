# 检测精度 — 区分测试/生产代码，消除误报

## 1. 问题

- observability 对 `#[cfg(test)]` 中的 unwrap 报警，测试代码使用 unwrap 是正常做法
- reliability 对错误消息中的 format! + SQL 关键词误报为 SQL 注入

## 2. 方案

- helpers 新增 test block 检测，observability 跳过测试代码中的 unwrap
- reliability SQL injection 检测排除错误消息上下文

## 3. 验收标准

- `#[cfg(test)]` 模块内的 unwrap 不计入 observability 评分
- `#[test]` 函数内的 unwrap 不计入 observability 评分
- `format!("DELETE failed...")` 等错误消息不触发 SQL injection 报警
- 真正的 SQL 拼接仍被检测
- `cargo test` 全部通过
