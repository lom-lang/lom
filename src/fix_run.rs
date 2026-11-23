// src/fix_run.rs — designs/0028（v1.17.0）L2：`lom fix <file> --run` 的运行时观察
//
// 形态：`lom fix <file> --plan --run` / `--apply --run`；可选 `--run-budget <ms>`、
// `--allow-io`。不带 --run 的 fix 路径不经本模块（默认路径逐字节不变）。
//
// 流程（设计 §4.2）：
//   1. 闸门（§4.2-2 / §4.4）——三种 skipped，reason 区分：
//      - static_error：静态有 error 级诊断，程序本就跑不起来，不执行；
//      - package_import：同目录有 lom.toml 且源码 import 了非标准库模块。
//        fix 面不解析依赖包——执行只会得到"未知模块/包"的伪观察，且包源码里
//        的写内建会绕过下面的名字闸门。（没有 lom.toml 时拼错的模块名照常
//        执行并如实观察为 RUNTIME005——那与 `lom <file>` 的行为一致。）
//      - io_gate：源码词法层面出现持久副作用内建名，除非 --allow-io。
//   2. 执行一次（§4.2-3 / §4.5）：参数为空（argv[0] = 文件路径）；程序 stdout
//      丢弃（Interpreter::discard_output——不走管道）；256MB 栈工作线程 +
//      mpsc 通道回传，主线程 recv_timeout 等预算（缺省 5000 ms）。超时 =>
//      outcome "timeout"，输出照常走完后进程退出（放弃失控线程）。
//   3. 捕获首个运行时诊断（经 L1 侧通道带位置），追加喂给 fix::generate_plan
//      ——既有六个 RUNTIME 臂产出建议，全是 hint。
//   4. 信封（§4.6）：lom-fix/v1 新增可选顶层字段 run { attempted, outcome,
//      budget_ms }（本批另加可选 reason / detail 两个说明字段——§4.4"并说明"、
//      §4.5"结构化说明"的落点）；计划数组追加 stage: "runtime" 的计划。
//
// 退出码（§4.7）：分析完成的一切情形 rc 0（error / pass / timeout / skipped）。
// 观察环节自身失败（trace 出口打不开、解释器线程异常终止）= 工具失败带 rc 2。
//
// wasm 目标（playground 引擎）：该目标无线程，无法设运行预算——--run 直接
// 拒绝（stderr 说明 + rc 2），不 ship 无界执行路径（登记为跨目标分歧）。
// 该目标下执行路径的若干项（Pass/Error/Timeout 的构造、tool_failure、trace
// 注入项）不被使用——仅对 wasm 放开 dead_code；桌面构建零豁免。
#![cfg_attr(target_family = "wasm", allow(dead_code))]

use crate::ast::Item;
use crate::diagnostics::{Diagnostic, Diagnostics, Severity};
use crate::fix::{self, Plan};
#[cfg(not(target_family = "wasm"))]
use crate::interpreter::Interpreter;
use crate::json::escape_str;
use crate::lexer::{Lexer, Token};
use crate::parser;
#[cfg(not(target_family = "wasm"))]
use std::sync::mpsc::RecvTimeoutError;

/// 运行预算缺省值（裁决点 4 甲：5000 ms，与 playground 引擎 5 秒看门狗同量级）
pub const DEFAULT_RUN_BUDGET_MS: u64 = 5000;

/// 持久副作用内建名单（裁决点 3 甲）。依据：持久副作用只可能来自这两个
/// 内建——println / print 只写被丢弃的 stdout；file_read / file_exists /
/// env::args 只读。内建集冻结（SPEC §14），名单穷尽；**新增带持久副作用内建
/// 的 RFC 必须同步扩展此名单**（下方单测钉"file 模块导出 − 只读两名 == 本
/// 名单"，漏登记即红）。
pub const PERSISTENT_IO_BUILTINS: &[&str] = &["file_write", "file_append"];

/// 解释器工作线程栈——与 main.rs 运行路径同款 256MB：DEFAULT_MAX_CALL_DEPTH
/// （60000 层）按此栈大小标定，小栈会让合法深递归先于守卫物理溢出。
#[cfg(not(target_family = "wasm"))]
const WORKER_STACK_BYTES: usize = 256 * 1024 * 1024;

/// `run.outcome` 四态（设计 §4.6）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOutcome {
    Pass,
    Error,
    Timeout,
    Skipped,
}

impl RunOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunOutcome::Pass => "pass",
            RunOutcome::Error => "error",
            RunOutcome::Timeout => "timeout",
            RunOutcome::Skipped => "skipped",
        }
    }
}

/// `run.reason` 稳定 token（可选说明字段；prose 在 `run.detail`）
pub const REASON_STATIC_ERROR: &str = "static_error";
pub const REASON_PACKAGE_IMPORT: &str = "package_import";
pub const REASON_IO_GATE: &str = "io_gate";
pub const REASON_BUDGET_EXCEEDED: &str = "budget_exceeded";
pub const REASON_TRACE_OPEN_FAILED: &str = "trace_open_failed";
pub const REASON_ABNORMAL_TERMINATION: &str = "abnormal_termination";

