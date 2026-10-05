# 在线 playground 设计（host-WASI 全语言面 + 静态托管；L2 自举演示留二期）

- **文档**：docs/designs/0019-playground.md
- **性质**：动工前置设计（解冻后既定路径 ②；先例 0001-0018）
- **设计基线**：HEAD `231dd13` / v1.6.1（CI #265 绿）。语言面 v1.0 冻结维持——**本批零语言面变化**；`src/` 仅允许两处 `cfg(target_family = "wasm")` 门（见裁决点 2）。发布线已解冻；playground 页面上线是对外动作，呈批后执行。
- **目标**：浏览器里跑**真 Lom**（宿主工具链全语言面）——README "playground: planned" 落位，传播杠杆最大的资产；repair-native 的实演窗口。
- **数字落盘前门禁**：写本文前后运行 `python tools/doc_audit.py`。

## 1. 供料结论（规划者亲读：main.rs / interpreter.rs / Cargo.toml / rustup targets）

1. **单 bin 无 lib**：`fn main()` 无条件 spawn 256MB 栈工作线程（main.rs L46-60，Phase 5.0 缓解树遍历深递归）。wasm 单线程目标上 `std::thread::spawn` 不可用——需 `cfg(target_family = "wasm")` 直跑门：wasm 下 `main_inner()` 在主线程执行、R56 的 join-panic 映射整段跳过（wasm 侧 panic 即 trap，由 harness 捕获显示）。**±6 行**。
2. **OS 面收敛**（grep 全部 std::fs/env 触点）：`fs::read_to_string`（主文件 + file_read + 包装载）、`fs::write`（file_write）、`fs::OpenOptions::append`（file_append）、`Path::exists`（file_exists）、argv 注入（env::args）。**无 clock/random/time**（语言设计使然）→ 浏览器侧 WASI（snapshot preview1）实现面极小：`args_get/args_sizes_get`（argv=`["lom","/playground.lom"]`）+ `fd_write`（stdout/stderr 捕获回 JS）+ 最小 `path_open`（仅预开 fd 命中 `/playground.lom` 映射 JS 字节，其余路径回 ENOENT）+ 其余导入回 ENOSYS 桩。全部 Err 臂走既有结构化运行时错误（读码确认 file_read/write/exists 均有干净 Err 分支）。
3. **深度守卫**：`DEFAULT_MAX_CALL_DEPTH = 80_000`（interpreter.rs L381，软件计数、超限报结构化诊断）。浏览器 wasm 物理栈（约数万 Rust 帧）先于 80k 耗尽 → 需 wasm 侧 cfg 降阈值（拟 `8_000`：保"结构化错误先于 trap"且覆盖教学级程序）。**±2 行**。
4. **构建**：`cargo build --target wasm32-wasip1 --release`（rustup 目标在位）；零第三方 crate → wasm 构建同样零依赖、零 unsafe——**`src/` 零 unsafe 口径不受影响**（unsafe 只存在于 Rust 工具链自带的 std/WASI shim，非本仓库代码；与桌面构建上 std 内部含 unsafe 同理）。SECURITY.md 随批补一句 wasm 构建边界注。预估产物 1.5-2.5MB（gzip 后 ~600KB 级，**主观推测**，以实测为准）。
5. **L2 自举演示素材在位**（二期"自举模式"用）：quine 产物 self_comp.wasm（bootstrap 管线产出）+ run_selfcomp.mjs harness（JS 侧已存在，迁浏览器即用）+ hex2wasm（JS 化约 20 行）。页面上加模式切换"由 Lom 自己编译"——自举叙事的交互化，但 L2 是实验性子集，**不适合作 playground 主引擎**。

## 2. 裁决点（动工前置）

1. **执行路线**：甲 host-WASI 全语言面（真 Lom，建议）/ 乙 L2 自举演示先行（叙事强但实验性子集，访客自然代码被拒会误 representation 语言）/ 丙两段制——甲先行上线，L2"自举模式"作二期增量（**建议**）。
2. **src/ 改动边界**：允许上述两处 cfg 门（合计 ±10 行，wasm-only 分支，桌面路径逐字节不变——全量回归 + 存量对拍实证）；**零 src/ 改动**的替代（lib 化重构 + 独立 wasm bin）超最小面不建议。若不允许动 src/，则 playground 路线作废需回本设计重议。
3. **托管**：GitHub Pages 静态站（免后端、免维护、同仓库）；页面 URL 形如 `https://lom-lang.github.io/lom/playground/`。上线动作 = 仓库开启 Pages + 静态产物分支/目录推送——**对外动作，呈批后由您开启或授权我推**。
4. **首屏默认示例**：repair-loop 实演（README 同款坏代码预载；一个按钮跑 `lom fix --apply` 语义、展示补丁 diff、再运行打印 5）——playground 开场即演示语言存在理由（**建议**；fix 是纯源码文本计算，WASI 下天然可用）。
5. **升版**：minor **v1.7.0**（用户可见新能力：playground + cfg 门 patch 的组合按 minor 先例）。
6. **CI 扩面**：新增 wasm32-wasip1 release 构建 step + node 侧 harness 冒烟（hello/fib/修复闭环四步在 harness 里全通）——保 playground 二进制常绿（对齐 verify 纪律）。

## 3. 方案骨架（裁决后细化）

- **产物**：`target/wasm32-wasip1/release/lom.wasm` → 随静态页分发；页面 `playground/index.html` + `harness.js`（手写 WASI 绑定 ~150 行）+ `app.js`（UI ~150 行）+ `lom.wasm`。**页面零第三方 JS/零 CDN**（供应链口径延续；textarea 原生编辑器首版够用，语法高亮留后续）。
- **UI 形态**：单栏编辑器 + Run/Fix/示例下拉 + stdout pane + 错误着色；`lom fix --apply` 按钮展示应用前后 diff；超时看门狗（Web Worker 里跑 wasm + `terminate()` 兜底，拟 5s——防死循环锁页）。
- **边界（页面上如实标注）**：递归深度阈值 8k（低于桌面 80k，超限结构化诊断）；`file` 模块仅主文件可读、写/其它路径 → 结构化运行时错误；`env::args` 恒空表。
- **示例集**：hello / fib / repair-loop（默认）/ 教程精选 3-5 枚。

## 4. 验收预设

1. node 冒烟（CI 化）：同 harness 在 node 跑 hello.lom、fib.lom、修复闭环四步（诊断→fix 计划→apply→运行 5）逐字比对桌面输出；
2. 桌面全量回归（§2.2 全清单 + 存量对拍）——cfg 门 wasm-only 实证零桌面影响；
3. 页面手测清单（Chrome/Edge 双浏览器）：编辑→运行→fix→示例切换→超时触发→file 边界提示；
4. （若裁决 3 通过）Pages 上线后线上可达 + README "planned" 句翻转为实链 + 回填登记。

## 5. 边界登记（随批入 TODO 如裁决通过）

浏览器递归深度 < 桌面（wasm 物理栈限制，阈值 8k 结构化先报）；file 模块浏览器受限面；playground 页零第三方供应链声明；L2 自举模式（若两段制）沿用实验性口径标注。

- **状态**：**设计产出（2026-10-04，动工前置待裁决六点）**。代码零改动。
