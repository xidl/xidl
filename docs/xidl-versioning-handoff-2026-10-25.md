# XIDL API 版本化开发交接文档（2026-10-25）

> 本文自包含：读完即可接手版本化方向的继续开发。包含背景、设计原则、架构、
> 模块导航与代码范例、已完成/未完成清单、开发流程与门禁、常见坑、以及一轮
> 「进一步思考后更优的设计」提案。仓库：https://github.com/xidl/xidl。

---

## 1. 背景

xidl 是一个 Rust 编写的 IDL 编译器（`xidlc`），一份 IDL 契约同时生成：

- TypeScript REST client/server/zod（`typescript-rest` 目标）
- Rust axum server/client（`rust-axum` 目标）
- OpenAPI 每版本一份文档（`openapi` 目标）
- Go / Python / JSON-RPC 等其它目标

版本化方向的目标：**一份 IDL 声明全部历史，编译器推导出每一个在支持的 API
版本**。业务与存储只认领域模型（内部最新，下称 Head）；对外的各个版本由相邻
两版之间的一对转换函数逐级得到（Stripe 式链式翻译，Polar 式按版本导入）。

产品侧的诉求来自真实多版本 API 的治理：老客户不能因为平台升级而悄悄变格式；
新版本不能丢失老版本仍然读取的数据；跨语言的生成物必须对同一份契约给出完全
一致的行为。

## 2. 三条公理（整个设计的前提）

1. **已发布的形状冻结**。任何版本的 wire 形状永不改动；要变就发新版本。
2. **信息只丢弃、不编造**。旧版本装不下的值，转换层带 `UnsupportedProjection`
   拒绝（含 `typePath` 定位），绝不猜一个值出来。
3. **存储只前进**。老版本仍然读取的数据由应用存储继续保存，直到最后一个需要
   它的版本退休；退休面由生成的 `NEEDS` 表报告。

推导出的验收标准（每对 up/down 都要写成测试）：

- 往返规则一：`up(down(prev, s), s) == s`（读到什么写回什么，系统不变）
- 往返规则二：`down(…, up(v, s)) == v`（写进什么读回什么）
- 签名固定：`down(prev, head)` / `up(view, cur)`，纯函数、无 IO、确定性
  （幂等指纹与 Webhook 字节固化依赖这一点）
- 「不编造」检查：输出里每个字段都能追溯到 `prev`、`head`、`cur` 或存储 reader

## 3. 变化目录（E1–E14）与覆盖矩阵

这是需求的「变化目录」：每种破坏性变化怎么转换、由谁负责。完整定义见 PR #382
的描述与本文各语料章节；速览：

| 记号          | 例子                                                                                                | 语义                                          |
| ------------- | --------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| 🟢 机械       | E2 字符串拆结构                                                                                     | 生成器自动                                    |
| 🔵 转换函数   | E3 合并、E5 枚举值拆字段、E6 精度、E7 一变多、E8 默认值翻转、E10 错误细分、E11 事件拆分、E12 新必填 | `down(prev, head)` / `up(view, cur)` 应用钩子 |
| 🟡 需保留数据 | E1 删除字段、E3                                                                                     | 存储保留，NEEDS 报告，退休才删                |
| 🟠 业务分支   | E13 删除接口                                                                                        | 退休接口用新操作重实现，保留到退休            |
| ⚪ 新接口     | E9 分页变更、E14 路径变更                                                                           | 只在新版本提供；版本只由头决定                |
| 🔴 无法表示   | E4 枚举新增值                                                                                       | 422，带 typePath                              |

**语料覆盖现状**（`bdd/features/data/` + `bdd/boilerplate/`，双语言）：

