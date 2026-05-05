# 扫描对象与局部膨胀价值 设计说明

> 日期: 2026-05-04
> 目的: 彻底回答两个问题
> 1. `decay` 到底在扫描谁，如何优雅、可扩展、可维护地定义扫描对象
> 2. `decay` 当前围绕“局部膨胀”到底解决了什么价值，还差什么

## 1. 先把问题说清楚

当前 `decay` 最大的概念混乱，不是 metric 算法，而是“扫描对象”定义不稳定。

它现在的实际行为是:

- 从项目根目录递归找 `.rs` 文件
- 排除 `target`、`.git`
- 额外参考根 `.gitignore`
- 用户再用 `--exclude` 手工补排除

这个实现能工作，但产品定义是不稳定的。因为它混淆了 3 个不同概念:

1. **发现范围**: 文件系统里有哪些文件候选
2. **分析对象**: 哪些文件应该进入复杂度分析
3. **决策对象**: 哪些结果应该参与“这次改动是否变坏”的裁决

如果这三层不拆开，就会反复出现同一类问题:

- 为什么 `tests/fixtures` 也被扫了
- 为什么 `examples` 会污染热点
- 为什么用户要自己知道何时 `--exclude`
- 为什么 README / PRD / 实现长期不一致

## 2. 扫描对象到底是谁

从产品视角，不应该把“扫描对象”定义成“所有 `.rs` 文件”。

正确的定义应该是:

> `decay` 的默认扫描对象，是 **当前项目中会进入日常维护和提交决策的 Rust 源代码**，而不是仓库里所有 Rust 文件。

这句话有两个关键词:

- **日常维护**
- **提交决策**

因此，下列文件虽然也是 Rust 文件，但默认不一定应该参与主流程裁决:

- `examples/`
- `tests/`
- `tests/fixtures/`
- `benches/`
- 生成代码
- vendor / third_party 镜像副本

它们不是“非法输入”，但默认不该和主维护面混在一起。

## 3. 扫描对象的正确模型

要彻底解决问题，需要从“按路径排除”升级为“按角色建模”。

### 3.1 三层模型

#### A. Candidate

候选文件。意思是:

- 文件存在于项目内
- 扩展名属于语言支持集合
- 有资格进入下一步分类

对 Rust 来说，Candidate 只是 “`.rs` 文件 + `build.rs`”。

#### B. Source Role

源文件角色。意思是:

- 这个文件在项目里扮演什么职责

对 Rust v0.1，建议至少定义这些角色:

- `prod`
  - `src/lib.rs`
  - `src/main.rs`
  - `src/**/*.rs`
  - 二进制 target 对应源码
  - `build.rs`
- `test`
  - `tests/**/*.rs`
  - 单元测试辅助模块
- `example`
  - `examples/**/*.rs`
- `bench`
  - `benches/**/*.rs`
- `fixture`
  - `tests/fixtures/**/*.rs`
  - 其他明确标记为 fixture / sample / golden 的目录
- `generated`
  - 生成代码或 vendored 副本
- `unknown`
  - 无法可靠归类，但又是 Rust 文件

#### C. Scan Policy

扫描策略。意思是:

- 不同命令默认应该分析哪些角色

例如:

- `check` / `diff` 默认只看 `prod`
- `init` 默认只为 `prod` 建 baseline
- `hotspots` 默认看 `prod`，但允许切到 `prod+test` 或 `all`

这样产品行为才稳定。

## 4. 推荐方案: 角色驱动的扫描范围系统

### 4.1 默认策略

v0.1 应明确改成:

- 默认扫描范围: `prod`
- 默认不纳入:
  - `test`
  - `example`
  - `bench`
  - `fixture`
  - `generated`

为什么这样最对:

- `decay` 当前主价值是 commit 前裁决
- commit 前最应该裁决的是主维护面代码
- 非主维护面代码会稀释信号
- 真正关心测试/示例复杂度的用户，应该显式打开，而不是被动接收

### 4.2 显式 scope，而不是无限补 `--exclude`

CLI 应从“排除思维”升级为“scope 思维”。

