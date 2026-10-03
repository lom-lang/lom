# Lom error_repair 31 题扩容基线重跑报告 — 2026-10-03

**测试日期**: 2026-10-03（v1.6.0 @ efaed10，发布解冻检查单第 4 项）
**触发**: error_repair 类目在 v1.5.0（2026-10-01，b61182f）从 24 题扩到 31 题（新增 123-129，含 127-129 三道 L2 面题）。检查单要求"任务集扩容后重跑基线采样"——本报告按 [REPORT-2026-09-16-err-repair-patch.md](REPORT-2026-09-16-err-repair-patch.md) 的同一方法论对 **31 题全集**完成双模型 ×10 采样 t=1.0，闭合该缺口。
**管线**: `eval/llm_eval.py --samples 10 --temperature 1.0 --only 10_error_repair`（采集）+ `eval/runner/run.ps1 -CandidatesDir`（评分事实源不二设：stdout 比对 + 退出码 0）+ `eval/passk_summarize.py`（汇总，内嵌同口径复评）；候选与原始回复留档 `eval/candidates_rerun/deepseek-v4-pro_err31/`、`eval/candidates_rerun/glm-5.3_err31/`（本地，.gitignore 惯例）。

---

## 总览

| 轮 | 集版本 | 模型（端点） | 采样 | 结果 |
|---|---|---|---|---|
| err31 | error_repair 31 题（086-100、109-113、119-129） | deepseek-v4-pro（thinking, t=1.0） | ×10 | **300/310 通过单元；29 题 10/10，126 题 0/10；pass@1 = pass@5 = pass@10 = 96.77%** |
| err31 | 同上 | glm-5.3（Coding Plan 端点, t=1.0） | ×10 | **306/310 通过单元；29 题 10/10，126 题 8/10、127 题 8/10；pass@1 = 98.71%，pass@5 = pass@10 = 100%** |

合计 20 次分类调用（每模型 10 次，断点续跑跳过 0 次——全新采集）、620 个候选提取零缺失（各 310/310，run_meta.json stats 全采样 missing=[]）、**606/620 通过单元**。

采集窗口（run_meta.json）：两模型并行启动于 2026-10-03T10:58:21Z；deepseek 11:15:29Z 完成（17 分 08 秒），glm 11:07:27Z 完成（9 分 06 秒）。

评分双路一致：20 次手工 `run.ps1` 逐采样评分（deepseek 各采样 error_repair 30/31；glm s1/s2/s5/s8 30/31、其余 31/31）与 `passk_summarize.py` 内嵌复评结果完全一致。

## 逐题通过矩阵（error_repair 31 题；P=通过，.=不通过）

31 题中 29 题双模型全 10/10（086-100、109-113、119-125、128、129），下表只列非满分与新题首测行；完整矩阵在 `passk_summary.json`（"matrix" 键，s1..s10 顺序）。

| 任务 | deepseek-v4-pro | glm-5.3 | 形态 |
|---|---|---|---|
| 123 NAM006 别名遮蔽 warning | 10/10 | 10/10 | 新题首测全过 |
| 124 MUT002 闭包捕获 warning | 10/10 | 10/10 | 新题首测全过 |
| 125 TYPE001 注解不符 warning | 10/10 | 10/10 | 新题首测全过 |
| 126 NAM002 重复定义 error | **0/10** | **8/10**（败 s1、s5） | **078 型提示词歧义**（见失败取证 A） |
| 127 L2 面·Map 算术 | 10/10 | **8/10**（败 s2、s8） | glm 两败为真实宿主面失败（取证 B）；ds 十过均走捷径形态（L2 边界见要点 3） |
| 128 L2 面·管道改写 | 10/10 | 10/10 | 新题首测全过，20/20 候选均真改写（0 个保留 `\|>`） |
| 129 L2 面·未导入内建 | 10/10 | 10/10 | 新题首测全过（该题宿主面本身要求真修复——RUNTIME002 拦截） |

## 要点