| 语料                                   | 覆盖                                                                                                                                                                                                                                                                                   |
| -------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `versioned_chain`（booking）           | E1（agenda 退休进存储）、E7（items↔agenda 不可逆拆包）、机械 rename、`@default` implied、幂等指纹（跨版本重放）、v1 PATCH、缺 owner、locked                                                                                                                                            |
| `versioned_transforms`（profile/目录） | E2（recipient→to）、E3（名字合并）、E4（chat 拒绝）、E5（read_at↔status）、E6（epoch↔RFC3339，严格解析）、E7（note↔notes，caller-aware 写规则）、E8（fallback 翻转）、E9（roster 整表投影）、E10（Sealed 409 降级）、E12（必填 tier）；HTTP 旅程含协商、404 回显、空表 500、逐元素拒绝 |
| `versioned_events`（E11）              | 事件拆分：双半边下行都映射旧名、事件 id 各自保留、上行声明取哪一半、旧合同没有的值在 hook 前拒绝                                                                                                                                                                                       |
| `versioned_adapters_business`          | 容器元素退休（Address.gate 在 sequence/map 里逐元素恢复/拒绝/OMIT）、继承结构（Signed:Labelled 全类型 hook 现状）、union 降级运行时（count→text）、错误降级表、@replaces、更新流（PATCH + 存储袋）                                                                                     |

未覆盖：**E13（退休接口用新操作重实现）**、E14 的路径变更部分（E14 的「版本只
由头决定」语义已被协商旅程覆盖）。

## 4. 总体架构

```
IDL 源码
  │ tree-sitter-idl（具体语法树）
  ▼
typed_ast            ── 与 tree-sitter AST 一一对应（不许合成节点）
  ▼
hir（xidl-parser）   ── 语义模型；versioning/ 做逐版本投影
  ▼                     每个 @version 成员一个模块（v2026_01_01 …）
rest_hir             ── HTTP 语义（路径/参数/表示/异常/流）
  ▼
adapter planner      ── xidlc/src/generate/adapter/
  │   collect/   每版本的类型/操作形状（VersionSurface）
  │   planner/   相邻版本 diff → TypePlan/OperationPlan（up/down 各一份）
  │   updates/   更新流（PATCH）的存储袋推导
  │   wire_layout/  每版本 wire 校验形状
  ▼
语言渲染器（Rust 预处理 + minijinja 渲染，模板只做取数与渲染）
  ├─ typescript_rest/  （.adapters.ts、client、server、zod、iface.zod）
  └─ rust_axum/        （.adapters.rs、server、client、请求提取器）
```

关键不变式（AGENTS.md 强制）：

- `typed_ast` 与 tree-sitter AST 一一对应
- 生成器实现用 Rust 做预处理/校验/上下文组装，`minijinja` 只渲染
- 手拼字符串片段 < 20 字符；解析/语义/生成/传输/CLI 各自分层
- 手写 `src/**/*.rs` 禁止 unwrap/expect/panic 控制流；错误信息必须点名失败的契约

## 5. 版本适配层（生成物契约）

以 `versioned_transforms` 为例。IDL
片段（`bdd/features/data/versioned_transforms.idl`）：

```idl
@version(in = "header", name = "X-Transform-Version")
enum ApiVersion {
    @rename("2026-01-01") v2026_01_01,
    @rename("2026-04-01") v2026_04_01,
    @rename("2026-07-01") v2026_07_01,
};

struct Profile {
    string id;
    string title;
    @removed(ApiVersion::v2026_07_01) string status;   // E5：退休进……
    @added(ApiVersion::v2026_07_01) @optional string read_at; // ……一个可推导字段
    @removed(ApiVersion::v2026_04_01) @optional string note;  // E7：一变多
    @added(ApiVersion::v2026_04_01) sequence<Note> notes;
    @added(ApiVersion::v2026_04_01) @optional @default(false) boolean fallback; // E8
};
```

生成的适配器（TS 侧 `versioned_transforms.adapters.ts` / Rust 侧
`.adapters.rs`） 提供：

### 5.1 VersionAdapters——应用必须实现的纯钩子

```ts
// 生成的接口（TS）。Rust 侧是 trait，语义相同。
export interface VersionAdapters {
  // 只有需要业务判断的相邻步骤才有钩子；机械步骤不问应用。
  profileDowncastToV20260401(
    source: types.v2026_07_01.Profile,
    stored: Pick<StoredValues, "profileStatus" | ...>,
  ): VersionedResult<ProfileDowncastDecisionFromV20260701>;
  ...
}
```

钩子规则：同步、纯、确定性、**必选**（不实现编译不过——语料里有
`expect_cargo_check_failure` / tsc 负例钉住）。返回值是 Decision 结构：
退休成员从 reader 来、新增成员由应用声明、被吞掉的旧成员用 `IGNORED` 确认。

### 5.2 StoredValues——按 Head 实例解析身份的 reader