/// `run` 信封（lom-fix/v1 与 lom-apply/v1 的可选顶层字段；仅 --run 时在场）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunReport {
    /// 是否真的发起了执行（skipped 为 false）
    pub attempted: bool,
    pub outcome: RunOutcome,
    pub reason: Option<&'static str>,
    pub detail: Option<String>,
    pub budget_ms: u64,
}

impl RunReport {
    fn make(
        attempted: bool,
        outcome: RunOutcome,
        reason: Option<&'static str>,
        detail: Option<String>,
        budget_ms: u64,
    ) -> Self {
        RunReport {
            attempted,
            outcome,
            reason,
            detail,
            budget_ms,
        }
    }

    fn skipped(reason: &'static str, detail: String, budget_ms: u64) -> Self {
        Self::make(
            false,
            RunOutcome::Skipped,
            Some(reason),
            Some(detail),
            budget_ms,
        )
    }

    fn tool_failure(reason: &'static str, detail: String, budget_ms: u64) -> Self {
        Self::make(
            true,
            RunOutcome::Error,
            Some(reason),
            Some(detail),
            budget_ms,
        )
    }

    /// 退出码（设计 §4.7）：分析完成（error / pass / timeout / skipped）= 0；
    /// 观察环节自身失败（trace 出口打不开 / 解释器线程异常终止）= 工具失败带 2。
    pub fn exit_code(&self) -> i32 {
        match self.reason {
            Some(REASON_TRACE_OPEN_FAILED) | Some(REASON_ABNORMAL_TERMINATION) => 2,
            _ => 0,
        }
    }
}

/// 一次 --run 观察的完整结果
#[derive(Debug)]
pub struct Observation {
    pub run: RunReport,
    /// outcome == Error 且拿到诊断时在场（L1 侧通道带位置）
    pub runtime_diag: Option<Diagnostic>,
}

/// `--trace` 族在观察运行期间的注入项（设计 §4.8：trace 照常产出）
#[derive(Debug, Clone)]
pub struct TraceOpts {
    pub out_path: Option<String>,
    /// 0 = 无限（缺省值由调用方用 TRACE_DEFAULT_MAX_EVENTS）
    pub max_events: usize,
    pub builtins: bool,
}

/// --run 的全部选项（cli.rs 解析后由 run_fix 组装）
#[derive(Debug, Clone)]
pub struct RunOpts {
    pub budget_ms: u64,
    pub allow_io: bool,
    pub trace: Option<TraceOpts>,
}

// ===== 闸门 =====

/// 闸门结论
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateVerdict {
    Proceed,
    Skip {
        reason: &'static str,
        detail: String,
    },
}

/// 源码词法层面出现的持久副作用内建名（首个命中）：(内建名, 行, 列)。
///
/// 按真实 token 判定，不做子串匹配——注释/字符串里提到 file_write、或
/// `file_writer` 这类不同标识符都不算。保守面（设计 §4.4）：名字出现在死
/// 分支、或被局部变量遮蔽时同样命中——误拒可用 --allow-io 放行，方向安全。
pub fn io_gate_hit(src: &str) -> Option<(&'static str, usize, usize)> {
    let (tokens, _) = Lexer::new(src).tokenize_recover();
    tokens.iter().find_map(|t| match &t.token {
        Token::Ident(name) => PERSISTENT_IO_BUILTINS
            .iter()
            .find(|b| **b == name.as_str())
            .map(|b| (*b, t.line, t.col)),
        _ => None,
    })
}

/// 源码里首个 import 了非标准库模块的 (模块名, 行)。调用前提：静态无 error
/// 级诊断，源码可解析。
fn package_import(src: &str) -> Option<(String, usize)> {
    parser::Parser::parse_recover(src)
        .program
        .items
        .iter()
        .find_map(|item| match item {
            Item::Import(imp) if !crate::interpreter::is_stdlib_module(&imp.module) => {
                Some((imp.module.clone(), imp.span.line))
            }
            _ => None,
        })
}

/// --run 闸门（设计 §4.2-2 / §4.4）。`diags` = 该源码的静态诊断。
pub fn gate(src: &str, path: &str, diags: &Diagnostics, allow_io: bool) -> GateVerdict {
    let errors = diags
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    if errors > 0 {
        return GateVerdict::Skip {
            reason: REASON_STATIC_ERROR,
            detail: format!(
                "静态分析有 {} 条 error 级诊断——程序本就跑不起来，未执行；先按修复计划处理静态错误。",
                errors
            ),
        };
    }
    // 包项目：发现规则与 main.rs 运行路径一致（文件所在目录的 lom.toml）
    let has_manifest = std::path::Path::new(path)
        .parent()
        .map(|d| d.join("lom.toml"))
        .is_some_and(|p| p.exists());
    if has_manifest && let Some((module, line)) = package_import(src) {
        return GateVerdict::Skip {
            reason: REASON_PACKAGE_IMPORT,
            detail: format!(
                "第 {} 行 import 了外部包 '{}'——fix 面不解析依赖包，未执行（执行只会得到伪运行时诊断，且包源码里的写内建不受名字闸门约束）。",
                line, module
            ),
        };
    }
    if !allow_io && let Some((name, line, col)) = io_gate_hit(src) {
        return GateVerdict::Skip {
            reason: REASON_IO_GATE,
            detail: format!(
                "第 {} 行第 {} 列出现持久副作用内建 {}——会写/追加磁盘文件，未执行；确认可放行后加 --allow-io。",
                line, col, name
            ),
        };
    }
    GateVerdict::Proceed
}