1. **新题 123-129 首测**：7 道新题中 5 道（123/124/125/128/129）双模型 ×10 全过（100/100 通过单元）；126 出现双模型系统性失败（ds 0/10、glm 8/10），127 出现 glm 偶发失败（8/10）——两处均已完成逐候选取证（下节），不是管线或评分器问题（12 个 126 失败候选全部 rc=0 可运行，127 失败候选 rc=1 有明确运行时错误）。
2. **与 24 题先例的跨时点稳定性**：24 个重叠任务（086-100、109-113、119-122）今日双模型各 10/10——**480/480，与 2026-09-16 先例（同为 480/480）跨 17 天完全一致**。注意重叠块并非全部字节相同：git 比对（`git show 2de1a30:eval/prompts/10_error_repair.md` 对照现版）显示 24 块中 17 块字节相同，7 块（089/090/093/095/100/113/120）在 v1.5.0 被刷新——嵌入诊断 JSON 换成真实 `--check --json` 采集（含 093 撤掉虚构的 MAT001 警告、095/100 把"运行时错误"散文换成真实 NAM003 静态错误），122 的 diff 仅为 footer 位置假象（其后新增了任务）。刷新后的 7 题今日同样双模型 10/10。
3. **L2 面题的评分边界（重要）**：runner 只验宿主行为（stdout + rc0），**L2 链不是评分面**（README §L2-face 生成纪律明示这是任务生成时门槛）。本轮实测暴露了这条边界的具体形态——127 题的 18 个宿主面通过候选中，**没有一个**做出参考解的 `map_has` + `match map_get ... Some(v)` 解包修复：13 个保留 `if False` 死分支（把 `map_get(m,"a") + 1` 或同型算术塞回不执行分支、else 打印常量 2；ds 9 个 + glm 4 个），5 个直接 `println(2)` 常量捷径（ds 1 个 + glm 4 个）。诊断性点检（`python tools/l2fix.py <candidate>`，非评分步骤）：死分支形态的 ds s1 候选被 L2 链**仍然拒绝**（`L2V001 闭包/枚举值参与算术`——L2 是整程序编译期检查，死分支逃不掉），而常量捷径形态（ds s6）L2 编译通过。即：**127 题的宿主面通过不等价于 L2 面达标**——点检实证 ds s1（死分支形态）宿主过、L2 仍拒；ds 名义 10/10 中 9 个死分支形态与其同构（推断，未逐一过 L2 链），仅 ds s6 常量捷径形态点检为 L2 编译通过。128/129 无此问题：128 的 20/20 候选全部真改写（宿主本可跑通 `|>`，改写纯因遵循题意），129 宿主面本身强制真修复。
4. **全量口径句建议**（供 positioning §3 措辞采用，本报告不改动任何其他文档）：可如实写"error_repair 31 任务双模型 × 10 采样 t=1.0（2026-10-03）：deepseek-v4-pro 300/310、glm-5.3 306/310 通过单元；glm pass@5=100%，ds 唯一不满分任务为 126（提示词输出面欠定，修复本身 10/10 全部成立）；24 题先例子集跨 17 天复测仍 480/480"。**不建议**沿用 09-16 的"全过（N/N）"句式——126 的失败是结构性的（见取证 A），跨采样不可收敛。

## 失败取证

### A. 任务 126（NAM002 重复定义修复）——078 型提示词歧义，非修复能力失败

任务面：broken 代码两个同名 `fn area`（双参矩形、单参圆），main 只调 `println(area(4, 5))`；**expected 为 `20\n12\n`**——即参考解额外打印了 `circle_area(2)`（3×2×2=12）。提示词全文只说"修复代码使其正确运行（矩形面积与圆面积都要能算）"，**既未要求打印圆面积，更未给出 r=2**；"能算"与"打印"之间、以及 r 的取值，全部欠定。

取证事实（12 个失败候选逐一实跑）：

- **12 个失败候选（ds 全部 10 个 + glm s1/s5）全部 rc=0**——NAM002 修复本身（重命名重复函数）全部正确完成，代码可运行。
- 失败全部落在 stdout 面，形态三种：只打印矩形面积（`20\n`，ds×6）、追加 `circle_area(3)`（`20\n27\n`，ds×3 + glm×2）、追加 `circle_area(1)`（`20\n3\n`，ds×1）。
- glm 通过的 8 个采样恰是"猜中"了参考解主函数（`println(rect_area(4, 5))` + `println(circle_area(2))`，如 glm s2 候选逐字节同参考解 main）——t=1.0 下对欠定输出面的随机收敛，不是可稳定复现的能力。

代表性失败候选（ds s1，实跑输出 `20\n27\n`、rc=0，expected `20\n12\n`）：

```
from io import {println}

fn rect_area(w: Int, h: Int) -> Int
    w * h
end

fn circle_area(r: Int) -> Int
    3 * r * r
end

fn main() -> Unit
    println(rect_area(4, 5))
    println(circle_area(3))
end
```

判定：**078 型提示词歧义**（与 078/118 家族同构——评分面期望的具体输出未被提示词文本钉死）。修复目标（消除重复定义）在 20/20 采样中全部达成；失败全部是"圆面积要不要打印、用什么参数打印"的猜测失配。处置建议（不在本轮执行）：二选一——提示词补输出契约（"main 依次打印 rect(4,5) 与 circle(2) 的面积"），或 expected 收敛为 `20\n`（保持 main 原样）。在处置前，126 的跨模型失败应按提示词面缺陷归档，不计入修复能力结论。