```ts
export interface StoredValues {
  // 「wire status，最后由 2026-04 服务」，从 downcast 退役那一步的 source 实例解析身份
  profileStatus(source: types.v2026_07_01.Profile): string | undefined;
}
```

应用实现 reader，从 Head 实例找键（id、zip……）。容器元素没有 id 时用元素自身
字段（见 business 语料 `addressGate(source) => gates.get(source.zip)`）。

### 5.3 NEEDS——退休面报告

```ts
export const NEEDS = {
  Parcel: {
    "2026-01": ["handling_note", "reference"],
    "2026-04": ["handling_note"],
    "2026-07": [],
  },
} as const;
```

哪个类型、哪个版本仍然从存储读哪些 wire 名；Head 读空数组。语料逐条断言。

### 5.4 OMIT / IGNORED

- 下行：optional 退休成员缺行 → decision 里给 `a.OMIT`（显式省略，不是丢弃）
- 上行：被新版本吞掉的成员 → decision 里带 `a.IGNORED` 确认（不转发）

### 5.5 定位的拒绝（locate 深路径）

元素转换失败时，surfaced `typePath` 是全路径（PR #380 引入，后修正 optional 不
重复前缀）：

```rust
// 生成物内的辅助（Rust 侧；TS 侧是 locate/mapSequence/mapRecord 同构）
fn locate<T>(result: VersionedResult<T>, path: &str) -> VersionedResult<T> {
    result.map_err(|error| UnsupportedProjection {
        type_path: format!("{path}.{}", error.type_path),
        ..error
    })
}
fn map_sequence<S, T>(source: &[S], convert: impl Fn(&S) -> VersionedResult<T>, path: &str)
    -> VersionedResult<Vec<T>>
{ source.iter().map(|item| locate(convert(item), path)).collect() }
```

语料断言 `Parcel.stops.Address.gate` / `Parcel.by_tag.Address.gate`。直接成员
不做前缀（hook 自己写的路径就是权威），optional 成员也不做（否则
`Route.protocol.Route.protocol` 双重前缀——这是修过的坑）。

### 5.6 dispatch 入口——应用先解析版本

```rust
// 生成物的契约注释：
// "A version resolved by the application before calling an adapter."
// "Context for application business policy; adapters do not select versions."
pub async fn dispatch_directory_publish<F, Fut>(
    version: Version, wire: Value, handler: F, adapters: &dyn VersionAdapters,
) -> Result<ProjectionOutcome<...>, AdaptationError>
```

**版本头提取/优先级/fallback 完全归应用层**（维护者在 PR #379 的反馈，已回
复确认）：Stripe（请求头 > SDK 版本 > 账户默认）与 Polar（请求头 > Current）
各不相同，由产品决定；xidl 生成物不含任何版本选择逻辑。语料里的
`resolve_version` 是「应用拥有策略」的示例：

```ts
// bdd/boilerplate/versioned_transforms/ts/transforms_app.ts（应用层示例策略）
resolve_version(header): 缺头→Head；日历日期→不晚于它的最近版本（调用方钉住
行为，平台升级不漂移）；更早/非法→400 并回写 unresolved
```

待办：按反馈补一条负向断言（生成物不含头提取），并把描述改写为「应用层示例
策略」。客户端侧每版本模块各 pin 一个具体版本号常量，不存在可移动的
"latest"——与「项目默认版本写成具体日期」一致。

### 5.7 @replaces 与 @default

- `@replaces(owner) organizer`：机械改名配对（#381）。上行忽略旧名、下行复制；
  absent optional 保持 absent（不编造 owner）。
- `@added(v) @optional @default(false) locked`（#382）：默认值是**给新调用方
  的**，不是给老调用方的。老调用方的隐含语义由 upcast 钩子显式声明
  （`locked: false` 写进 Decision），绝不让 default 悄悄生效；老调用方省略该
  成员时 Head 上保持 absent（写规则二）。

## 6. 模块导航（接手先读这些文件）

