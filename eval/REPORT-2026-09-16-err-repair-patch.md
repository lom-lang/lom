# Lom error_repair 高温补测报告 — 2026-09-16（③ 包）

**测试日期**: 2026-09-16（③ 修复闭环资产包，docs/TODO.md 四连包第三项）
**触发**: 八审 R46（P1）留口——positioning 宣称分层如实化时，任务 119/120（warning 级修复题）被记录为"LLM 采样实测待补"；本包顺带把新增的 121/122 一并覆盖。
**管线**: `eval/llm_eval.py --samples 10 --temperature 1.0 --only 10_error_repair`（采集）+ `eval/runner/run.ps1`（评分，事实源不二设）+ `eval/passk_summarize.py`（汇总）；原始回复留档 `eval/candidates_rerun/<model>_err22|err24/`（本地，.gitignore 惯例）。

---

## 总览

| 轮 | 集版本 | 模型 | 采样 | 结果 |
|---|---|---|---|---|
| err22 | error_repair 22 题（含 119/120，不含 121/122） | deepseek-v4-pro（thinking, t=1.0） | ×10 | **22 题 220/220 全过（含 119/120 各 10/10）** |
| err22 | 同上 | glm-5.3（Coding Plan, t=1.0） | ×10 | **22 题 220/220 全过（含 119/120 各 10/10）** |
| err24 | error_repair 24 题（22 题版 + 新增 121/122） | deepseek-v4-pro（thinking, t=1.0） | ×10 | **24 题 240/240 全过（含 121/122 各 10/10）** |
| err24 | 同上 | glm-5.3（Coding Plan, t=1.0） | ×10 | **24 题 240/240 全过（含 121/122 各 10/10）** |

合计 80 次分类调用、880 个候选提取零缺失、**920/920 通过单元**。

## 结论

1. **R46 留口闭环**：任务 119（MUT001 warning 修复）/120（NAM005 warning 修复）在 temperature=1.0 高温下双模型各 10/10——warning 级修复题与既有 error 级修复题同样稳健，"修复闭环在高温下有效"的宣称覆盖面补齐。
2. **新题即测**：任务 121（LEX005 全角标点）/122（TYPE002 真值 warning 预告 RUNTIME001）设计当日即完成双模型×10 实测，全过——error_repair 类目现值 24 题全部有高温采样记录，不再有"加入于实测之后"的任务。
3. **跨批次稳定性**：119/120 在 err22 与 err24 两轮独立采集中均 10/10（跨 20 次采样零失败）；error_repair 全类目与 2026-09-07 pass@k 复测（当时 20 题 200/200）形态一致——分类级结论跨时间稳定。
4. **全量口径恢复**：positioning §3 第一条可如实升级为"error_repair 24 任务双模型 × 10 采样全过（480/480，2026-09-16）"；评测集层面，121 任务中唯一无采样记录的是 118（078 明确版对照题——Q4 时有单采样 10/10 对照实测，078 歧义版 0/10）。

## 方法论（与 09-07 pass@k 严格对齐）

- 同模型（deepseek-v4-pro thinking / glm-5.3 Coding Plan 端点）、同温度 1.0、同 max_tokens 16384、同评分器（stdout + 退出码 0）。
- err22 与 err24 两轮：err22 基于 22 题版 prompt（B 包后、③ 包任务扩充前），err24 基于 24 题版（含新任务 121/122，prompt 文件同步后采集）。
- `passk_summarize` 按 manifest 全集建矩阵，无候选的非本类任务显示 0/10 属矩阵形状假象，本报告只统计 error_repair 24 题列。

## 与既有报告的关系

- [REPORT-2026-09-07-passk.md](REPORT-2026-09-07-passk.md)：116 集全量 pass@k（99.1%）——本报告不改变其结论，只补 error_repair 类目的任务覆盖面。
- [REPORT-2026-08-31-multimodel.md](REPORT-2026-08-31-multimodel.md)：113 集四模型单采样——历史基线不动。