// ===== 执行（桌面：工作线程 + 预算）=====

/// 工作线程回传：Ok(None) = 程序跑完；Ok(Some(diag)) = 首个运行时诊断（带位置）；
/// Err = 观察环节自身失败（trace 出口打不开）
#[cfg(not(target_family = "wasm"))]
type WorkerPayload = Result<Option<Diagnostic>, String>;

/// 工作线程体
#[cfg(not(target_family = "wasm"))]
fn run_program(src: &str, path: &str, trace: Option<&TraceOpts>) -> WorkerPayload {
    let program = parser::Parser::parse_recover(src).program;
    let mut interp = Interpreter::new();
    // 程序 stdout 丢弃（不走管道）；参数为空（argv[0] = 文件路径）
    interp.set_discard_output(true);
    interp.set_program_args(vec![path.to_string()]);
    if let Some(t) = trace {
        interp
            .set_trace(path, t.out_path.as_deref(), t.max_events, t.builtins)
            .map_err(|e| {
                format!(
                    "无法打开 trace 出口 '{}': {}",
                    t.out_path.as_deref().unwrap_or("<stderr>"),
                    e
                )
            })?;
    }
    let result = interp.run(&program);
    interp.trace_finish();
    Ok(match result {
        Ok(()) => None,
        Err(e) => {
            let mut diags = Diagnostics::new(path);
            // L1：位置取解释器侧通道（最内层出错表达式；DepthLimit 自带位置）
            diags.add_runtime(&e, src, interp.err_site());
            diags.diagnostics.pop()
        }
    })
}

/// 主线程对 recv 结果的分类（纯函数——Timeout / Disconnected 两路可直测）
#[cfg(not(target_family = "wasm"))]
fn classify(recv: Result<WorkerPayload, RecvTimeoutError>, budget_ms: u64) -> Observation {
    match recv {
        Ok(Ok(None)) => Observation {
            run: RunReport::make(true, RunOutcome::Pass, None, None, budget_ms),
            runtime_diag: None,
        },
        Ok(Ok(Some(diag))) => Observation {
            run: RunReport::make(true, RunOutcome::Error, None, None, budget_ms),
            runtime_diag: Some(diag),
        },
        Ok(Err(msg)) => Observation {
            run: RunReport::tool_failure(REASON_TRACE_OPEN_FAILED, msg, budget_ms),
            runtime_diag: None,
        },
        Err(RecvTimeoutError::Timeout) => Observation {
            run: RunReport::make(
                true,
                RunOutcome::Timeout,
                Some(REASON_BUDGET_EXCEEDED),
                Some(format!(
                    "运行超过预算 {} ms，已放弃等待；未获得运行时诊断——程序可能含死循环（或预算过小，可用 --run-budget 调大）。",
                    budget_ms
                )),
                budget_ms,
            ),
            runtime_diag: None,
        },
        // 发送端在工作线程体内，仅在 run_program 返回后发送：通道先于消息
        // 断开 = 线程 panic 中途终止（panic 信息已由默认 hook 打到 stderr）
        Err(RecvTimeoutError::Disconnected) => Observation {
            run: RunReport::tool_failure(
                REASON_ABNORMAL_TERMINATION,
                "观察运行期间解释器线程异常终止（见上方 panic 信息）；未获得运行时诊断。"
                    .to_string(),
                budget_ms,
            ),
            runtime_diag: None,
        },
    }
}

/// 执行一次（设计 §4.5 机制：大栈工作线程 + mpsc 通道 + 主线程 recv_timeout）
#[cfg(not(target_family = "wasm"))]
pub fn execute(src: &str, path: &str, opts: &RunOpts) -> Observation {
    use std::sync::mpsc;
    use std::time::Duration;

    let (tx, rx) = mpsc::channel::<WorkerPayload>();
    let w_src = src.to_string();
    let w_path = path.to_string();
    let w_trace = opts.trace.clone();
    let spawned = std::thread::Builder::new()
        .stack_size(WORKER_STACK_BYTES)
        .spawn(move || {
            let _ = tx.send(run_program(&w_src, &w_path, w_trace.as_ref()));
        });
    if let Err(e) = spawned {
        return Observation {
            run: RunReport::tool_failure(
                REASON_ABNORMAL_TERMINATION,
                format!("无法启动解释器线程: {}", e),
                opts.budget_ms,
            ),
            runtime_diag: None,
        };
    }
    classify(
        rx.recv_timeout(Duration::from_millis(opts.budget_ms)),
        opts.budget_ms,
    )
}