### B. 任务 127（L2 面·Map 算术）——glm 两败为真实宿主面失败

glm s2/s8 候选同型（走提示词 hint 的字面方向"用 map_get 取出后参与算术"，但未解包 Option）：

```
from io import {println}
from map import { map_empty, map_set, map_get }

fn main() -> Unit
    let m = map_empty()
    map_set(m, "a", 1)
    println(map_get(m, "a") + 1)
end
```

实跑（`./target/release/lom.exe <candidate>`）：rc=1，`[type] warning [TYPE001] '+' 不支持 Option(Named("_Any")) 和 Int` → `[runtime] error [RUNTIME000] 运算 Add 不支持 枚举变体 和 Int`。`map_get` 返回 Option，与 Int 直接相加在宿主面即为运行时错误——**真实修复失败**（方向对、缺 Some(v) 解包一步），与 A 的歧义形态性质不同。参考解形态（map_has + match Some(v)）在全部 20 个候选中零出现。

## 方法细节（与 09-07 / 09-16 严格对齐）

- 采集命令：`python eval/llm_eval.py --provider deepseek --model deepseek-v4-pro --thinking --samples 10 --temperature 1.0 --only 10_error_repair --out-name deepseek-v4-pro_err31`；`--provider glm-coding --model glm-5.3`（同参数，`--out-name glm-5.3_err31`）。
- **实际使用端点**：deepseek → `https://api.deepseek.com/chat/completions`（thinking enabled，t=1.0）；glm → `https://open.bigmodel.cn/api/coding/paas/v4/chat/completions`（**glm-coding 端点首次即认证成功，无需回落 `--provider glm`**；key 读取走脚本内建回落：环境变量未设 → eval/.api_keys.json 的 glm 键，Coding Plan 订阅 key；thinking 未启用，与 09-16 先例同形）。run_meta.json 回显 provider/model/temperature/thinking/max_tokens 与上述一致。
- max_tokens 16384（脚本默认，未改）；prompt = `eval/prompts/10_error_repair.md` 整文件单条 user 消息（20919 字符，31 任务块经 `tools/eval_prompt_check` 口径的锚定提取，本报告不重复该检查）。
- 评分：`run.ps1 -CandidatesDir <s{k}> -LomBin ./target/release/lom.exe`（target/release/lom.exe = lom 1.6.0，构建于 2026-10-01），每模型 10 次 + `passk_summarize.py --ks 1 5 10` 内嵌复评 10 次，两路一致。pass@k 无偏估计 `1 - C(n-c,k)/C(n,k)`；passk_summarize 按 manifest 全集 128 任务建矩阵，非本类任务 0/10 为矩阵形状假象，本报告只统计 error_repair 31 题列（总体 23.4%/23.9% 行即该假象，不引用）。
- pass@k（error_repair 31 题）：deepseek-v4-pro 96.77% / 96.77% / 96.77%（@1/@5/@10）；glm-5.3 98.71% / 100.0% / 100.0%。
- L2 链点检（仅诊断，非评分步骤）：`python tools/l2fix.py <file.lom>`。

## 与既有报告的关系

- [REPORT-2026-09-16-err-repair-patch.md](REPORT-2026-09-16-err-repair-patch.md)：24 题 480/480 先例——本报告在其任务面扩到 31 题，重叠 24 题结论跨时点复现；其"error_repair 全类目不再有'加入于实测之后'的任务"的宣称自本日起对 31 题版本重新成立（123-129 设计于 2026-10-01，实测于 2026-10-03）。
- [REPORT-2026-09-07-passk.md](REPORT-2026-09-07-passk.md)：116 集全量 pass@k（99.1%）——不受影响（078 失败形态不变）。
- [REPORT-2026-08-31-multimodel.md](REPORT-2026-08-31-multimodel.md)：113 集四模型单采样——历史基线不动。

## 遗留

1. **126 提示词欠定**（取证 A）：建议任务维护者补输出契约或收敛 expected；处置前该题失败不应计为模型修复能力失败。本轮不改任务文件（铁律）。
2. **L2 面题的评分面空洞**（要点 3）：127 的宿主面通过无法区分"真 L2 修复 / 死分支保留 / 常量捷径"，本轮 18 个宿主通过中 0 个真修复。若未来要把"L2 修复能力"做成可宣称指标，需在 runner 之外加 l2fix/全链门（生成时门槛已有，评分时没有）——设计议题，不在本轮范围。
3. 118（078 明确版对照题）仍无多采样记录——与本轮无关，维持 09-16 报告的既有记录状态。