建议新增统一参数:

```text
--scope prod
--scope prod,test
--scope all
```

并保留 `--exclude` 作为二级修正，而不是主机制。

推荐语义:

- `decay init` 默认 `--scope prod`
- `decay check` 默认 `--scope prod`
- `decay diff` 默认读取快照自带 scope，不允许和快照 scope 漂移
- `decay hotspots` 默认 `--scope prod`

可选增强:

```text
--include-role test
--include-role example
--exclude path-pattern
```

但对 v0.1/v0.2 来说，`--scope` 已经足够。

### 4.3 Rust 的分类来源

角色分类不要靠一堆硬编码字符串散落在 walker 里，应该有明确优先级。

推荐顺序:

1. **Cargo metadata / target 信息**
   - 最权威
   - 可识别 lib/bin/example/test/bench/build-script
2. **约定目录回退**
   - `src/`
   - `tests/`
   - `examples/`
   - `benches/`
   - `tests/fixtures/`
3. **用户配置覆盖**
   - 显式标记某路径属于 `generated` 或 `fixture`
4. **unknown**
   - 不能确定时归到 `unknown`

这比单纯靠 `.gitignore` 和 `--exclude` 健壮得多，因为:

- `.gitignore` 解决的是“文件该不该被 Git 跟踪”
- 不是“文件在产品里属于哪个维护角色”

## 5. 维护性设计

### 5.1 模块职责拆分

当前 `walk.rs` 同时承担了:

- 文件发现
- `.gitignore` 处理
- 排除规则
- 路径匹配

后续应拆成三层:

- `discover`
  - 只负责找 candidate files
- `classify`
  - 只负责把文件映射成 source role
- `scope`
  - 只负责根据命令策略筛选角色

这样做的好处:

- walker 不再承载产品语义
- 新语言只要换 classifier，不用重写 CLI 语义
- 测试也能分别验证“找到了文件”“分类对不对”“scope 对不对”

### 5.2 数据结构建议

建议引入:

```rust
pub enum SourceRole {
    Prod,
    Test,
    Example,
    Bench,
    Fixture,
    Generated,
    Unknown,
}

pub struct CandidateFile {
    pub abs_path: PathBuf,
    pub rel_path: String,
    pub role: SourceRole,
}

pub struct ScanScope {
    pub include_roles: BTreeSet<SourceRole>,
    pub exclude_patterns: Vec<String>,
}
```

并让 snapshot 持久化 scope 摘要，例如:

- `language = rust`
- `scope = prod`
- `roles_included = ["prod"]`

这样 `diff` 才不会把两次不同 scope 的结果误认为同一基线。

### 5.3 向后兼容策略

为了不一次性打碎 CLI，可以分两步:

#### 第一阶段

- 新增 `--scope`
- 默认改成 `prod`
- `--exclude` 保留
- 热点和 baseline 文案明确显示实际 scope

#### 第二阶段

- snapshot 持久化 scope
- 若 `diff` 发现 scope 不一致，直接报错并提示重建 baseline
- 逐步把“推荐用 `--exclude`”改成“推荐选对 `--scope`”

## 6. 为什么这个方案更优雅

优雅，不是参数更多，而是语义更稳定。

这个方案优雅在 4 点:

1. **产品语义清楚**
   - 扫描的是“主维护面源码”，不是“目录里所有 Rust 文件”
2. **扩展方式统一**
   - 后续支持 Python/TS 也能复用 Candidate → Role → Scope 这套模型
3. **维护成本低**
   - 新增一个角色，不需要到处补路径排除
4. **对用户心智更友好**
   - 用户理解 `prod / all` 比理解十几个排除模式简单得多

## 7. 当前价值到底在哪里

在“局部膨胀”这个方向上，`decay` 当前已经打中的价值，不是“复杂度分析”，而是:

> 在 AI 改完代码之后，给用户一个独立于 AI 自评、可重复、足够快的 commit 前复杂度裁决。

它当前已经成立的价值点有 3 个:

### 7.1 它比人肉读 diff 更稳定

人肉读 diff 的问题不是完全做不到，而是:

