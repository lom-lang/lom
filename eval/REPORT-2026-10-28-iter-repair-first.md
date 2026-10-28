# Lom 迭代修复效率题型双模型首发采样报告 — 2026-10-28

> **English TL;DR** — First sampling run for the iterative-repair category
> (ids 131-139, designs/0027 B3): two models × 9 tasks × 10 trajectories ×
> R=5, single-step feedback chain, temperature=1.0. **All 180 trajectories
> converged within 2 rounds; 0 censored.** Per-task mean C(1) = 83.3%
> (glm-5.3, Coding-Plan endpoint) / 91.1% (deepseek-v4-pro+thinking), and
> **C(2) = C(5) = 100% for both models on every task**. The category's core
> hypothesis is confirmed on its anchor task: **131 (≡ the deliberately hard
> 127) converts from 30% round-1 pass to 100% within round 2** — all 14
> round-1 failures across both models (the same missing Option-unwrapping
> step, RUNTIME000, that made 127 fail 18/20 in the 2026-10-07 matrix) were
> repaired in round 2 after the real runtime diagnostic was fed back.
> Mid-run interruption honestly registered: the deepseek collector was killed
> by the host environment after 54/90 trajectories; a resume driver replayed
> the missing 36 with zero trajectory loss (details in the honesty section).

**测试日期**: 2026-10-28（北京时间下午；UTC 采集窗口 06:51-08:50，见方法节；lom 1.16.0 @ `a8d014b`，工作树干净）
**评测面**: lom-eval 11_iterative_repair 题型首测——**9 任务（131-139）× 2 模型 × 10 独立轨迹 × R=5**，单步反馈链协议（designs/0027 §1.2 裁决 0-2=B；工程面已于 v1.16.0 落地，本报告为该题型首批 LLM 采样数）
**触发**: 用户 2026-10-28 裁决"跑，顺便 deepseek 也可以跑"——设计批 0027 裁决 0-6 原定"glm 单模型首发、双模型留正式测量批"，本批即双模型一次到位。
**管线**: `eval/iter_eval.py`（逐题逐轮：round 1 发与 pass@1 同构的标准题 prompt；round N>1 发原题面 + 上轮完整候选代码 + 上轮实跑真实诊断——静态 `--check --json` 原文 + 运行时 rc/stdout/stderr 原文；模型每轮回写完整代码；通过 = stdout 逐字（CRLF 归一）+ rc 0，与 runner 同口径；通过早停，达 R 未过记删失）+ `eval/iter_summarize.py`（轮数分布 + 删失单列 + C(N)）。轨迹与汇总留档 `eval/iter_runs/<model>_iter10/`（gitignored；`iter_summary.json` 为本报告全部数字之源）。

---

## 总览（双模型对照，per-task 均值口径，与 pass@k 报告同式）

| 模型 | 参数 | C(1)≡pass@1 | C(2) | C(5) | 删失率 | 收敛轨迹 |
|---|---|---|---|---|---|---|
| **glm-5.3**（Coding Plan 端点） | t=1.0，max_tokens 16384，thinking off | **83.3%** | **100%** | **100%** | 0.0% | 90/90 |
| **deepseek-v4-pro**（API key 端点） | t=1.0，max_tokens 16384，thinking enabled | **91.1%** | **100%** | **100%** | 0.0% | 90/90 |

- 轮数池化直方图（180 条轨迹实取）：round 1 收敛 **157**、round 2 收敛 **23**、round ≥3 收敛 **0**、删失 **0**——本题型面内**全部收敛轨迹都在 2 轮内**，R=5 的预算一次都未被触及。
- 模型调用总数 203 次（每轮一调；glm 105 = 75 单轮 + 15 双轮×2，ds 98 = 82 单轮 + 8 双轮×2——由轨迹直方图分模型合计）。

## 逐题结果

### glm-5.3（`eval/iter_runs/glm-5.3_iter10/iter_summary.json`）

| 题 | 轨迹 | 收敛 | 删失 | 中位 | 均值 | P90 | C(1) | C(2) | C(5) |
|---|---|---|---|---|---|---|---|---|---|
| 131（←127） | 10 | 10 | 0 | 2 | 1.7 | 2 | 30.0% | 100.0% | 100.0% |
| 132（←120） | 10 | 10 | 0 | 1 | 1.2 | 2 | 80.0% | 100.0% | 100.0% |
| 133（←122） | 10 | 10 | 0 | 1 | 1.2 | 2 | 80.0% | 100.0% | 100.0% |
| 134（←128） | 10 | 10 | 0 | 1 | 1.0 | 1 | 100.0% | 100.0% | 100.0% |
| 135（←129） | 10 | 10 | 0 | 1 | 1.0 | 1 | 100.0% | 100.0% | 100.0% |
| 136（←130） | 10 | 10 | 0 | 1 | 1.0 | 1 | 100.0% | 100.0% | 100.0% |
| 137（新题·诊断驱动） | 10 | 10 | 0 | 1 | 1.2 | 2 | 80.0% | 100.0% | 100.0% |
| 138（新题·诊断驱动） | 10 | 10 | 0 | 1 | 1.1 | 1 | 90.0% | 100.0% | 100.0% |
| 139（新题·诊断驱动） | 10 | 10 | 0 | 1 | 1.1 | 1 | 90.0% | 100.0% | 100.0% |

