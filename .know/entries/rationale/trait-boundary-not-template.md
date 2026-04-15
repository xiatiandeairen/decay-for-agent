# Trait 定义生命周期边界，不定义实现模板

## Context

v3 重构将 8 个维度统一到 `Dimension` trait。评估逻辑表面相似（查数据→算指标→判阈值→出诊断），但中间数据形态各异：

- structural / complexity / fragility：DB 查询（文件表、git_changes 表）
- maintainability / observability / performance：文件内容逐行扫描
- quality：混合（文件系统分类 + 内容分析）
- reliability：混合（文件内容 + 依赖文件解析）

## Decision

trait 只约束输入（`&DataStore`）和输出（`DimensionResult`），不提供模板方法。

```rust
pub trait Dimension: Send + Sync {
    fn name(&self) -> &'static str;
    fn evaluate(&self, store: &DataStore) -> Result<DimensionResult>;
}
```

## Why not template methods

强制抽取 `score_from_ratio()` / `collect_metrics()` 等模板方法会导致：

1. 参数签名不断膨胀以适配不同维度的中间数据
2. 部分维度被迫构造无意义的中间结构体来满足模板接口
3. score 和 diagnose 共享中间数据的优化被模板方法的调用顺序限制

## Guideline

- trait 定义**生命周期边界**（何时调用、输入什么、输出什么）
- 实现者自决**内部结构**（查询策略、中间数据、score/diagnose 的融合方式）
- 如果多个实现出现真正的重复代码（≥3 处相同逻辑），提取为自由函数而非 trait 默认方法
