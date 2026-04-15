# Dimension trait: score/diagnose 拆分导致重复查询

## 问题

原 `Dimension` trait 有 `score()` 和 `diagnose()` 两个独立方法，`evaluate()` 作为默认实现依次调用两者。这导致：

- DB 维度（structural/complexity/fragility）：每条 SQL 执行两次
- SourceFile 维度（其余 5 个）：`analyze()` 遍历全部源文件两次

## 根因

接口粒度过细（interface granularity）——按职责拆了两个方法，但调用方总是同时需要两者，且两者共享中间数据。

## 修复

合并为单一 `evaluate()` 作为 required method：
- DB 维度：一次查询，结果同时用于 score 和 issues
- SourceFile 维度：一次 `analyze()` 调用，从中间结构派生 score + issues

## 规则

设计 trait 接口前先确认调用方的使用模式。如果两个方法总是一起调用且共享数据，不应拆开。