### deepseek-v4-pro + thinking（`eval/iter_runs/deepseek-v4-pro_iter10/iter_summary.json`）

| 题 | 轨迹 | 收敛 | 删失 | 中位 | 均值 | P90 | C(1) | C(2) | C(5) |
|---|---|---|---|---|---|---|---|---|---|
| 131（←127） | 10 | 10 | 0 | 2 | 1.7 | 2 | 30.0% | 100.0% | 100.0% |
| 132（←120） | 10 | 10 | 0 | 1 | 1.0 | 1 | 100.0% | 100.0% | 100.0% |
| 133（←122） | 10 | 10 | 0 | 1 | 1.0 | 1 | 100.0% | 100.0% | 100.0% |
| 134（←128） | 10 | 10 | 0 | 1 | 1.0 | 1 | 100.0% | 100.0% | 100.0% |
| 135（←129） | 10 | 10 | 0 | 1 | 1.0 | 1 | 100.0% | 100.0% | 100.0% |
| 136（←130） | 10 | 10 | 0 | 1 | 1.0 | 1 | 100.0% | 100.0% | 100.0% |
| 137（新题·诊断驱动） | 10 | 10 | 0 | 1 | 1.0 | 1 | 100.0% | 100.0% | 100.0% |
| 138（新题·诊断驱动） | 10 | 10 | 0 | 1 | 1.0 | 1 | 100.0% | 100.0% | 100.0% |
| 139（新题·诊断驱动） | 10 | 10 | 0 | 1 | 1.1 | 1 | 90.0% | 100.0% | 100.0% |

（中位/均值/P90 仅收敛轨迹参与，删失单列——本批删失全零，故分布即全量。）

## 核心问题专节：131（≡127）的 round-2 补齐率

**直接回答：round-1 失败轨迹在 round-2 真实诊断喂入后的补齐率 = 14/14 = 100%（glm 7/7，ds 7/7）；10 采样中两模型均为 3 条 round-1 收敛 + 7 条 round-2 收敛，C(2) = C(5) = 100%，删失 0。**

轨迹级取证（`eval/iter_runs/<model>_iter10/s<k>/131/rounds.jsonl` + 各轮 `.lom` 实读）：

- **round-1 失败形态与历史 127 完全同族**：glm 7 条全部 `TYPE001 + RUNTIME000`（`map_get` 返回 Option 未解包直接算术——passk-128 报告中 18/20 候选的同一缺口）；ds 7 条中 5 条 `TYPE001 + RUNTIME000/001`（同缺口族）、2 条 `TYPE003 + RUNTIME001`（解包形态已在但另有类型错）。
- **round-2 全部补齐且是真修复**：14 条失败轨迹的 round-2 收敛代码全部含 Option 解包步（`match map_get ... Some(v) =>` 或同义形态——逐条实读核验 `final_unwrap=True` 14/14），无一靠捷径或 echo。
- **round-1 通过的也不是 echo**：两模型各 3 条 round-1 通过候选全部真含解包步（`r1_unwrap=True` 6/6）。
- **与 passk-128 校准位对照**：127 在 2026-10-07 矩阵的 pass@1 = 双模型各 1/10；本次 131 C(1) = 双模型各 3/10。同管线同题面下的采样波动量级（n=10 二项），方向无系统性漂移；**关键对照在条件收敛**：历史矩阵中 20 候选仅 2 条补齐解包步（无反馈机会），本次协议给了 round-2 反馈后失败面 14/14 全部补齐——"写错后喂入真实运行时诊断 → 一步修复"的修复闭环价值在锚点题上得到直接量化（单轮 30% → 双轮 100%）。

## 双模型对照要点

1. **量级一致，ds 单轮略强**：C(1) 91.1% vs 83.3%；差距全部来自 132/133/137/138 四题的 glm 单采样失败（各 1-2 条，round 2 全部补齐）——无系统性题型差异。
2. **131 上两模型逐轨迹同构**：中位 2 / 均值 1.7 / C(1)=30% / C(2)=100% 完全一致；失败诊断码族一致（RUNTIME000 主导）。
3. **新题 137-139（诊断驱动设计）**：ds 137/138 单轮满分、139 为 9/10；glm 80-90%。设计假设"round-1 信息不足、诊断到达 round 2 才可解"在这两模型上仅弱成立（见诚实节第 3 条）。

## 诚实节（失败样本/中断/边界，如实登记）

