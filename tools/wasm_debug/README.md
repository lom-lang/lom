# tools/wasm_debug — WASM 线性内存诊断工具（W 工作包沉淀，v1.1.1）

定位 8.4 挂账越界（RFC-0003 修订 26-27）时建的工具链。零第三方依赖（node + Python 标准库）。
复用于将来任何 wasm 载体的内存/布局类 bug（L2 自举编译器也适用）。

## 组件

| 文件 | 用途 |
|---|---|
| `gen_scale.py` | 生成递增规模的合法 Lom 程序（fn 重复声明 / expr 长加法链 / nest 深嵌套），规模阈值标定用。**独立可用**：`python tools/wasm_debug/gen_scale.py fn 4384 > t.lom` |
| `dbg_run.mjs` | `run_wasm.mjs` 的诊断变体：`LOM_TRACE_ALLOC=1` 采集全量分配日志、`LOM_ALLOC_OUT` / `LOM_SNAP_OUT` 落盘分配日志与内存快照、`LOM_SNAP_AT=N` 在第 N 次分配时中途快照（两侧同刻对比的关键）、`LOM_PRE_GROW` 原样透传。**正常模块可直接跑**；分配日志功能需按下文给 codegen 打探针补丁 |
| `heapdiff.py` | 堆语义图 diff：按分配日志逐对象解码（string/record/map/cons/enum 按尺寸结构），指针归一化为 owner 分配序号后按分配序对齐对比——直接字节 diff 在指针平移下全是噪音，语义级才能只留真实差异。`python tools/wasm_debug/heapdiff.py <trap快照> <trap分配表> <pass快照> <pass分配表>` |

## 分配日志的探针补丁（需要时临时打，勿长留）

`dbg_run.mjs` 的 handler 已在，但 codegen 侧探针随 v1.1.1 修复回滚了。需要分配日志时
给 `src/wasm_codegen.rs` 打 4 处临时补丁再 `cargo build --release`（定位完回滚）：

1. 导入表加 `("lom_dbg_alloc", ty_ii32_unit)`（新 type `(i32,i32)->()`），`N_IMPORTS` +1
   （`IMP_DBG_ALLOC` = 原导入总数，RT_* 相对 N_IMPORTS 定义会自动重排）；
2. `build_alloc()` 尾部 `hp = new_hp` 之后插 `a.lget(0).lget(2).call(IMP_DBG_ALLOC);`
   （上报 size 与分配后 hp）；
3. （可选，抓野 scrutinee）导入 `("lom_dbg_wild", (i64)->())`，在 `Pattern::Variant`
   载荷提取前插 `a.lget(s).call(IMP_DBG_WILD);`——dbg_run.mjs 的 handler 会在
   tag≠6 或 ptr 越界时抛出并打印野值第一现场；
4. 导出 `("lom_hp", ExportKind::Global, 0)` 供 harness 读堆指针。

## 方法论（一次一针；RFC-0003 修订 26 的实战顺序）

1. **确定性检查**：同源编译多次比 md5——先排除编译侧漂移；
2. **布局敏感性检查**：变 argv[0] 长度（复制 wasm 到不同长度路径）重跑——若结果随长度
   翻转，即堆布局敏感（这是 8.4 "非确定性"的真相）；
3. **分配序列对比**：trap/pass 双侧 `LOM_TRACE_ALLOC=1` 跑出分配表，先比尺寸序列
   （排除错误尺寸分配），hp 全程不降（排除 bump 走飞）；
4. **同刻快照**：`LOM_SNAP_AT=N` 在第 N 次分配时导出内存，两侧同 N 对比；
   N 取 trap 前最后几次分配做二分，可把差异窗口缩到个位数分配；
5. **语义图 diff**（heapdiff.py）——若到 trap 前全堆干净，说明野值是运行时算出的，
   不是存储腐蚀（8.4 案即此），转第 6 步；
6. **野值现场探针**（lom_dbg_wild）在可疑解引用点抓值，从值的 tag/ptr 直接反推来源。

## 历史档案

- 8.4 越界（2026-09-02 挂账 → 2026-09-07 根治）：RFC-0003 修订 25-29；
- `LOM_HP_TRACE` / trap 时内存页数：eval/runner/run_wasm.mjs 内置（无需本目录）。