| 路径                                                                   | 职责                                                                                                           |
| ---------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `xidl-parser/src/versioning/`                                          | 逐版本投影（project/）、注解校验、`@replaces`（replaces.rs）、生命周期                                         |
| `xidl-parser/src/rest_hir/`                                            | HTTP 语义模型；`HttpInputBinding.source_param` 是「文本传输位」判断的数据源                                    |
| `xidlc/src/generate/adapter/`                                          | 共享计划器：`model.rs`（TypeShape/ValuePlan/TypePlan）、`collect/`、`planner/`、`updates.rs`、`wire_layout.rs` |
| `xidlc/src/generate/typescript_rest/adapters/`                         | TS 适配器上下文与渲染（context/names/render/*、wire/）                                                         |
| `xidlc/src/generate/rust_axum/adapters/`                               | Rust 适配器（context、render/{conversions,endpoints,operations,stored,unions,update,wire}）                    |
| `xidlc/src/generate/typescript_rest/interface/method.rs`               | 操作信封；`helpers::text_positions(op)` 决定哪些参数位 coerce                                                  |
| `xidlc/src/generate/typescript/definition/type_expr.rs`                | zod schema 构建；`ZodCoercion::{Strict,Coerce}` 双模式（#391）                                                 |
| `xidlc/tests/version_adapters_targets.rs` / `rust_adapters_targets.rs` | 适配器 insta 快照测试                                                                                          |
| `xidlc/tests/adapter_baseline/`                                        | 普通目标的字节级 baseline（README 有重生成规程）                                                               |
| `bdd/features/steps/version_adapters.py`                               | 版本化 BDD 步骤；`prepare_rust_application` 的 companions 映射                                                 |
| `bdd/boilerplate/versioned_*/`                                         | 各语料的应用实现（TS+Rust），是「生成器 × 真实业务」的合同                                                     |

### 代码范例：钩子 + reader + 往返（Rust，节选自 business/chain 语料）

```rust
// hooks.rs：退休成员从存储读，身份从 Head 实例解析
fn parcel_downcast_to_v_2(&self, source: &v3::Parcel, stored: &dyn StoredValues)
    -> VersionedResult<ParcelDowncastToV2Decision>
{
    Ok(ParcelDowncastToV2Decision {
        handling_note: stored.parcel_handling_note(source), // None → OMIT
    })
}

// chain.rs：往返规则一，老 wire 形状在 Head 一来一回后不变
let round_trip = downcast_booking(Version::V20260601, to_head, storage, hooks).unwrap();
assert_eq!(value, v1, "the oldest wire shape survives the round trip");