/// wasm 目标：无线程 => 无法设运行预算 => 直接拒绝（不 ship 无界执行路径）
#[cfg(target_family = "wasm")]
pub fn execute(_src: &str, _path: &str, _opts: &RunOpts) -> Observation {
    eprintln!(
        "fix --run 在 wasm 目标不可用：该目标无线程，无法设置运行预算——未执行；请在桌面版 lom 上使用，或直接运行程序。"
    );
    std::process::exit(2)
}

/// --run 的完整流程（设计 §4.2 步骤 2-3）：闸门 → 执行。
pub fn observe(src: &str, path: &str, diags: &Diagnostics, opts: &RunOpts) -> Observation {
    match gate(src, path, diags, opts.allow_io) {
        GateVerdict::Skip { reason, detail } => Observation {
            run: RunReport::skipped(reason, detail, opts.budget_ms),
            runtime_diag: None,
        },
        GateVerdict::Proceed => execute(src, path, opts),
    }
}

// ===== 运行时修复计划 + 信封序列化 =====

/// 把观察到的运行时诊断喂给 fix::generate_plan——走既有六个 RUNTIME 臂
/// （全部 hint-only，设计 §1.1；单测钉住）。
pub fn runtime_plans(diag: &Diagnostic, src: &str, path: &str) -> Vec<Plan> {
    let mut diags = Diagnostics::new(path);
    diags.diagnostics.push(diag.clone());
    diags.ok = false;
    let program = parser::Parser::parse_recover(src).program;
    let fns = fix::fn_infos(&program, src);
    fix::generate_plan(&diags, src, &fns).plans
}

/// `run` 信封的字段行（4 空格缩进；末行无尾逗号）——fix::to_json_with 的
/// run_fields 入参。
pub fn run_fields_json(run: &RunReport) -> String {
    let mut out = String::new();
    out.push_str(&format!("    \"attempted\": {},\n", run.attempted));
    out.push_str(&format!("    \"outcome\": \"{}\",\n", run.outcome.as_str()));
    if let Some(r) = run.reason {
        out.push_str(&format!("    \"reason\": \"{}\",\n", r));
    }
    if let Some(d) = &run.detail {
        out.push_str(&format!("    \"detail\": \"{}\",\n", escape_str(d)));
    }
    out.push_str(&format!("    \"budget_ms\": {}\n", run.budget_ms));
    out
}

/// `--apply --run --json` 追加在 lom-apply/v1 `ok` 之后的顶层字段：
/// `runtime_plans`（与 lom-fix/v1 的计划对象同形）+ `run`（末字段，无尾逗号）。
pub fn apply_extra_json(run: &RunReport, runtime_plans: &[Plan]) -> String {
    // runtime_plans 之后还有 run 字段：收尾带逗号（is_last = false）
    let mut out = fix::plans_field_json("runtime_plans", runtime_plans, false);
    out.push_str("  \"run\": {\n");
    out.push_str(&run_fields_json(run));
    out.push_str("  }\n");
    out
}

/// 人类模式的"运行观察"段（--run 时追加在 fix 输出之后）。
///
/// `first_plan_above`：Some(k) = plan 模式，运行时计划已并入上方诊断列表
/// （k 为其中首条的 1-based 序号），本段只指路；None = apply 模式（apply
/// 输出无计划列表），本段内列出。两种模式都点明：仅提示，--apply 不应用。
pub fn run_to_human(
    run: &RunReport,
    runtime_diag: Option<&Diagnostic>,
    runtime_plans: &[Plan],
    first_plan_above: Option<usize>,
) -> String {
    let mut out = String::from("\n=== 运行观察（--run）===\n");
    let head = format!(
        "  结果: {}（预算 {} ms）",
        run.outcome.as_str(),
        run.budget_ms
    );
    match (run.outcome, runtime_diag) {
        (RunOutcome::Pass, _) => {
            out.push_str(&format!("{}——程序正常跑完，未观察到运行时诊断。\n", head));
        }
        // 超时 / 闸门拒绝 / 观察环节自身失败（rc 2）：没有诊断，只有说明
        (RunOutcome::Timeout, _) | (RunOutcome::Skipped, _) | (RunOutcome::Error, None) => {
            out.push_str(&format!(
                "{}——{}\n",
                head,
                run.detail.as_deref().unwrap_or("")
            ));
        }
        (RunOutcome::Error, Some(d)) => {
            out.push_str(&format!("{}——观察到运行时诊断。\n", head));
            out.push_str(&Diagnostics::render_one(d));
            match first_plan_above {
                Some(k) => out.push_str(&format!(
                    "  修复建议见上方诊断 #{}（仅提示——不会被 --apply 应用）。\n",
                    k
                )),
                None if runtime_plans.is_empty() => {
                    out.push_str("  （该运行时诊断没有对应的修复建议。）\n")
                }
                None => {
                    out.push_str("  运行时修复计划（仅提示——不会被 --apply 应用）：\n");
                    for p in runtime_plans {
                        out.push_str(&format!(
                            "    [{}] {}:{} {}\n",
                            p.diagnostic.code,
                            p.diagnostic.line,
                            p.diagnostic.col,
                            p.diagnostic.message
                        ));
                        for (j, f) in p.fixes.iter().enumerate() {
                            out.push_str(&format!(
                                "      {}. {} [{}]\n",
                                j + 1,
                                f.description,
                                f.confidence.as_str()
                            ));
                        }
                    }
                }
            }
        }
    }
    out
}

