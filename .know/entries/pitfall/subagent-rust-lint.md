# subagent 生成的 Rust 代码需要 lint 修复

## Symptoms

subagent（sonnet）生成的 Rust 代码编译通过但 clippy 报 collapsible-if、cargo fmt 格式不符。集成后 CI 会失败。

## Root cause

subagent 没有运行 cargo fmt/clippy，且对 rustfmt 的换行规则不够精确。

## Lesson

集成 subagent 生成的 Rust 代码后，必须运行 cargo fmt + cargo clippy -- -D warnings 再做后续验证。
