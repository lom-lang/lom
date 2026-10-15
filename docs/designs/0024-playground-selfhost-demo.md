# Playground 二期 B-only 设计（L2 自举演示模式：浏览器内"由 Lom 编译 Lom"）

- **文档**：docs/designs/0024-playground-selfhost-demo.md
- **性质**：动工前置设计（designs/0019 丙路线既定的二期增量；先例 0001-0023）
- **设计基线**：HEAD `9f58430` / v1.11.0（CI run 38671865630 推送后首跑进行中；台账零 open，顺延号 R148 起）
- **裁决**：2026-10-15 用户裁决"执行可立即动工项"——本批即交接菜单①⑤，五个裁决点均按建议项：**范围 B-only**（只做自举演示，不做访客任意代码走 L2 编译的 A 面）/ **quine 基准 = 预置 hex** / **L2 独立看门狗 60s** / **口径标注**（L2 子集承载自举叙事，主引擎仍为全语言面）/ **CI 冒烟扩展**。前置调研结论（交接状态块登记）：栈关切解除——self_comp 编译型 wasm 默认栈全过、256KB 下界余量 ≥3.8×；单趟编译默认栈约 3s。
- **状态**：**已交付（2026-10-15，仓库版本 v1.12.0；tag `61ff36e`，CI run `38688935472` 七 job 绿——含扩展后 playground job）——执行者实施 + 规划者验收（selfboot.js env 契约/app.js 看门狗/ci.yml 追加段亲读；smoke 全链 11/11+6/6、doc_audit 73/73、verify_selfcomp 368/368、build_selfboot 三件亲跑；浏览器（Chromium）手测四步链 QUINE 272925 byte-identical ~26.5s、四行 done、按钮复位）。实施偏差如实登记四项：① .gitignore 追加 playground/dist-selfboot/（设计称 gitignored 但 hex/lom 副本不在既有 *.wasm 覆盖内——必要面）；② smoke.mjs import 行扩展（readFileSync,+existsSync，一期输出形态逐字不变）；③ 本机链总耗时 ~24-30s（node 默认栈），高于设计预估 ~10s——60s 看门狗余量充足，慢机器如实登记；④ worker 分发物查找次序 ./name → ./dist-selfboot/name（本地手测回退，生产平铺一次命中）。gh-pages 部署为独立呈批件（Chrome/Edge 线上复测随部署）。**
- **语言面**：v1.0 冻结维持——本批**零 src/ 改动、零 .lom 改动、零语言面变化**（纯 playground/ 前端 + 构建脚本 + 文档批）。
- **目标**：playground 页面加"self-hosting demo"模式——访客按一个按钮，在浏览器里亲眼看到 272925 字节强 quine 的产生与逐字节验证：Lom 的 L2 子集编译器（自身已被编译为 wasm）现场编译 self_comp.lom，产物再解码为 wasm，再让它编译自己一次，输出与预置基准逐字节一致。自举叙事的交互化（0019 §1.5 既定素材位）。

## 1. 运行链（页面按钮触发，四步逐步展示）

预置分发物（部署时构建，见 §3）：`selfcomp.wasm`（层 2 编译器产物，≈272925 字节）、`self_comp.lom`（源文本，仓库既有 examples/selfhost/ 副本）、`selfboot-baseline.hex`（预置基准 = 分发 selfcomp.wasm 对应的 hex 文本）。

1. **STEP 1 自举编译**：selfcomp.wasm（WebAssembly 实例）编译 self_comp.lom → 产出 hex 文本 A（经 env.lom_file_write 落虚拟文件）。预期默认栈 ~3s。
2. **STEP 2 解码**：浏览器 JS 版 hex2wasm（解码桩 ~20 行，与 tools/hex2wasm.py 同口径：剥空白后两字符一字节）将 hex A → wasm bytes B。
3. **STEP 3 self-apply**：以 wasm bytes B 新建实例，再次编译 self_comp.lom → 产出 hex 文本 Q。
4. **STEP 4 quine 验证**：hex Q 与预置基准 hex 逐字符比对 → 一致则显示"QUINE 达成：272925 bytes, byte-identical"（字节数 = hex 字符数/2，与 verify_selfcomp --bootstrap L3 同口径）；不一致如实显示 MISMATCH（不粉饰）。

每步显示耗时；四步总耗时预期 ~10s 内（远低于看门狗）。