// ===== 单元测试 =====

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Stage;
    use crate::fix::{ActionKind, FixPlan};

    fn clean_diags() -> Diagnostics {
        Diagnostics::from_parse_result("fn main() -> Unit\n    println(1)\nend\n", "t.lom")
    }

    fn opts(budget_ms: u64) -> RunOpts {
        RunOpts {
            budget_ms,
            allow_io: false,
            trace: None,
        }
    }

    /// needle 在 src 中首次出现的 1-based (line, col)——期望位置由源码文本算出
    fn pos_of(src: &str, needle: &str) -> (usize, usize) {
        let at = src.find(needle).expect("源码中应有该片段");
        let before = &src[..at];
        (
            before.matches('\n').count() + 1,
            before.rsplit('\n').next().unwrap().chars().count() + 1,
        )
    }

    /// 运行时诊断的测试构造（走真实解释器路径的版本见 execute_* 测试）
    fn rt_diag(code: &str, msg: &str, line: usize, col: usize) -> Diagnostic {
        Diagnostic {
            severity: Severity::Error,
            stage: Stage::Runtime,
            code: code.into(),
            message: msg.into(),
            file: "t.lom".into(),
            line,
            col,
            source_line: None,
            is_hole: false,
            hint: None,
        }
    }

    // ---- 闸门 ----

    /// 闸门名单钉死：恰为 file_write / file_append 两个名字
    #[test]
    fn gate_list_is_exactly_the_two_write_builtins() {
        assert_eq!(
            PERSISTENT_IO_BUILTINS.to_vec(),
            vec!["file_write", "file_append"]
        );
    }

    /// 名单穷尽的结构性钉子（设计 §4.4"内建集冻结，名单穷尽"）：file 模块
    /// 的导出 − 只读两名（file_read / file_exists）必须恰等于闸门名单。
    /// 新增带持久副作用内建的 RFC 若漏登记名单，本测试即红。
    #[test]
    fn gate_list_covers_all_non_read_file_builtins() {
        let (_, file_exports) = crate::interpreter::stdlib_modules()
            .iter()
            .find(|(m, _)| *m == "file")
            .expect("file 模块应存在");
        let mut non_read: Vec<&str> = file_exports
            .iter()
            .copied()
            .filter(|n| *n != "file_read" && *n != "file_exists")
            .collect();
        non_read.sort_unstable();
        let mut gate_list: Vec<&str> = PERSISTENT_IO_BUILTINS.to_vec();
        gate_list.sort_unstable();
        assert_eq!(
            non_read, gate_list,
            "file 模块的写内建集合变了——同步 PERSISTENT_IO_BUILTINS"
        );
        assert!(!PERSISTENT_IO_BUILTINS.contains(&"file_read"));
        assert!(!PERSISTENT_IO_BUILTINS.contains(&"file_exists"));
        // 两个名字都是真实内建（不是悬空名字）
        for name in PERSISTENT_IO_BUILTINS {
            assert_eq!(crate::interpreter::module_of(name), Some("file"));
        }
    }

    /// 词法判定而非子串：直接调用 / as 别名 import / 死分支 命中；
    /// 注释、字符串、同前缀不同标识符、只读内建 不命中
    #[test]
    fn io_gate_is_lexical_not_substring() {
        let direct = "from file import { file_write }\nfn main() -> Unit\n    let u = file_write(\"a.txt\", \"x\")\nend\n";
        // 首个命中 = import 行的那个名字（不是调用点）
        let (l, c) = pos_of(direct, "file_write");
        assert_eq!(io_gate_hit(direct), Some(("file_write", l, c)));
        assert_eq!((l, c), (1, 20), "import 行的 file_write");
        let alias = "from file import { file_append as w }\nfn main() -> Unit\n    let u = w(\"a.txt\", \"x\")\nend\n";
        let (l, c) = pos_of(alias, "file_append");
        assert_eq!(io_gate_hit(alias), Some(("file_append", l, c)));
        let dead = "fn main() -> Unit\n    if False\n        let u = file_write(\"a\", \"b\")\n    end\nend\n";
        let (l, c) = pos_of(dead, "file_write(\"a\"");
        assert_eq!(io_gate_hit(dead), Some(("file_write", l, c)));
        let mentioned = "# file_write 只在注释里\nfn main() -> Unit\n    println(\"file_append 在字符串里\")\n    let file_writer = 1\n    println(file_writer)\nend\n";
        assert_eq!(io_gate_hit(mentioned), None, "注释/字符串/同前缀标识符不算");
        let read_only = "from file import { file_read, file_exists }\nfn main() -> Unit\n    let s = file_read(\"a.txt\")\n    let e = file_exists(\"b.txt\")\n    println(s)\n    println(e)\nend\n";
        assert_eq!(io_gate_hit(read_only), None, "只读内建不触发闸门");
    }

    /// 静态 error 优先于其余两项：即使源码同时含 file_write、即使给了
    /// --allow-io，reason 也是 static_error
    #[test]
    fn gate_skips_on_static_error_first() {
        let src = "from file import { file_write }\nfn main() -> Unit\n    println(nosuchvar)\n    let u = file_write(\"a\", \"b\")\nend\n";
        let mut diags = clean_diags();
        diags.diagnostics.push(Diagnostic {
            severity: Severity::Error,
            stage: Stage::Type,
            code: "NAM003".into(),
            message: "调用未定义函数 'nosuchvar'".into(),
            file: "t.lom".into(),
            line: 3,
            col: 13,
            source_line: None,
            is_hole: false,
            hint: None,
        });
        let expected = GateVerdict::Skip {
            reason: REASON_STATIC_ERROR,
            detail:
                "静态分析有 1 条 error 级诊断——程序本就跑不起来，未执行；先按修复计划处理静态错误。"
                    .to_string(),
        };
        assert_eq!(gate(src, "t.lom", &diags, false), expected);
        assert_eq!(gate(src, "t.lom", &diags, true), expected);
    }

    /// warning 不拦截（渐进式语义）：只有 warning 的源码照常执行
    #[test]
    fn gate_proceeds_on_warnings_only() {
        let src = "fn main() -> Unit\n    println(len(\"abc\"))\nend\n";
        let mut diags = Diagnostics::new("t.lom");
        let program = parser::Parser::parse_recover(src).program;
        crate::typechecker::check_program(&program, src, "t.lom", &mut diags);
        assert!(
            diags
                .diagnostics
                .iter()
                .all(|d| d.severity == Severity::Warning),
            "前置：应只有 warning"
        );
        assert_eq!(gate(src, "t.lom", &diags, false), GateVerdict::Proceed);
    }

    /// 包项目跳过 / 非包项目照常：拼错的模块名（无 lom.toml）照常执行——
    /// 那是程序自己的 RUNTIME005，应如实观察
    #[test]
    fn gate_package_import_only_with_manifest() {
        let src =
            "from mathlib import { square }\nfn main() -> Unit\n    println(square(5))\nend\n";
        let diags = clean_diags();
        assert_eq!(
            gate(src, "no_such_dir/x.lom", &diags, false),
            GateVerdict::Proceed,
            "无 lom.toml：非标准库 import 照常执行"
        );
        let dir = std::env::temp_dir().join(format!("lom_fix_run_gate_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("建临时目录失败");
        std::fs::write(
            dir.join("lom.toml"),
            "name = \"app\"\nversion = \"0.1.0\"\n",
        )
        .expect("写清单失败");
        let in_pkg = dir.join("main.lom").to_string_lossy().into_owned();
        let got = gate(src, &in_pkg, &diags, false);
        assert!(
            matches!(
                got,
                GateVerdict::Skip {
                    reason: REASON_PACKAGE_IMPORT,
                    ..
                }
            ),
            "{:?}",
            got
        );
        let stdlib_src = "from file import { file_read }\nfn main() -> Unit\n    println(file_read(\"x\"))\nend\n";
        assert_eq!(
            gate(stdlib_src, &in_pkg, &clean_diags(), false),
            GateVerdict::Proceed,
            "标准库 import 不算外部包"
        );
        assert!(
            matches!(
                gate(src, &in_pkg, &diags, true),
                GateVerdict::Skip {
                    reason: REASON_PACKAGE_IMPORT,
                    ..
                }
            ),
            "--allow-io 不越过包闸门"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    /// io_gate 跳过 + --allow-io 放行；detail 指出位置
    #[test]
    fn gate_io_skip_and_allow_io_override() {
        let src = "from file import { file_write }\nfn main() -> Unit\n    let u = file_write(\"a.txt\", \"x\")\nend\n";
        let diags = clean_diags();
        let got = gate(src, "t.lom", &diags, false);
        match &got {
            GateVerdict::Skip { reason, detail } => {
                assert_eq!(*reason, REASON_IO_GATE);
                assert!(detail.contains("--allow-io"), "{}", detail);
                assert!(detail.contains("file_write"), "{}", detail);
            }
            other => panic!("应 skipped，得到 {:?}", other),
        }
        assert_eq!(gate(src, "t.lom", &diags, true), GateVerdict::Proceed);
    }

    // ---- 分类纯函数（桌面）----

    /// Timeout：发送端存活、预算内无消息——等满预算，分析完成 rc 0
    #[cfg(not(target_family = "wasm"))]
    #[test]
    fn classify_timeout() {
        use std::sync::mpsc;
        use std::time::{Duration, Instant};
        let (tx, rx) = mpsc::channel::<WorkerPayload>();
        let t0 = Instant::now();
        let obs = classify(rx.recv_timeout(Duration::from_millis(20)), 20);
        assert!(t0.elapsed() >= Duration::from_millis(15), "应等满预算");
        assert_eq!(obs.run.attempted, true);
        assert_eq!(obs.run.outcome, RunOutcome::Timeout);
        assert_eq!(obs.run.reason, Some(REASON_BUDGET_EXCEEDED));
        assert_eq!(obs.run.budget_ms, 20);
        assert!(obs.runtime_diag.is_none());
        assert_eq!(obs.run.exit_code(), 0, "超时属于分析完成");
        drop(tx);
    }

    /// Disconnected = 工作线程异常终止 => 工具失败 rc 2
    #[cfg(not(target_family = "wasm"))]
    #[test]
    fn classify_disconnected_is_tool_failure() {
        use std::sync::mpsc;
        use std::time::Duration;
        let (tx, rx) = mpsc::channel::<WorkerPayload>();
        drop(tx);
        let obs = classify(rx.recv_timeout(Duration::from_millis(1000)), 1000);
        assert_eq!(obs.run.outcome, RunOutcome::Error);
        assert_eq!(obs.run.reason, Some(REASON_ABNORMAL_TERMINATION));
        assert!(obs.runtime_diag.is_none());
        assert_eq!(obs.run.exit_code(), 2);
    }

    /// pass / error（带诊断）/ trace 出口打不开
    #[cfg(not(target_family = "wasm"))]
    #[test]
    fn classify_pass_error_and_trace_failure() {
        use std::sync::mpsc;
        use std::time::Duration;
        let (tx, rx) = mpsc::channel::<WorkerPayload>();
        tx.send(Ok(None)).unwrap();
        let obs = classify(rx.recv_timeout(Duration::from_millis(1000)), 7);
        assert_eq!(
            obs.run,
            RunReport::make(true, RunOutcome::Pass, None, None, 7)
        );
        drop(tx);

        let (tx, rx) = mpsc::channel::<WorkerPayload>();
        tx.send(Ok(Some(rt_diag("RUNTIME000", "整数除以零", 4, 13))))
            .unwrap();
        let obs = classify(rx.recv_timeout(Duration::from_millis(1000)), 7);
        assert_eq!(obs.run.outcome, RunOutcome::Error);
        assert_eq!(obs.run.exit_code(), 0);
        assert_eq!(obs.runtime_diag.map(|d| (d.line, d.col)), Some((4, 13)));
        drop(tx);

        let (tx, rx) = mpsc::channel::<WorkerPayload>();
        tx.send(Err("无法打开 trace 出口 'x'".to_string())).unwrap();
        let obs = classify(rx.recv_timeout(Duration::from_millis(1000)), 7);
        assert_eq!(obs.run.reason, Some(REASON_TRACE_OPEN_FAILED));
        assert_eq!(obs.run.exit_code(), 2);
        drop(tx);
    }

    // ---- 真实工作线程（桌面）----

    /// pass：干净程序跑完；程序输出被丢弃（本进程 stdout 不受影响由进程级
    /// 集成测试钉，这里只验状态）
    #[cfg(not(target_family = "wasm"))]
    #[test]
    fn execute_pass() {
        let src = "fn main() -> Unit\n    println(\"不得出现在任何地方\")\nend\n";
        let obs = execute(src, "t.lom", &opts(5000));
        assert_eq!(obs.run.outcome, RunOutcome::Pass);
        assert!(obs.run.attempted);
        assert!(obs.runtime_diag.is_none());
    }

    /// error：运行时诊断经 L1 带位置
    #[cfg(not(target_family = "wasm"))]
    #[test]
    fn execute_error_carries_position() {
        let src = "fn main() -> Unit\n    let a = 1\n    let b = 0\n    println(a / b)\nend\n";
        let obs = execute(src, "p03.lom", &opts(5000));
        assert_eq!(obs.run.outcome, RunOutcome::Error);
        let d = obs.runtime_diag.expect("应有运行时诊断");
        assert_eq!((d.line, d.col), (4, 13));
        assert_eq!(d.code, "RUNTIME000");
        assert_eq!(d.stage, Stage::Runtime);
        assert_eq!(d.source_line.as_deref(), Some("    println(a / b)"));
    }

    /// timeout：死循环在预算量级内返回（放弃失控线程）
    #[cfg(not(target_family = "wasm"))]
    #[test]
    fn execute_timeout_within_budget() {
        use std::time::{Duration, Instant};
        let src = "fn main() -> Unit\n    let mut i = 0\n    while True\n        i = i + 1\n    end\nend\n";
        let t0 = Instant::now();
        let obs = execute(src, "t.lom", &opts(150));
        let elapsed = t0.elapsed();
        assert_eq!(obs.run.outcome, RunOutcome::Timeout);
        assert!(
            elapsed >= Duration::from_millis(150),
            "应等满预算：{:?}",
            elapsed
        );
        assert!(
            elapsed < Duration::from_secs(5),
            "应在预算量级内返回：{:?}",
            elapsed
        );
        assert!(obs.runtime_diag.is_none());
    }

    // ---- 运行时计划 / 信封 ----

    /// 运行时计划：stage runtime、带位置、全是 hint、retry false
    #[test]
    fn runtime_plans_are_hints_with_position() {
        let src = "fn main() -> Unit\n    let a = 1\n    let b = 0\n    println(a / b)\nend\n";
        let (l, c) = pos_of(src, "a / b");
        let plans = runtime_plans(&rt_diag("RUNTIME000", "整数除以零", l, c), src, "p03.lom");
        assert_eq!(plans.len(), 1);
        let p = &plans[0];
        assert_eq!(p.diagnostic.stage, Stage::Runtime);
        assert_eq!(p.diagnostic.code, "RUNTIME000");
        assert_eq!((p.diagnostic.line, p.diagnostic.col), (4, 13));
        assert!(!p.retry);
        assert!(!p.fixes.is_empty());
        assert!(
            p.fixes.iter().all(|f| f.action == ActionKind::Hint),
            "运行时计划必须全是 hint"
        );
    }

    /// 安全锁（源头）：六个 RUNTIME 臂对任何消息都只产出 Hint 动作、
    /// retry 恒 false——运行时建议永不可被 --apply 自动应用（设计 §4.6）。
    #[test]
    fn all_runtime_arms_are_hint_only() {
        let src = "fn main() -> Unit\n    println(1)\nend\n";
        let samples = [
            ("RUNTIME000", "整数除以零"),
            (
                "RUNTIME000",
                "递归深度超过 60000 层（256MB 栈的安全上限）：函数 'down' 疑似缺少终止条件",
            ),
            ("RUNTIME001", "len 期望 String，得到 Int"),
            ("RUNTIME002", "未定义变量: 'x'"),
            ("RUNTIME003", "代码洞（hole）@ 3:5 — 该处解析失败，无法执行"),
            ("RUNTIME004", "提前返回"),
            ("RUNTIME005", "未知模块/包 'm'（PKG005）"),
        ];
        for (code, msg) in samples {
            let mut diags = Diagnostics::new("t.lom");
            diags.diagnostics.push(rt_diag(code, msg, 2, 5));
            diags.ok = false;
            let plan = fix::generate_plan(&diags, src, &[]);
            assert_eq!(plan.plans.len(), 1, "{}", code);
            let p = &plan.plans[0];
            assert_eq!(p.diagnostic.stage, Stage::Runtime, "{}", code);
            assert!(!p.retry, "{} 不得标记 retry", code);
            assert!(!p.fixes.is_empty(), "{} 至少一条建议", code);
            assert!(
                p.fixes.iter().all(|f| f.action == ActionKind::Hint),
                "{} 出现了非 Hint 动作",
                code
            );
        }
    }

    /// run 信封字段行：三必填 + 可选 reason/detail 的精确形态
    #[test]
    fn run_fields_json_shape() {
        let pass = RunReport::make(true, RunOutcome::Pass, None, None, 5000);
        assert_eq!(
            run_fields_json(&pass),
            "    \"attempted\": true,\n    \"outcome\": \"pass\",\n    \"budget_ms\": 5000\n"
        );
        let skipped = RunReport::skipped(REASON_IO_GATE, "含 \"file_write\"".to_string(), 300);
        assert_eq!(
            run_fields_json(&skipped),
            "    \"attempted\": false,\n    \"outcome\": \"skipped\",\n    \"reason\": \"io_gate\",\n    \"detail\": \"含 \\\"file_write\\\"\",\n    \"budget_ms\": 300\n"
        );
    }

    /// fix::to_json_with：None 分支与 to_json 逐字节一致；Some 分支在 summary
    /// 之后追加 run 字段、计划数组照常
    #[test]
    fn fix_to_json_with_run_appends_field_only_when_asked() {
        let src = "fn main() -> Unit\n    let a = 1\n    let b = 0\n    println(a / b)\nend\n";
        let diags = clean_diags();
        let (l, c) = pos_of(src, "a / b");
        let plans = runtime_plans(&rt_diag("RUNTIME000", "整数除以零", l, c), src, "t.lom");
        // 默认路径逐字节不变（纯静态计划）
        let static_only = fix::generate_plan(&diags, src, &[]);
        assert_eq!(
            fix::to_json_with(&static_only, None),
            fix::to_json(&static_only)
        );
        // 静态计划 + 运行时计划（run_fix 的合并形态）
        let mut all = static_only.plans;
        all.extend(plans);
        let merged = FixPlan {
            file: "t.lom".to_string(),
            ok: false,
            plans: all,
        };
        assert_eq!(fix::to_json_with(&merged, None), fix::to_json(&merged));
        // --run 形态
        let run = RunReport::make(true, RunOutcome::Error, None, None, 5000);
        let json = fix::to_json_with(&merged, Some(&run_fields_json(&run)));
        assert!(
            json.contains("  },\n  \"run\": {\n    \"attempted\": true,\n    \"outcome\": \"error\",\n    \"budget_ms\": 5000\n  },\n  \"plans\": [\n"),
            "{}",
            json
        );
        assert!(json.contains("\"stage\": \"runtime\""));
        assert_eq!(json.matches("\"run\"").count(), 1);
    }
}
