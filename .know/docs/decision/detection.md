# Detection: Grep 模式匹配选型

## 背景

decay 需要检测源代码中的质量问题（unwrap/panic、空 catch、SQL 注入、嵌套循环、TODO/FIXME 等）。需要一种跨语言、零依赖、可扩展的检测方法。

## 选项

| 方案 | 优势 | 劣势 |
|------|------|------|
| Grep 模式匹配 | 语言无关、零外部依赖、实现简单、扩展容易 | 无语法感知、误报率较高、无法理解作用域和类型 |
| AST 解析 | 精确定位、理解代码结构、低误报 | 需要每种语言的 parser、依赖重、维护成本高 |
| ML/语义分析 | 可发现复杂模式、适应性强 | 需要训练数据、推理延迟高、CLI 工具不适合 |

## 决策

选择 grep 模式匹配。decay 的核心定位是"快速、轻量、跨语言"的健康体检工具，而非精确的 lint 工具。grep 方式满足以下需求：

1. **语言无关** — 同一套模式可检测 Rust、Python、JavaScript、Go 等多种语言中的类似问题
2. **零依赖** — 不需要安装 tree-sitter、语言 SDK 或外部工具
3. **可接受的精度** — 对于项目级健康评估，漏检或误检个别实例不影响整体判断

## 实现细节

### 模式匹配引擎 (`dimension/helpers.rs`)

```rust
pub fn count_pattern_matches(lines: &[String], patterns: &[&str]) -> Vec<PatternHit>
```

核心扫描函数：逐行遍历，跳过注释行，对每行检查所有 patterns（`contains` 匹配）。

### 误报缓解机制

1. **注释过滤** — `is_comment()` 跳过 `//`, `#`, `///`, `/*`, `*` 开头的行

2. **测试代码过滤** — `mark_test_lines()` 标记 `#[cfg(test)]` 和 `#[test]` 块内的代码，`count_pattern_matches_filtered()` 接受 test_mask 参数跳过测试代码中的匹配

3. **文件上下文检测** — `detect_file_context()` 根据路径和内容识别文件类型：
   - **Test** — `/test/`, `/tests/`, `_test.`, `.test.`, `/spec/` 等路径
   - **FFI** — `/ffi/`, `/bindings/`, `/sys/` 路径，或 >= 3 个 `extern "C"` 块
   - **Parser** — `/parser`, `/lexer`, `/ast/` 路径
   - **Builder** — 同时有 `fn new(`, `fn build(`, `-> Self` 的文件
   
   维度实现可根据 FileContext 降级或跳过某些检测（如 FFI 代码中的 unsafe 不报 Warning）

4. **生成文件过滤** — `is_generated_file()` 跳过 .lock、.min.js、.md、.json、.yaml、.toml 等文件

5. **语言过滤** — filter pipeline 在收集阶段就过滤掉非主要语言的文件，减少无关文件的噪声

### 典型检测模式

| 维度 | 检测模式 | 目的 |
|------|---------|------|
| observability | `.unwrap()`, `.expect(` | panic 风险 |
| reliability | `"SELECT`, `"INSERT`, `"UPDATE` + 字符串拼接 | SQL 注入 |
| performance | `for.*{.*for.*{` | 嵌套循环 |
| maintainability | `TODO`, `FIXME`, `HACK` | 技术债标记 |
| observability | `catch {}`, `except: pass` | 静默错误吞噬 |

## 后果

- 新增检测模式只需向 patterns 数组添加字符串，成本极低
- 精度依赖模式设计质量——过于宽泛会误报，过于具体会漏报
- 无法检测跨行模式（如分散在多行的 SQL 拼接）
- 不理解变量类型和作用域（如 `unwrap()` 对 `Option` 和自定义方法不区分）
- 对于需要精确分析的场景，用户应配合专用 lint 工具（clippy, eslint 等）

## 状态

已确认 — 项目初始