// 首个缺事实的步骤即拒绝：信息只丢弃不编造
let missing = downcast_booking(Version::V20261001, orphan, storage, hooks).unwrap_err();
assert_eq!(missing.type_path, "Booking.roster");
```

### 代码范例：E11 事件拆分（TS，`versioned_events/ts/events.ts`）

```ts
const hooks: a.VersionAdapters = {
  // 两个半边都服务旧名：delivered/read 各自是一次 updated 推送
  eventKindDowncastToV20260401: () => ok("updated"),
  // 旧名只保一个半边：应用声明哪一半拥有这个名字（有损，写进变更说明）
  eventKindUpcastFromV20260401: () => ok("delivered"),
};
// 断言：老订阅者对同一条消息收到两次 updated，事件 id 各自保留；
// 老写入永远不产生 read（不编造）；delivered 出现在 v2 wire 上直接被
// 校验拒绝（hook 之前）。
```

## 7. 已完成（截至 2026-10-25）

**分支栈（自底向上，全部 CI 绿，依序合并）**：

1. **#379** `feat/version-stored-values`——存储 reader 决定被移除成员；
   StoredValues/NEEDS/OMIT/IGNORED 语义；booking 链语料。
2. **#380** `feat/version-update-adapters`——更新流（PATCH）从配对退休推导；
   容器形状 C1–C6：union 降级运行时、容器元素丢弃、元素内退休成员（gate）、
   深路径 locate（生成器改进 + optional 修正）、继承语料（现状受支持面）+ issue
   #389；v1 PATCH、幂等、空补丁。
3. **#381** `feat/version-rename-from`——`@replaces` 机械配对（维护者裁定： 不做
   rename_from，做 replaces）；absent 保持 absent。
4. **#382** `feat/version-default-upcast`——`@default` 的 implied 值语义； E1–E12
   全目录 transforms 语料（双语言 + HTTP 旅程）；E11 事件拆分语料。

**独立修复**：

- **#385** `fix/union-enum-variant-paths`（修 #384）：union 跨用户枚举的变体
  路径限定。CI 绿，待维护者合并。
- **#391** `fix/ts-strict-json-decode`（修 #388）：JSON 位严格解码、文本位
  （path/query/header/cookie、表单体）coerce、布尔只认 true/false；双语言 hurl
  回归（`wire_strict` 语料）。提交时 CI 运行中。

**其它**：#383 `feat/deprecated-sites`（@deprecated 扩面）开着；issue #368
（removed-member 分类，评论里维护者已给方向：merge 语义不进 xidl，靠应用
before/after）。

## 8. 未完成 / 后续方向（按优先级建议）

1. **#391、#385 合并后重爆版本化栈**（#379–#382 的快照会因严格解码变化需要
   重录；流程见 §9.3）。
2. **#390**（新登记）：缺失必填标量的跨语言分歧——TS 拒绝（400），Rust 用类型
   默认值编造（200 + `count: 0`）。修法二选一：Rust 提取器拒绝缺失必填标量，
   或两侧都声明 optional；补 per-transport hurl 回归。这违反「不编造」公理，
   优先级高。
3. **#389**：继承结构的成员级 decision。现状：派生类型退化为全类型 hook（无
   Decision/StoredValues/IGNORED/NEEDS），请求位直接拒绝生成。设计要点：
   `StructDcl.inherited` 已由语义分析展平，surface collector
   (`adapter/collect/mod.rs:146`) 目前显式 Unsupported；需要给 IR 的 Member 加
   **访问路径**（Rust 模型是组合 `base.tag`，TS 是 extends 扁平），渲染层按
   语言取路径。issue 里有最小复现 IDL。
4. **版本头策略边界的负向钉住**：断言生成适配器产物不含头提取/版本选择；
   语料描述改写为「应用层示例策略」（见 §5.6，PR #379 已回复）。
5. **E13 退休接口**：接口级 `@removed` 保留到退休日，用最新操作重实现；需要
   「退休接口仍在 wire 上服务老版本」的运行时腿 + NEEDS 的接口级条目。
6. **NEEDS 报告补全**：全类型 hook（继承）服务的退休成员不出现在表里—— 随 #389
   一起修。
7. **webhook 平台特性 #323**：outbound 接口、OpenAPI webhooks、Standard Webhooks
   助手。E11 语料（versioned_events）已经把「事件拆分的翻译语义」
   钉住，是它的前置。
8. **#320 跨目标 wire 一致性**、#366/#367 相关适配器特性——见 issue。

## 9. 开发流程与门禁（照做即可，坑都标了）

### 9.1 分支与提交

- 功能分支自 `master`；版本化栈内部叠放（#379→#380→#381→#382），重爆用
  `git rebase --onto <新基> <旧基>` 只重放本 PR 自己的提交。
- 每个提交走 conventional commits（`fix(versioning): …` /
  `test(versioning): …`）； **测试必须位于功能/修复 PR
  内**（维护者明确要求，参考 #386 被打回）。
- 手写代码遵守 `AGENTS.md`（行数门 400、pub 项 Rustdoc ≤20 行、禁
  dead_code/`#[allow(dead_code)]`、函数 ≤4 参、错误点名契约……）。

### 9.2 门禁命令

```sh
just test                  # 全量（rust/go/ts/npm/bdd）
cargo test --all -F transport-all   # CI 的 rust 单测口径
cargo clippy --all-targets --all-features -- -D warnings
pre-commit run -a          # 含 biome 2.4.10、rustfmt、ruff、行数门……
just test-coverage         # tarpaulin ≥ 95%
```

BDD 分片与 tag 的映射在 `.github/workflows/check-bddtest.yml`（如
`@bdd_rest-versioning`）；本地：`behave bdd/features --tags=@bdd_rest-versioning`。

### 9.3 改生成器后的重录规程（最容易翻车，逐步照抄）

版本戳是**编译期**烙进 `xidlc` 的（`XIDLC_VERSION`/`XIDLC_GIT_HASH`）： 任何
`cargo build/test` 不带环境变量都会用真实版本号覆盖 `target/debug/xidlc`，导致
baseline 对不上。规程：

