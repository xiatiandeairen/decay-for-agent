# Rust inline test 不等于 test file

## 陷阱

`#[cfg(test)]` 标记的是同一源文件内的 test section，不是独立的 test file。如果将整个文件归为 test file，会导致：

- source_files 计数为 0（所有 Rust 文件都有 inline test）
- test ratio 虚高至 100%
- quality_assurance 维度评分完全失真

## 正确做法

```rust
// 独立测试文件 (tests/test_foo.rs) → 整个文件计为 test
// 含 inline test 的源文件 → 分离行数
fn split_inline_test_lines(lines: &[String]) -> (usize, usize) {
    // 找到 #[cfg(test)] 位置，之前的行数归 source，之后归 test
}
```

## 适用范围

`dimension/quality.rs` 中的文件分类逻辑。类似语言（Go 的 _test.go 是独立文件不受影响）。
