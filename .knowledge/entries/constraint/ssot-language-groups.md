# 源文件扩展名：LANGUAGE_GROUPS 是唯一定义源

## 约束

`filter_pipeline::LANGUAGE_GROUPS` 是"什么是源代码文件"的唯一定义。其他需要判断源文件的位置必须从此派生，禁止独立维护扩展名列表。

## 派生方式

```rust
// filter_pipeline.rs — 唯一定义
pub const LANGUAGE_GROUPS: &[LanguageGroup] = &[...];
pub fn is_known_source_ext(ext: &str) -> bool { ... }

// util.rs — 派生
pub fn is_source_file(path: &str) -> bool {
    filter_pipeline::is_known_source_ext(&ext)
}
```

## 背景

曾存在 `util::SOURCE_EXTENSIONS` 和 `filter_pipeline::LANGUAGE_GROUPS` 两套独立列表，新增语言时只改一处导致 data_store 加载的文件集与 filter 不一致。