```sh
# 1) 全程带钉死环境（建议 export 在同一 shell 里）
export XIDLC_VERSION=0.0.1 XIDLC_GIT_HASH=snapshot
cargo build -p xidlc --features cli      # 烙好戳的 CLI

# 2) 重录 insta 快照
INSTA_UPDATE=always cargo test -p xidlc
cargo test -p xidlc                      # 复验

# 3) 重录 baseline（见 xidlc/tests/adapter_baseline/README.md）
for f in versioned_adapters versioned_adapters_business versioned_rest \
         wire_contract http_body_shape; do
  ./target/debug/xidlc gen --out-dir <空目录> typescript-rest --client --server bdd/features/data/$f.idl
  ./target/debug/xidlc gen --out-dir <空目录> rust-axum      --client --server bdd/features/data/$f.idl
  # 序列化为 {文件名: 内容} 的 2 空格缩进排序 JSON + 尾换行
done

# 4) cargo test --all -F transport-all 复验（其中 ordinary_targets_match…
#    逐字节比对 baseline）
```

### 9.4 常见坑清单

- **版本戳**：见 §9.3；症状是 baseline diff 里整文件只有第一行
  `// Code generated by xidlc-v…` 不同。
- **pre-commit 本地缓存过期**：本地旧缓存的 biome 规则与 CI 新装 2.4.10 不一致，
  本地过、CI 挂。症状：CI 的 biome "files were modified by this hook"。修：
  `pre-commit clean` 后重跑（会重装环境）。
- **stacked 分支重爆**：冲突几乎都在快照/baseline；解决方式不是手解冲突标记，
  而是按 §9.3 在该提交的树上重录。`git rebase --onto` 只重放本 PR 提交；
  误把上游提交也重放会大面积冲突（跳过被吸收的提交即可）。
- **BTreeMap 排序 vs TS Map 插入序**：整表拒绝时 surfaced 的第一个元素不同
  （Rust 按键排序，TS 按插入序）——语料断言要按各自的真实顺序写，注释说明。
- **生成模型的双语言形状差**：TS `extends`（扁平访问）、Rust 组合
  （`#[serde(flatten)] base: Base`）——跨语言断言别假设同一访问路径。
- **`typed_ast` 不许合成节点**；生成器手拼字符串 <20 字符；模板只渲染。
- **多进程 cargo**：`cargo test` 会重编 `xidlc` bin；并行的 INSTA 录制/验证
  之间偶尔争用 manifest 测试产物——重跑即过（已见过两次，均为环境噪音）。

## 10. 参考项目（为什么这样设计）

- **Stripe API versioning**：日期版本 + 变更日志 + 服务端按 `Stripe-Version`
  头逐版本翻译；「请求头 > SDK 版本 > 账户默认」的优先级链是应用层策略的范本。
  我们的链式相邻转换 + NEEDS 退休表直接对标它的「平台内部只有最新形态」。
- **Polar（@polar-sh/sdk 按 `/2026-10` 子路径导入）**：SDK 侧钉具体版本、自动
  带版本头；「项目默认版本在创建时写成具体日期，不存 latest」——我们生成的 client
  每版本模块 pin 常量版本号即同一原则。其 Current/pinned 双轨对应我们 的
  Head/退休窗口。
- **Cadwyn**（Python/FastAPI 版本化框架）：用「版本路由 + 中间件改写请求/响应」
  的运行时翻译；对照结论：我们把翻译下沉到**编译期生成的纯函数**（可快照、可
  审计、跨语言一致），运行时只留应用钩子。（对比结论：Cadwyn
  在运行时逐请求改写；xidl 在编译期生成纯函数链。）
- **serde / serde_urlencoded（Rust）**：严格解码的语义基准。#391 之后 TS 与 Rust
  的行为对齐：JSON 位严格、文本位（含表单）按字段类型解析、布尔只认
  true/false（`bool::from_str` 同义）。
- **Standard Webhooks**：#323 平台特性的协议参照；E11 语料钉住的事件拆分语义
  （旧订阅者收两次 updated、id 各自保留）需要在变更说明中向用户声明。

## 11. 进一步思考后的改进设计提案（接手者可据此开工）

### 11.1 版本策略外置的正式化（对应 §5.6、§8.4）

