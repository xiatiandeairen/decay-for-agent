# 位置精度提升 技术方案

## 1. 背景

PRD 要求在已有采集数据中提取行号信息填入 Action 的 `target.line_range` 和 `target.symbol`。

## 2. 方案

### 2.1 performance — 直接填充

`nest_details: (path, line_no, depth)` 已有 line_no。

```rust
// before
target: Target { file: path, line_range: None, symbol: None }
// after
target: Target { file: path, line_range: Some((line_no as u32, line_no as u32)), symbol: None }
```

### 2.2 maintainability — 扩展 long_func_details

```rust
// before: (path, func_name, func_len)
// after:  (path, func_name, func_len, start_line)
target: Target { file: path, line_range: Some((start, start + len)), symbol: Some(func_name) }
```

start_line 从 `func_positions` 的 `(name, start)` 直接传入。

### 2.3 observability — 扩展 unwrap_details

```rust
// before: Vec<(String, usize)>          — (path, count)
// after:  Vec<(String, usize, Vec<u32>)> — (path, count, line_numbers)
```

analyze 内 unwrap 检测时 push `(i + 1) as u32`。
Action target 用 `(first_line, last_line)` 覆盖范围。

### 2.4 reliability — 扩展 injection/secret_details

```rust
// before: Vec<(String, String)>       — (path, pattern)
// after:  Vec<(String, String, u32)>  — (path, pattern, line_no)
```

analyze 内 `for line in &sf.lines` 改为 `for (i, line) in sf.lines.iter().enumerate()`，检测匹配时记录 `(i + 1) as u32`。

### 2.5 不提升的维度

| 维度 | 原因 | target.file 精度 |
|------|------|-----------------|
| structural | 项目级指标（文件数、深度） | "src/" 或 "." |
| complexity | 文件级指标（size_bytes） | 具体文件路径 |
| fragility | 文件级指标（churn） | 具体文件路径 |
| quality | 项目级指标（测试比例） | "." |

这些维度的 target.file 已经是最佳精度，无行级数据可用。

## 3. 文件变更清单

| Action | File | 变更 |
|--------|------|------|
| modify | `src/dimension/performance.rs` | line_no → line_range |
| modify | `src/dimension/maintainability.rs` | long_func_details 加 start_line，填 line_range + symbol |
| modify | `src/dimension/observability.rs` | unwrap_details 加 Vec<u32>，填 line_range |
| modify | `src/dimension/reliability.rs` | injection/secret_details 加 u32 line_no，填 line_range |

## 4. 迭代记录

- 2026-04-15: 初始方案