## 2. 文件与结构（playground/ 下新增，一期六文件零改动或最小追加）

| 文件 | 职责 |
|---|---|
| `selfboot.js`（新增，~300-400 行） | 自举演示 harness：env 导入契约的浏览器化（print_i64/print_f64/print_str/ftoa/json_parse/json_stringify/lom_alloc/file_read/file_write/file_exists/env_args——逐项镜像 tools/selfcomp/run_selfcomp.mjs 的 node 实现语义）+ JS 版 hex2wasm + 四步链驱动与进度回调 |
| `selfboot-worker.js`（新增，~60 行） | 独立 Web Worker：fetch 三分发物各一次、驱动 selfboot.js 链；真 trap 转译 { trap } 载荷（与一期 worker.js 同协议风格） |
| `index.html`（追加一节） | 页面加模式区块："Self-hosting demo"——按钮 + 四步进度 + 输出 pane + **口径标注**（自举引擎 = L2 子集（supported face 的 strict subset）；全语言面程序请用上方主引擎；quine 数字与本仓库 [README](https://github.com/lom-lang/lom) 一致）|
| `app.js`（最小追加） | selfboot-worker 消息接线 + **L2 独立看门狗 60s**（区别于一期主引擎 5s 看门狗：自举链合法耗时更长；超时 terminate worker + 提示）|
| `smoke.mjs`（扩展） | node 冒烟加自举链：构建产物后在 node 跑四步链（selfboot.js 在 node 可复用——env 导入以回调注入，node 侧提供同语义胶水）+ 基准比对 + quine 字节数断言 272925 |
| `build_selfboot.py`（新增，tools/ 邻位脚本） | 部署/冒烟前的分发物构建：宿主 lom 跑 self_comp 编译自身 → selfcomp.hex（=基准）→ hex2wasm → selfcomp.wasm；输出到指定目录；**不进 CI 单独 job**（第七 job 内作 step 调用）|

虚拟文件模型：selfboot 链内 file_read/file_write 走 JS Map（路径→字节缓冲）——self_comp.lom 以虚拟路径预载、输出 hex 写虚拟路径后回读；不依赖一期 harness 的 /playground.lom backing 单文件模型（自举链需要多文件）。**已知边界（交接登记面）**：宿主 wasm 内建 file_read/file_exists 全形态不可用（一期 harness path_open 链根因未定位）——本链不经宿主内建（L2 编译器的 env.lom_file_* 是独立通道，一期冒烟已证可用），该登记面维持独立修复批候选不动。

## 3. 分发物与部署

- 分发三件 + 现有一期五源件 + lom.wasm 同推 gh-pages 孤儿分支（部署流程零变化，仅多文件）。
- 总分发增量 ≈ 272,925（wasm）+ 545,850（hex 文本）+ 544,901（self_comp.lom 副本）≈ 1.36MB。
- CI 第七 job 追加 step：`python tools/build_selfboot.py`（构建产物，~2 分钟：宿主自编译一趟）→ `node playground/smoke.mjs`（既有 + 自举扩展段）。

## 4. 验收预设

1. 本地：`node playground/smoke.mjs` 全链（一期段不回归 + 自举四步 + 基准一致 + 272925 断言）逐字通过；
2. 浏览器手测（Chrome/Edge）：demo 按钮四步渐进显示 → QUINE 达成文案 → 口径标注在位 → 60s 看门狗不误触发（正常链 ~10s）；
3. 全量回归基线不变（零 src/ 零 .lom：581+40、368/368、quine 272925、eval 129×2 等全部构造性不变）；
4. 部署后线上可达 + 页面功能实测（部署前单独呈批——铁律 6）。

## 5. 边界与登记（随批入 TODO）

- 自举链耗时与时限：60s 看门狗为设计余量（正常 ~10s；慢机器浏览器 wasm 层可能数倍，60s 覆盖）。
- 分发物为部署时构建快照：随仓库 self_comp.lom 演进需重建（构建脚本产物，不手工维护）。
- hex2wasm JS 化为解码桩第二实现（python 为第一实现）：同口径（剥空白、两字符一字节），node 冒烟对拍锁定。
- L2 子集承载自举演示的口径句为一期边界句（0019 §5"L2 自举模式沿用实验性口径标注"）在 L2 转正（v1.8.0）后的更新形态：标注"strict subset / supported face"，不再用 experimental 字样。