现状：生成物不选版本（好），但「应用如何解析版本」没有共享的形状，每个语料 手写
`resolve_version`。提案：生成一个**纯数据**的 `VersionPlan`（版本值、排
序、头名/查询名/cookie 名），并提供语言侧极薄的 `resolve(raw: string|null)`
原语集合（`exact | nearestNotAfter | reject`），**不提供任何组合策略**——策略
（Stripe 链、Polar 链、账户默认）由应用组合这些原语实现。收益：负向钉住天然
成立（生成物里搜不到 `if header missing then`），语料改用原语组合展示示例策
略。风险：低；纯增量。

### 11.2 继承成员级 decision（对应 #389）

设计：给 `adapter::model::Member` 增加 `path: Vec<String>`（空 = 自有字段）；
`collect/constr` 对 `parent` 非空的 StructDcl 用 `inherited ++ member` 展平并
给继承成员标 `["base"]`（多级继承由语义分析已展平，路径只有一级——但 Rust
模型嵌套是 `base.base.x`，需要按链深填）。渲染：Rust 取
`source.{path.join(".")}.{name}`，TS 取 `source["{wire}"]`。plan/decision/
StoredValues reader 命名沿用派生类型名（`item_tag` reader 读 `&v3::Item`）。
NEEDS 同步补条目。验收：flat 结构与继承结构在同一 IDL 上的生成物**逐行同构**
（快照测试直接对比两份输出除类型名外一致）。

### 11.3 缺失必填标量的统一（对应 #390）

建议方向：Rust 提取器对非 `@optional` 标量参数缺失时拒绝（与 TS 的 zod required
对齐），而不是两侧都放成可选——「不编造」公理优先。实现点：rust_axum 请求
结构体的 serde 属性（`#[serde(default)]` 只给 `@optional` 参数）。回归：每
transport（path/query/header/cookie）一条「缺失必填 → 400」hurl 腿，双语言。

### 11.4 E13 退休接口

IDL：`@removed(V::v3) @get(path="/poll") …`。语义：接口退休后**仍对声明它的
版本服务**（v1/v2 wire 上还在），Head 不暴露；NEEDS 增加「接口级」条目（哪些
版本仍读该操作）；应用用最新操作重实现（语料里就是「cursor 分页拼出全量」的
业务分支，绑定变更 id 与退休日——见目录 Q4）。生成器现状：`@removed` 接口
已有校验（version_adapters_targets 里有 removed operations 的负例），缺的是
「退休接口继续服务老版本」的正向生成与运行时腿。

### 11.5 Webhook 事件拆分的订阅展开（对应 #323 + E11）

E11 已钉：下行双半边→旧名；上行旧名→**一个**声明的半边（一进一出）。目录里 E11
的 `upSubscription`（一进多出 flatMap 展开）在枚举逐值机制里不可表达——
这是**有意的**：把「订阅一个旧事件等于订阅两个新事件」留给业务（信息可能被
编造：调用方也许只想要其中一半）。若未来要支持，正确形状是给
`sequence<EventKind>` 成员提供**成员级**展开钩子（decision 返回列表），而不是
放宽枚举钩子。#323 落地时把该取舍写进变更说明模板。

### 11.6 覆盖率与一致性巡检（低成本高价值）

- 把「往返规则一/二」做成语料框架的复用断言（现在每个语料手写）；生成器侧
  可以加一个 dev-only 目标：对每个已发布版本生成样例请求/响应并自动跑两条
  规则（目录 Q3 的「测试自动生成」）。
- `xidlc version diff`（已有雏形，见 version_diff 语料）扩展为「保留字段表」
  的机器可读输出（Q2 的 CI 拦截点：删存储列的迁移引用了表内字段 → 拒绝）。

## 12. 一页速览（接手第一天的 todo）

1. 读 `AGENTS.md` + 本文档 §2/§5/§9。
2. `gh pr list` 看开着的 PR；合并顺序：#385、#391（独立）→
   #379→#380→#381→#382（栈）。
3. #391/#385 合并后按 §9.3 重爆栈并重录。
4. 从 §8 的优先级列表挑下一件（建议 #390 → #389 → 版本策略负向钉住 → E13）。
5. 所有新行为：**先写业务语料（双语言），再动生成器**；生成器改动必带快照 与
   baseline 重录；测试进同一个 PR。

---

_文档由 2026-10-25 会话整理；对应工作：PR #379–#382、#385、#391；issue
#384、#388（已修）、#389、#390、#366–#368、#323、#320。_
