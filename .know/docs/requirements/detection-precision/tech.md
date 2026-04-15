# 检测精度 技术方案

## 1. helpers.rs — test block 追踪

新增 `mark_test_lines(lines) -> Vec<bool>` — 返回每行是否在测试块内。

追踪逻辑：
- 遇到 `#[cfg(test)]` → 下一个 `mod` 开始到对应 `}` 结束为测试块
- 遇到 `#[test]` → 下一个 `fn` 开始到对应 `}` 结束为测试块
- 使用大括号计数追踪块边界

## 2. observability.rs — 跳过测试代码

- `count_pattern_matches` 新增 `test_mask: Option<&[bool]>` 参数
- 当 test_mask 存在且对应行为 true 时跳过
- observability 传入 test_mask，其他维度传 None

## 3. reliability.rs — 错误消息过滤

SQL injection 检测增加排除条件：
- 行内包含 `bail!`, `anyhow!`, `panic!`, `eprintln!`, `error!`, `warn!`
- 或行内包含常见错误消息词：`"failed"`, `"error"`, `"unable"`, `"could not"`

## 4. 文件变更清单

| Action | File | 变更 |
|--------|------|------|
| modify | `src/dimension/helpers.rs` | mark_test_lines + tests |
| modify | `src/dimension/observability.rs` | 传入 test mask |
| modify | `src/dimension/reliability.rs` | SQL injection 假阳性过滤 + test |