1. **ds 采集中断与 resume（无数据损失）**：deepseek 段第一进程于 UTC 06:51 启动、完成 s1-s6 全 54 条轨迹（最后轨迹 `s6/139` mtime 07:50:08）后被宿主环境中止（中止时 `s7/131` round-1 调用正处于空回复重试等待——日志末行"空回复（finish_reason=length），重试..."；非 API 计费错误）。resume 驱动 `target/probes/b3_first/resume_ds.py`（gitignored）镜像 `iter_eval.py` 轨迹内循环补跑缺失 36 条（跳过已有 final 行；call_api 自带 2 次重试 + 轨迹级重试 1 次；实际 0 次轨迹级重试、0 次 API 失败），窗口 07:55:30-08:50:19 UTC，36/36 收敛。`deepseek-v4-pro_iter10/run_meta.json` 为 resume 后补写（原进程未及写出），其 `collection_note` 字段如实登记两段窗口。
2. **中途网络/截断事件**：ds 侧 1 次 `IncompleteRead`（call_api 内建重试成功，无轨迹影响）；上述被中止前的 1 次空回复（thinking 耗尽 16384 tokens，finish_reason=length）在 resume 后同轨迹重调成功。glm 段无异常（进程 exit 0，日志无 stderr 行）。**零样本因 API 失败计败或删失。**
3. **137-139 设计预期 vs 实测（对题型设计的诚实反馈）**：三题设计为"运行时失败信息刻意不入 prompt、单轮信息不足"，预期 C(1) 显著低于 C(2)。实测 C(1) = 80-100%：模型在"静态零诊断 + 题面如实声明运行时会失败"的提示下，单轮即可凭先验写出覆盖错误形态的代码（主观推测：三题的运行时错误族——整数上溢守卫等——在模型先验内）。round-2 诊断的边际价值在这三题上实测仅为 0-20 个百分点。题型对"更强模型 + 可预期错误族"的分辨力有限，后续若要拉开 C(1)/C(2) 差距需更冷僻的运行时错误族（此为设计层观察，非本批结论面）。
4. **C(1) 校准波动**：131 的 30%/30% 对照 passk-128 之 127 的 10%/10%；132/133 的 glm 80% 对照 passk-128 之 120/122 的 100%——均为 t=1.0、n=10 的二项采样波动量级（95% CI 宽），单次差异不构成模型行为漂移的证据。
5. **评分面边界取证（README 已登记边界的本次实证）**：131 与 134 的 broken 种子在宿主评分面（stdout+rc0）可被 echo 通过（题意考点在评分面不可见的 L2 面）。本次实测：131 round-1 通过的 6 条候选全部真含解包步（非 echo，见专节）；134 的 20 条 round-1 候选**全部与参考解一致**（管道形态 → 非管道形态的真修复，逐条 diff 取证 echo 数 = 0/20）。边界在本批未实际触发，但口径上仍如 README 登记保留。
6. **宣称面边界**：本报告只落 `eval/REPORT-2026-10-28-iter-repair-first.md`，不改 README/claims/HANDOVER/TODO——数字入宣称面由规划者验收后另批。

## 方法与参数

- **端点**（与 REPORT-2026-10-07-passk-128 同款）：glm-5.3 → `--provider glm-coding`（`https://open.bigmodel.cn/api/coding/paas/v4/chat/completions`，Coding Plan 订阅端点，key 经 `eval/.api_keys.json` 的 `glm` 字段回落路径读取——文件无 `glm-coding` 专用字段，`GLM_CODING_API_KEY` 环境变量未设；实测首调认证通过）；deepseek-v4-pro → `--provider deepseek`（`https://api.deepseek.com/chat/completions`，`--thinking`）。开测前双端点最小请求探活均 HTTP OK。key 值不出现在任何轨迹/报告（轨迹仅含端点行为产物）。
- **参数**：`--samples 10 --temperature 1.0 --max-rounds 5 --max-tokens 16384`（工具默认，未改）；ds 加 `--thinking`；两模型每题每轮单条 user 消息。
- **采集窗口（UTC）**：glm 06:51:09-07:32:57（41 分钟，单进程）；ds 06:51-07:50 + 07:55-08:50（两段，见诚实节 1）。
- **复现**：`python eval/iter_eval.py --provider glm-coding --model glm-5.3 --samples 10 --temperature 1.0 --max-rounds 5 --out-name glm-5.3_iter10`；ds 同式换 `--provider deepseek --model deepseek-v4-pro --thinking`；汇总 `python eval/iter_summarize.py --dir eval/iter_runs/<run>`。轨迹目录含每轮 prompt 原文/模型回复/静态诊断/运行记录（审计可复算）。
- **已知边界（沿用 pass@k 口径）**：模型为商用端点快照行为，非可固定权重；跨时点对比是"同管线 + 同任务面"的行为稳定性对比。