- 慢
- 依赖注意力
- 容易漏掉局部嵌套/分支膨胀

`decay check` 把这件事变成了重复性流程。

### 7.2 它比 AI 自评更可信

AI 自评的结构性问题是:

- 会为自己写出的代码辩护
- 同一份 diff 输出不稳定

`decay` 至少把“这次是不是更复杂了”从主观辩论改成了可重复的 metric delta。

### 7.3 它抓住了最容易在 AI 工作流里失控的一类问题

局部膨胀之所以是正确切口，是因为它对应最常见的 AI 行为:

- 修 bug 加一层 `if`
- 为了安全再套一层 guard
- 不拆分职责，继续塞进原函数

这类退化频率高、检测确定、又适合在 commit 前拦截。

## 8. “局部膨胀”现在做得不够的地方

这里要区分两类“不够”:

- **检测边界不够**
- **产品价值兑现不够**

### 8.1 检测边界不够

当前系统主要识别的是:

- 同一个函数在 metric 上变大了

但它还识别不了很多“看起来没膨胀，实际上也更坏了”的情况:

1. **改名重写 / 挪动函数**
   - 会退化成“删了一个，新增一个”
   - 连续性断掉
2. **把复杂度摊平到多个小函数**
   - 单函数分数可能下降
   - 但整体职责和重复可能更坏
3. **重复扩散**
   - AI 很常见的“复制一份再改名”
   - 不属于当前 A 类检测覆盖面
4. **闭包复杂度挂到外层函数**
   - 行为上能工作
   - 但对定位不够精确

所以当前“局部膨胀”只覆盖了退化空间的一部分，而且是最容易量化的那一部分。

### 8.2 产品价值兑现不够

这个更关键。

当前系统已经证明:

- 能发现退化
- 能输出 delta

但还没有完全证明:

- 这些输出是否足够频繁地改变真实决策
- 用户是否会长期信任并养成习惯

具体差在 5 点:

1. **扫描对象还不稳定**
   - 主维护面和辅助代码混在一起
   - 信号被稀释
2. **热点视图仍然过重**
   - 容易把产品理解成 complexity linter
   - 而不是 regression judge
3. **弱信号 metric 仍在拖后腿**
   - `params` 这类指标行动性不够强
4. **缺少“真实漏判被抓住”的强案例链**
   - 受控退化证明机制成立
   - 但没完全证明不可替代价值
5. **缺少更直接的行动建议**
   - 现在告诉你“变糟了”
   - 但还不够告诉你“这次值不值得立即返工”

## 9. 对“局部膨胀”最该补的不是更多 metric

短期最该补的不是继续加 metric，而是把主链路做实。

优先级建议:

1. **先修扫描对象定义**
   - 默认 scope 改成 `prod`
   - 解决信号污染
2. **再修输出心智**
   - 强化 `check` / `diff`
   - 弱化“长热点清单”的主地位
3. **再清理低行动性信号**
   - 重新校准或降权 `params`
4. **最后再扩检测面**
   - 如重复扩散、重命名追踪、职责扩散

原因很简单:

- 如果扫描对象都不对，任何 metric 都会被噪音放大
- 如果主工作流心智不对，再好的算法也会被用户误解

## 10. 最终结论

### 关于扫描对象

`decay` 不应该再把“扫描对象”定义成“仓库里的所有 Rust 文件”。

应该定义成:

> **默认扫描当前项目的主维护面源码（prod source），并允许用户显式扩展到 test/example/bench/all。**

这是唯一既符合产品定位，又可扩展、可维护的方案。

### 关于当前价值

`decay` 当前真正成立的价值是:

> **在 AI 改完代码后，用足够快、可重复的方式，对局部复杂度膨胀做一次 commit 前裁决。**

### 关于“不够”

它现在不够的地方，不是“还没有更多 metric”，而是:

- 扫描对象定义不稳定
- 局部膨胀覆盖面还只是一部分
- 价值证据还没有强到证明“用户会长期依赖它”

所以当前最应该做的，不是继续补零散规则，而是把“扫描范围模型”和“commit 前裁决主链路”彻底定型。
