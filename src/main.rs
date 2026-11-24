// Lom CLI 入口 — Phase 3.1
// 用法:
//   lom <file.lom>                运行 .lom 程序（默认）
//   lom <file.lom> --json         仅诊断，输出 JSON（不执行）
//   lom <file.lom> --check        仅诊断，输出人类可读（不执行）
//   lom info <file.lom> [--json]  导出类型信息（Phase 2.6）
//   lom fix <file.lom> [--plan] [--json]  生成修复计划（Phase 2.7）
//   lom fix <file.lom> --apply [--dry-run] [--json]  应用修复到源文件（Phase 3.1）
//   lom --json <file.lom>         等价（选项可前可后）
//   lom --help | -h               帮助

use cli::CliArgs;
use cli::apply_iterative;
use cli::extract_completion_uri;
use cli::extract_did_change_params;
use cli::extract_did_open_params;
use cli::extract_hover_params;
use cli::merge_packages_for_wasm;
use cli::parse_args;
use cli::print_help;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::process;

mod apply;
mod ast;
mod cli;
mod codes;
mod diagnostics;
mod doc;
mod dump;
mod fix;
mod fix_history;
mod fix_run;
mod fmt;
mod info;
mod interpreter;
mod json;
mod l2fix;
mod lexer;
mod lsp;
mod package;
mod parser;
mod repl;
mod typechecker;
mod wasm;
mod wasm_codegen;

fn main() {
    // playground（designs/0019）：wasm 目标单线程直跑——wasi p1 无线程支持，
    // spawn 恒 Err；256MB 大栈缓解也不适用（浏览器 wasm 栈由引擎决定，深度
    // 由解释器侧 cfg 降阈值的软件守卫结构化先报）。桌面路径保持不变。
    #[cfg(target_family = "wasm")]
    {
        main_inner();
    }
    // Phase 5.0: 用大栈线程运行解释器，缓解树遍历解释器递归深度限制
    // （自举验证发现：非尾递归的 Lom 程序在长输入时栈溢出 Rust 默认 1MB 栈）
    #[cfg(not(target_family = "wasm"))]
    {
        let child = std::thread::Builder::new()
            .stack_size(256 * 1024 * 1024) // 256MB
            .spawn(main_inner)
            .expect("failed to spawn interpreter thread");
        // process::exit 在线程内会直接终止进程；正常路径 main_inner 已自带退出码。
        // R56（九审）：worker 线程 panic（join 返回 Err）不得被吞成 exit 0——
        // 默认 panic hook 已把 panic 信息打到 stderr，这里补一行结论并映射非零退出码。
        if let Err(_panic_payload) = child.join() {
            eprintln!("内部错误：解释器线程异常终止（见上方 panic）");
            // β-2（ACRI v0.1 §2.2）：工具自身失败带（内部错误）≥2——与
            // check 的诊断 error=1 区分（此前 1）。
            process::exit(2);
        }
    }
}

fn main_inner() {
    let args: Vec<String> = env::args().collect();
    let cli = parse_args(&args);

    if cli.help {
        print_help(&args[0]);
        return;
    }

    // Phase 6.1: --version（版本号单一事实源是 Cargo.toml）
    if cli.version {
        println!("lom {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    // Phase 4.1.3: lom fix --history 不需要文件参数，提前处理
    if cli.subcommand.as_deref() == Some("fix") && cli.history {
        let history_path = std::path::Path::new(".lom/fix-history.jsonl");
        match fix_history::read_history(history_path) {
            Ok(entries) => {
                if cli.json {
                    print!("{}", fix_history::to_json(&entries));
                } else {
                    print!("{}", fix_history::to_human(&entries));
                }
            }
            Err(e) => {
                eprintln!("读取修复历史失败: {}", e);
                // β-2（ACRI v0.1 §2.2）：IO 错误 = 工具自身失败带 rc 2（此前 1）
                process::exit(2);
            }
        }
        return;
    }

    // Phase 4.2: lom repl 启动交互式 REPL，不需要文件参数
    if cli.subcommand.as_deref() == Some("repl") {
        run_repl();
        return;
    }

    // Phase 4.3: lom lsp 启动 LSP 服务器（stdio JSON-RPC），不需要文件参数
    if cli.subcommand.as_deref() == Some("lsp") {
        run_lsp();
        return;
    }

    // Phase 4.4: lom build 读取 lom.toml，解析依赖，对包源码执行类型检查
    // Phase 7.2: lom build <file.lom> --target wasm [-o out.wasm] 编译到 WASM
    if cli.subcommand.as_deref() == Some("build") {
        if let Some(f) = &cli.file {
            // run_build_wasm 永不返回（-> !），故无 return
            run_build_wasm(f, cli.target.as_deref(), cli.output.as_deref());
        }
        run_build(cli.json);
        return;
    }

    // L2.3 包批（designs/0008 §6.1，裁决 1 丙）：lom pkg-expand <main.lom> [--list]
    // 把 main.lom + 依赖包源码展开为单编译单元文本到 stdout（L2 编译器
    // self_comp.lom 的输入）；--list 只输出包名逗号串（脚本取 L2 第三参）。
    if cli.subcommand.as_deref() == Some("pkg-expand") {
        match &cli.file {
            Some(f) => run_pkg_expand(f, cli.list_pkgs),
            None => {
                eprintln!("用法：lom pkg-expand <main.lom> [--list]");
                // β-2：用法错误 = 工具失败带 rc 2（此前 1）
                process::exit(2);
            }
        }
    }

    // ===== 子命令：codes（designs/0023 §5 β-2 / ACRI v0.1 §2.1 REQUIRED）=====
    // 机器可读码表出口：宿主九命名空间 + L2 八族的静态注册表
    // （src/codes.rs；登记依据 target/probes/beta2/codes_inventory.md）。
    // 不需要文件参数；rc 恒 0。
    if cli.subcommand.as_deref() == Some("codes") {
        if cli.json {
            println!("{}", codes::to_json());
        } else {
            codes::print_human();
        }
        return;
    }

    // ===== 子命令：explain（designs/0027 D5 / ACRI v0.1 §2.1 OPTIONAL——
    // 逐码长文档出口，codes.rs EXPLAIN 注册表的首个生产消费方）=====
    // <code> 经 file 槽传入（l2fix 先例：位置参数装任意输入；本块在文件
    // 读取之前分发，"NAM003" 不会被当路径读）。未知码 → 结构化错误
    // （lom-explain/v1 error 信封 / stderr）+ 同族近邻建议，rc 2
    // （ACRI §2.2 工具失败带）。
    if cli.subcommand.as_deref() == Some("explain") {
        let code = match &cli.file {
            Some(c) => c,
            None => {
                eprintln!("用法：lom explain <code> [--json]");
                // β-2 口径：用法错误 = 工具失败带 rc 2
                process::exit(2);
            }
        };
        match codes::explain_lookup(code) {
            Some((entry, doc)) => {
                if cli.json {
                    print!("{}", codes::explain_to_json(entry, doc));
                } else {
                    codes::print_explain_human(entry, doc);
                }
            }
            None => {
                let neighbors = codes::same_family_codes(code);
                if cli.json {
                    print!("{}", codes::explain_error_json(code, &neighbors));
                } else {
                    codes::print_explain_error(code, &neighbors);
                }
                process::exit(2);
            }
        }
        return;
    }

    // ===== 子命令：l2fix（designs/0018 裁决点 1 甲的既定后续——L2 已于
    // v1.8.0 转正，宿主 CLI 补桥接面）：解析已采集的 L2 拒绝文本 → 七族
    // 修复建议（tools/l2fix.py 的 Rust 内建移植）=====
    if cli.subcommand.as_deref() == Some("l2fix") {
        run_l2fix(&cli);
        return;
    }

    let path = match &cli.file {
        Some(p) => p,
        None => {
            print_help(&args[0]);
            // β-2（ACRI v0.1 §2.2）：用法错误（缺输入文件）= 工具失败带 rc 2（此前 1）
            process::exit(2);
        }
    };

    let src = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            // 文件读取错误也输出 JSON 诊断（如果 --json），保持输出一致性
            // （信封内容不变——LEX000 输入错误借用面维持 v1 形态）
            if cli.json {
                let mut diags = diagnostics::Diagnostics::new(path);
                diags.ok = false;
                diags.diagnostics.push(diagnostics::Diagnostic {
                    severity: diagnostics::Severity::Error,
                    stage: diagnostics::Stage::Lex, // 借用 lex 阶段表示输入错误
                    code: "LEX000".into(),
                    message: format!("无法读取文件 '{}': {}", path, e),
                    file: path.to_string(),
                    line: 0,
                    col: 0,
                    source_line: None,
                    is_hole: false,
                    hint: None,
                });
                println!("{}", diags.to_json());
            } else {
                eprintln!("无法读取文件 '{}': {}", path, e);
            }
            // β-2（ACRI v0.1 §2.2）：输入不可读 = 工具自身失败带 rc 2（此前 1）
            // ——与 check 的诊断 error=1 区分（诊断信封照常输出，仅退出码进带）
            process::exit(2);
        }
    };

    // ===== 子命令：info（Phase 2.6 类型信息导出）=====
    if cli.subcommand.as_deref() == Some("info") {
        run_info(&src, path, cli.json);
        return;
    }

    // ===== 子命令：doc（Phase 6.4 文档生成）=====
    if cli.subcommand.as_deref() == Some("doc") {
        run_doc(&src, path, cli.json);
        return;
    }

    // ===== 子命令：fmt（Phase 6.5 格式化）=====
    if cli.subcommand.as_deref() == Some("fmt") {
        run_fmt(&src, path, &cli);
        return;
    }

    // ===== 子命令：fix（Phase 2.7 修复计划 / Phase 3.1 应用修复）=====
    if cli.subcommand.as_deref() == Some("fix") {
        run_fix(&src, path, &cli);
        return;
    }

    // ===== --dump-tokens（Phase 8.1，RFC-0003 自举 lexer 对账工具）=====
    // 格式即契约：每行 "<Token Debug 名> @<line>:<col>"，与自举 --tokens 模式逐字比对。
    // 词法错误摘要走 stderr（不污染 token 流比对）。
    if cli.dump_tokens {
        let (tokens, lex_errors) = lexer::Lexer::new(&src).tokenize_recover();
        for t in &tokens {
            println!("{:?} @{}:{}", t.token, t.line, t.col);
        }
        if !lex_errors.is_empty() {
            eprintln!("（词法含 {} 个错误）", lex_errors.len());
        }
        return;
    }

    // ===== --dump-ast（Phase 8 前置，RFC-0003 §8.1 验收工具）=====
    // dump 是调试/验收产品：打印到 stdout，恒退出 0；
    // 容错解析的 Hole 节点照常输出，解析错误摘要走 stderr。
    if cli.dump_ast {
        let result = parser::Parser::parse_recover(&src);
        print!("{}", dump::dump_program(&result.program));
        if !result.errors.is_empty() {
            eprintln!(
                "（解析含 {} 个错误，AST 中带 Hole 节点）",
                result.errors.len()
            );
        }
        return;
    }

    // ===== 诊断阶段（词法 + 语法，Phase 2.2 容错解析器一次性收集全部错误）=====
    let mut diags = diagnostics::Diagnostics::from_parse_result(&src, path);
    let parsed_ok = diags.ok;

    // ===== 包解析（designs/0030 §3.1 R246 / §3.2 R244）：纯函数 =====
    // --json / --check / 运行三条带文件路径共用同一份结果。
    let packages = resolve_entry_packages(path);
    let graph: Option<&package::DependencyGraph> = match &packages {
        EntryPackages::Resolved(g) => Some(g),
        EntryPackages::NoManifest => None,
        EntryPackages::Failed(err, manifest) => {
            // 坏清单 → 一条结构化 PKG 诊断（rc 1 / ok false），词法 / 语法
            // 诊断照报（排在前面）。带着空 externals 继续做类型检查只会产出
            // 使用点的假 NAM003——入口文件的类型检查跳过（下方按 Failed 判）。
            diags
                .diagnostics
                .push(package::pkg_error_diagnostic(err, manifest.as_deref()));
            diags.ok = false;
            None
        }
    };
    // R104 批（designs/0014 §2.4）单点接线：PKG007 的打印留在调用点——入口
    // 文件解析通过且依赖图解析成功时，每条带文件路径恰打印一次（改前口径）。
    if parsed_ok && let Some(g) = graph {
        package::warn_public_symbol_clashes(g);
    }
    // designs/0029 §3.3（R236）：依赖包诊断的可见性——入口文件的三条路径只查
    // 入口文件本身；这里静默检查各依赖包源文件，有诊断则在 stderr 提示条数。
    // 信封、`ok`、rc 与 stdout 都不变（只提示、不合并）。
    if let Some(g) = graph
        && let Some(line) = dependency_diag_notice(g)
    {
        eprintln!("{}", line);
    }

    // ===== 类型检查（Phase 2.4 渐进式类型检查器）=====
    // 入口文件解析通过且包解析成功时执行；externals = 依赖包公开符号并集
    // （无清单单文件 = 空集，与 check_program 同一函数、同一输入）。
    if parsed_ok && !matches!(packages, EntryPackages::Failed(..)) {
        let program = parser::Parser::parse_recover(&src).program;
        let externals = graph.map(entry_externals).unwrap_or_default();
        typechecker::check_program_with_externals(&program, &src, path, &mut diags, &externals);
    }

    if cli.json {
        // JSON 模式：仅诊断，不执行
        print!("{}", diags.to_json());
        process::exit(if diags.ok { 0 } else { 1 });
    }

    // 这里的提前退出只管两类：词法 / 语法解析失败（改前口径：本分支位于类型
    // 检查之前，只看得到解析诊断）与清单 / 依赖解析失败（designs/0030 §3.1，
    // R246：项目根本跑不起来，rc 1、不执行）。**静态类型错误不在此列**——
    // 渐进式承诺"诊断永不拦截执行"不变：类型检查已挪到本分支之前，故不能再
    // 用 `!diags.ok` 作条件（那会让任何 NAM003 一类的静态错误都挡住运行）。
    if !parsed_ok || matches!(packages, EntryPackages::Failed(..)) {
        // 人类可读诊断输出（--check 模式：诊断是程序产品，输出到 stdout）
        if cli.check {
            print!("{}", diags.to_human());
        } else {
            // 默认运行模式：诊断输出到 stderr（程序失败的错误信息）
            eprint!("{}", diags.to_human());
        }
        process::exit(1);
    }

    // ===== 类型检查（Phase 2.4 渐进式类型检查器）=====
    // --check 模式：类型检查已完成，输出收集到的 TYPE/MAT/NAM 诊断
    if cli.check {
        // 有诊断（error 或 warning）时都输出，便于 LLM/用户看到类型提示
        if !diags.diagnostics.is_empty() {
            print!("{}", diags.to_human());
            // 渐进式语义：仅 error 阻止（退出码 1），warning 不阻止（退出码 0）
            if !diags.ok {
                process::exit(1);
            }
        } else {
            println!("{}: 诊断通过，无错误。", path);
        }
        return;
    }

    // ===== 默认运行模式：类型检查可见化（2026-08-22，评审整改）=====
    // 此前运行模式完全跳过类型检查——"渐进式类型"在默认路径上不可见（名不副实）。
    // 现在：照常执行类型检查，诊断打印到 stderr，但**永不拦截执行**——
    // 渐进式承诺不变（动态可跑），只是让 LLM/用户在每次运行时都能看到类型反馈。
    if !diags.diagnostics.is_empty() {
        eprint!("{}", diags.to_human());
    }

    // ===== 执行阶段 =====
    let program = parser::Parser::parse_recover(&src).program;
    let mut interp = interpreter::Interpreter::new();
    // Phase 4.4: 文件所在目录有 lom.toml 时加载依赖包
    // 使 `from pkg import { ... }` 能解析外部包符号。
    // designs/0030 §3.1：依赖图取上方纯函数解析的那份——清单 / 依赖解析
    // 失败的场合已在静态面以 PKG 诊断 rc 1 退出，到不了这里。
    if let Some(g) = graph {
        interp.load_packages(g);
    }
    // Phase 3.5: 传递程序参数（-- 之后的参数），供 env::args() 读取
    // 第一个参数约定为 .lom 文件路径（与大多数 CLI 约定一致：argv[0] = 程序名）
    let mut full_args = vec![path.to_string()];
    full_args.extend(cli.program_args.iter().cloned());
    interp.set_program_args(full_args);
    // ===== designs/0026（v1.14.0）：执行 trace 注入（--trace 族；运行模式
    // 专用——--check/--json/子命令路径不经此处。set_program_args 同款公开
    // setter 先例）。--trace-out 打不开 = 工具失败带 rc 2（β-2 口径）=====
    if cli.trace {
        let max_events = cli
            .trace_max_events
            .unwrap_or(interpreter::TRACE_DEFAULT_MAX_EVENTS);
        if let Err(e) = interp.set_trace(
            path,
            cli.trace_out.as_deref(),
            max_events,
            cli.trace_builtins,
        ) {
            eprintln!(
                "无法打开 trace 出口 '{}': {}",
                cli.trace_out.as_deref().unwrap_or("<stderr>"),
                e
            );
            process::exit(2);
        }
    }
    let run_result = interp.run(&program);
    // designs/0026：尾行 summary——Ok/运行时错误两路都写（错误路径随后的
    // process::exit(1) 不丢事件：文件侧逐行 flush）
    interp.trace_finish();
    if let Err(e) = run_result {
        // 运行时错误：构造诊断输出
        // designs/0028（v1.17.0）L1：位置取解释器位置侧通道——最内层出错
        // 表达式的 span 起点（eval_expr 外壳写入）。此前此处写死 (0, 0)
        // （Phase 2.3 时 AST 尚无 span 的历史欠账；表达式级 span 自 v0.21.0
        // 起已在 AST 上）。不在表达式求值路径上抛出的错误（import 解析等）
        // 侧通道为空，维持 (0, 0)；DepthLimit 自带位置，由 from_runtime 覆盖。
        // designs/0029 §3.6（R242）：错误发生在依赖包的函数体内时，位置属于
        // 包文件——按来源文件取源码行并填该条诊断的 file（信封顶层 file
        // 仍是入口文件）。来源为入口文件（None）时走原路径，行为与此前逐字
        // 节相同；来源文件读不到（运行期间被移走等）退回入口文件渲染。
        match interp.err_source_file() {
            None => diags.add_runtime(&e, &src, interp.err_site()),
            Some(dep_file) => match fs::read_to_string(dep_file) {
                Ok(dep_src) => diags.add_runtime_in_file(&e, dep_file, &dep_src, interp.err_site()),
                Err(_) => diags.add_runtime(&e, &src, interp.err_site()),
            },
        }
        // designs/0029 §3.4（R237）：只打新增的这一条运行时诊断，其后补一行
        // 完整汇总（静态 + 运行时的合计）。静态诊断已在上方"类型检查可见化"
        // 块打过第一遍——此前这里整份重打，静态诊断在一次运行里出现两次
        // （汇总行也两次）。无静态诊断时本分支的输出与此前逐字节相同。
        if let Some(last) = diags.diagnostics.last() {
            eprint!("{}", diagnostics::Diagnostics::render_one(last));
        }
        eprint!("{}", diags.summary_line());
        process::exit(1);
    }
}

/// Phase 2.6: 执行 `lom info` 子命令
///
/// 输出类型信息：函数签名、枚举定义、导入声明。
/// - `--json`：输出 lom-info/v1 schema（给 LLM 消费）
/// - 默认：人类可读格式（终端浏览）
///
/// 不执行类型检查 — info 只描述"声明了什么"。
/// 解析失败时输出错误诊断（JSON 或人类可读）。
fn run_info(src: &str, path: &str, json: bool) {
    let result = parser::Parser::parse_recover(src);

    if !result.is_ok() {
        // 解析失败：输出诊断（复用 diagnostics 模块）
        let diags = diagnostics::Diagnostics::from_parse_result(src, path);
        if json {
            print!("{}", diags.to_json());
        } else {
            eprint!("{}", diags.to_human());
        }
        process::exit(1);
    }

    let info = info::collect_info(&result.program, path);

    if json {
        print!("{}", info::to_json(&info));
    } else {
        print!("{}", info::to_human(&info));
    }
    process::exit(0);
}

/// Phase 6.7 评审整改 → designs/0030 §3.1（R246）函数拆分：入口文件的包解析
/// 结果——纯函数 [`resolve_entry_packages`] 的返回值（不打印、不退出）。
enum EntryPackages {
    /// 入口文件所在目录无 lom.toml（不读盘——单文件程序零成本）
    NoManifest,
    /// 清单与依赖图解析成功
    Resolved(package::DependencyGraph),
    /// 清单 / 依赖解析失败。第二个分量 = 出错清单的真实路径：入口清单自身
    /// 加载失败时其错误只带占位符 `'lom.toml'`，此处给出真实路径；依赖解析
    /// 失败（路径不存在 / 循环 / 包源码不可读）时它是声明该依赖的清单；
    /// 嵌套依赖的清单自身解析失败时无从得知是哪一份 → None（不伪造）。
    Failed(package::PkgError, Option<String>),
}

/// designs/0030 §3.1（R246）/ §3.2（R244）：按入口文件所在目录解析 lom.toml
/// 与依赖图——**纯函数**（不打印、不退出、不吞错）。
///
/// 收集依赖包公开符号（fn/enum/变体名）供 typechecker 免于对包符号误报
/// NAM003（Phase 6.7 评审整改：此前 pkg_demo 在默认运行路径喷 5 条假 error）。
/// 此前的 collect_package_symbols 把"清单或依赖解析失败"与"没有依赖包"压成
/// 同一个返回值（`Err(_) => Vec::new()`）——静态面（--check / --json）于是对
/// 根本跑不起来的项目报告干净，入口用到包符号时另出使用点的假 NAM003。现
/// 调用方拿到 [`EntryPackages::Failed`] 并把它变成一条结构化 PKG 诊断
/// （rc 1 / ok false），且跳过入口文件的类型检查。
///
/// PKG007 的打印不在本函数——留在各调用点（`warn_public_symbol_clashes`：
/// --check / 默认 run / build-wasm 三路径各打印一次；fix / apply 面不打）。
fn resolve_entry_packages(path: &str) -> EntryPackages {
    let file_dir = std::path::Path::new(path)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let toml_path = file_dir.join("lom.toml");
    if !toml_path.exists() {
        return EntryPackages::NoManifest;
    }
    let toml_display = toml_path.display().to_string();
    match package::load_manifest_file(&toml_path) {
        Ok(manifest) => match package::resolve_dependencies(&manifest, &file_dir) {
            Ok(graph) => EntryPackages::Resolved(graph),
            Err(e) => {
                // 嵌套依赖的清单自身解析失败：错误只带占位符 'lom.toml'
                //（入口清单此时已解析成功，失败的必是某个依赖的清单），
                // 无从得知是哪一份——不伪造
                let nested_placeholder = matches!(
                    &e,
                    package::PkgError::ManifestRead { path, .. }
                        if path.as_str() == "lom.toml"
                );
                if nested_placeholder {
                    EntryPackages::Failed(e, None)
                } else {
                    EntryPackages::Failed(e, Some(toml_display))
                }
            }
        },
        Err(e) => EntryPackages::Failed(e, Some(toml_display)),
    }
}

/// 入口文件路径的 externals：依赖图全部包 public_symbols 的扁平并集（原
/// collect_package_symbols 的返回值口径——不带打印）。
fn entry_externals(graph: &package::DependencyGraph) -> Vec<String> {
    graph
        .packages
        .values()
        .flat_map(|p| p.public_symbols.iter().cloned())
        .collect()
}

/// Phase 6.4: 执行 `lom doc` 子命令
///
/// 解析源码后从 AST 提取顶层 fn/enum 签名，从源码回捞 `#` 文档注释，
/// 输出 Markdown（默认）或 lom-doc/v1 JSON（--json）。
fn run_doc(src: &str, path: &str, json: bool) {
    let result = parser::Parser::parse_recover(src);

    if !result.is_ok() {
        let diags = diagnostics::Diagnostics::from_parse_result(src, path);
        if json {
            print!("{}", diags.to_json());
        } else {
            eprint!("{}", diags.to_human());
        }
        process::exit(1);
    }

    let module = doc::collect_doc(&result.program, src, path);

    if json {
        print!("{}", doc::to_json(&module));
    } else {
        print!("{}", doc::to_markdown(&module));
    }
    process::exit(0);
}

/// Phase 6.5: 执行 `lom fmt` 子命令
///
/// 默认输出格式化结果到 stdout（预览）；--apply 就地改写；--check 已格式化退出 0 否则 1。
/// 词法错误拒绝格式化（fmt 必须基于可靠 token 流，见 fmt.rs 设计说明）。
fn run_fmt(src: &str, path: &str, cli: &CliArgs) {
    match fmt::format_source(src) {
        Ok(formatted) => {
            if cli.check {
                if formatted == src {
                    println!("{}: 已格式化。", path);
                    process::exit(0);
                } else {
                    eprintln!("{}: 未格式化（运行 lom fmt --apply 修复）", path);
                    process::exit(1);
                }
            } else if cli.apply {
                if formatted == src {
                    println!("{}: 无需修改。", path);
                } else {
                    match fs::write(path, &formatted) {
                        Ok(_) => println!("{}: 已格式化并写入。", path),
                        Err(e) => {
                            eprintln!("写入文件 '{}': {}", path, e);
                            // β-2：IO 错误 = 工具失败带 rc 2（此前 1）
                            process::exit(2);
                        }
                    }
                }
                process::exit(0);
            } else {
                print!("{}", formatted);
                process::exit(0);
            }
        }
        Err(e) => {
            eprintln!("无法格式化 '{}': {}", path, e);
            process::exit(1);
        }
    }
}

/// Phase 7.2: lom build <file.lom> --target wasm [-o out.wasm]
/// 把 Lom 源文件编译为 WASM 二进制（动态语义，与解释器逐字对齐为长期目标）。
/// 解析错误（LEX/PARSE）按既有惯例走人类可读诊断；不支持的构造报编译期错误。
fn run_build_wasm(file: &str, target: Option<&str>, output: Option<&str>) -> ! {
    // 1. target 校验
    match target {
        Some("wasm") => {}
        Some(t) => {
            eprintln!("未知编译目标 '{}'（当前仅支持 --target wasm）", t);
            // β-2：用法错误 = 工具失败带 rc 2（此前 1）
            process::exit(2);
        }
        None => {
            eprintln!("lom build <file> 需要 --target wasm（不带文件的 lom build 是包管理流程）");
            // β-2：用法错误 = 工具失败带 rc 2（此前 1）
            process::exit(2);
        }
    }
    // 2. 读源文件
    let src = match fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("无法读取文件 '{}': {}", file, e);
            // β-2：输入不可读 = 工具失败带 rc 2（此前 1）
            process::exit(2);
        }
    };
    // 3. 解析（容错模式收集全部诊断；有 error 则不编译）
    let diags = diagnostics::Diagnostics::from_parse_result(&src, file);
    if !diags.ok {
        eprint!("{}", diags.to_human());
        process::exit(1);
    }
    let program = parser::Parser::parse_recover(&src).program;
    // 7.8 包链接：main.lom 所在目录有 lom.toml 时，把依赖包源码合并进编译单元
    // （包内 item 在前，主文件在后；重名函数后主文件覆盖——对齐解释器 load_packages 语义；
    // 发现目录 = 文件所在目录而非 cwd——对齐解释器路径，D 包二期修复）
    let base_dir = std::path::Path::new(file)
        .parent()
        .unwrap_or(std::path::Path::new("."));
    let (program, pkg_names) = merge_packages_for_wasm(program, base_dir);
    // 7.9：类型检查可见性对齐——编译前跑检查器，诊断走 stderr，不拦截编译（渐进式承诺与解释器一致）
    {
        let mut tdiags = diagnostics::Diagnostics::new(file);
        // R94 批 2b：包符号 externals（对齐默认运行/--check 路径的
        // check_program_with_externals 形态）——合并单元中包内
        // `from libinner import {triple as t3}` 的别名经 collect_import
        // 的 externals else-if 分支放行，NAM003 假阳性消失
        // designs/0030 §3.1（R246）：PKG007 的打印留在本调用点；依赖图由
        // 同一纯函数解析——merge_packages_for_wasm 已对清单 / 依赖解析失败
        // rc 1 退出，走到这里的只有 Resolved / NoManifest。
        let externals = match resolve_entry_packages(file) {
            EntryPackages::Resolved(g) => {
                package::warn_public_symbol_clashes(&g);
                entry_externals(&g)
            }
            _ => Vec::new(),
        };
        typechecker::check_program_with_externals(&program, &src, file, &mut tdiags, &externals);
        // R105 批（designs/0014 §2.3/§2.5）：warning 亦可见（对齐默认 run 路径
        // "诊断非空即打 stderr"形态）——NAM006 遮蔽提示在 build 路径不再被
        // "仅 error 才打印"吞掉；error 照旧不拦截编译（渐进式承诺不变）。
        if !tdiags.diagnostics.is_empty() {
            eprint!("{}", tdiags.to_human());
        }
    }
    // 4. 编译
    // 无包走 compile_program（零开销路径），有包走 with_packages
    let bytes = match if pkg_names.is_empty() {
        wasm_codegen::compile_program(&program)
    } else {
        wasm_codegen::compile_program_with_packages(&program, &pkg_names)
    } {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };
    // 5. 写产物（默认 <file> 换 .wasm 后缀）
    let out_path = match output {
        Some(o) => o.to_string(),
        None => {
            let p = std::path::Path::new(file);
            p.with_extension("wasm").to_string_lossy().into_owned()
        }
    };
    match fs::write(&out_path, &bytes) {
        Ok(()) => {
            eprintln!("已编译 {} → {}（{} 字节）", file, out_path, bytes.len());
            process::exit(0);
        }
        Err(e) => {
            eprintln!("无法写入 '{}': {}", out_path, e);
            // β-2：IO 错误 = 工具失败带 rc 2（此前 1）
            process::exit(2);
        }
    }
}

/// L2.3 包批（designs/0008 §6.1，裁决 1 丙）：执行 `lom pkg-expand` 子命令
///
/// 把 main.lom 与依赖包源码在文本层展开为单编译单元（包内顶层 import 剥除、
/// 主文件 import 保留、包序 = 包根路径排序），输出到 stdout 供 L2 编译器
/// （examples/selfhost/self_comp.lom）消费。`--list` 只输出包名逗号串——
/// verify 脚本取它作 self_comp 的第三参（裁决 3 甲：argv 包名清单），
/// 默认模式的 stdout 保持纯展开产物。
///
/// 退出码：
///   0 — 展开成功（含无 lom.toml 的幂等退化：主文件源码原样）
///   1 — 文件读取失败 / PKG001/002/003（清单缺失字段、路径不存在、循环依赖——
///       复用宿主结构化诊断原文，stderr 输出）
fn run_pkg_expand(file: &str, list_only: bool) {
    match cli::expand_package_unit(std::path::Path::new(file)) {
        Ok((unit, names)) => {
            if list_only {
                println!("{}", names.join(","));
            } else {
                print!("{}", unit);
            }
            process::exit(0);
        }
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}

/// designs/0018 裁决点 1 甲的既定后续（L2 已于 v1.8.0 转正）：执行
/// `lom l2fix` 子命令——把**已采集的 L2 拒绝文本**（`codegen error: [L2xxx]`
/// 等码行）解析为七族修复建议（l2fix.rs = tools/l2fix.py 的 Rust 内建移植，
/// 自包含——不 shell 调 python、不依赖外部 .lom 文件与子进程链路）。
///
/// 文本来源三选一（--text 优先）：`--text <s>` / 文件路径 / `-`（stdin——
/// 真实用法 `lom examples/selfhost/self_comp.lom -- bad.lom out.hex 2>&1 |
/// lom l2fix -`，判定靠码行、COMPILE-ERROR 恒 rc=0 的管道采集面）。
///
/// 输出：默认人类可读；--json 输出 l2-fix/v2 轻量 schema（独立 schema，
/// 不撞宿主 lom-fix/v1——L2 工具码不进宿主协议，designs/0018 §3 既定口径；
/// β-2 起 v2：entries 条目新增可选 position 字段——codegen 行 ln:cl 位置
/// 前缀提级为结构化字段，无位置时字段缺省，message 维持 v1 原文）。
///
/// 退出码：0 = 分析完成（有建议条目 / "L2 编译通过" / 零码行提示）；
/// 2 = 输入读取失败 / 缺参（β-2 起：工具自身失败带，此前 1）。
fn run_l2fix(cli: &CliArgs) {
    use std::io::Read;
    let (text, label) = if let Some(t) = &cli.text {
        (t.clone(), "--text".to_string())
    } else {
        match &cli.file {
            Some(f) if f == "-" => {
                let mut buf = String::new();
                if std::io::stdin().read_to_string(&mut buf).is_err() {
                    eprintln!("无法读取 stdin");
                    // β-2：IO 错误 = 工具失败带 rc 2（此前 1）
                    process::exit(2);
                }
                (buf, "-".to_string())
            }
            Some(f) => match fs::read_to_string(f) {
                Ok(s) => (s, f.clone()),
                Err(e) => {
                    eprintln!("无法读取文件 '{}': {}", f, e);
                    // β-2：输入不可读 = 工具失败带 rc 2（此前 1）
                    process::exit(2);
                }
            },
            None => {
                eprintln!("用法：lom l2fix <file|-> [--json] [--text <s>]");
                // β-2：用法错误 = 工具失败带 rc 2（此前 1）
                process::exit(2);
            }
        }
    };
    let entries = l2fix::analyze(&text);
    if cli.json {
        println!("{}", l2fix::to_json(&entries, &label));
    } else {
        l2fix::print_human(&entries, &text, &label);
    }
    process::exit(0);
}

fn run_build(json: bool) {
    let toml_path = std::path::Path::new("lom.toml");
    if !toml_path.exists() {
        if json {
            println!("{}", build_error_json("当前目录无 lom.toml"));
        } else {
            eprintln!("错误：当前目录无 lom.toml（Phase 4.4 包管理清单）");
            eprintln!("创建最小清单：");
            eprintln!("  name = \"myapp\"");
            eprintln!("  version = \"0.1.0\"");
            eprintln!();
            eprintln!("  [dependencies]");
            eprintln!("  lib = {{ path = \"../lib\" }}");
        }
        // β-2：输入缺失（未编码用法错误）= 工具失败带 rc 2（此前 1）
        process::exit(2);
    }

    let manifest = match package::load_manifest_file(toml_path) {
        Ok(m) => m,
        Err(e) => {
            if json {
                println!("{}", build_error_json(&e.to_string()));
            } else {
                eprintln!("清单解析失败: {}", e);
            }
            process::exit(1);
        }
    };

    let root_path = std::path::Path::new(".");
    let graph = match package::resolve_dependencies(&manifest, root_path) {
        Ok(g) => g,
        Err(e) => {
            if json {
                println!("{}", build_error_json(&e.to_string()));
            } else {
                eprintln!("依赖解析失败: {}", e);
            }
            process::exit(1);
        }
    };

    // designs/0029 §3.1（R234）：逐文件检查的结论参与退出码——任一源文件有
    // error 级诊断 → 1（诊断带）；源文件不可读 → 2（工具失败带，ACRI §2.2
    // "unreadable input ≥ 2"）；仅 warning 维持 0。人读与 JSON 两个模式跑
    // **同一套**逐文件检查：JSON 模式静默（不打 ✓/✗ 行与诊断），只为决定
    // rc；信封字段不变（designs/0021 "JSON 模式 schema 面稳定"的边界保留）。
    // 此前（designs/0021 批口径）诊断不置退出码、恒 0——该边界于 designs/0029
    // 批经用户裁决推翻。
    let externals_map = package_closure_externals(&graph);
    let mut any_error = false;
    let mut any_unreadable = false;

    // 人类可读输出
    // 缺口 B（designs/0021 §1）：无文件 build 此前不发 PKG007——本流程
    // 不经带文件三路径的 PKG007 调用点（resolve_entry_packages 之后的单点
    // 接线），两包
    // 撞名在依赖图解析成功后此处补发（stderr、不拦截；JSON 路径不打，
    // schema 面不动）。
    if !json {
        package::warn_public_symbol_clashes(&graph);
    }
    // v1.4.13 登记项 2（v1.4.9 登记）：包级 externals 预计算——每包
    // manifest.dependencies 出发的传递闭包公开符号并集（图已过环检测，
    // DFS 安全；见 package_closure_externals）
    if !json {
        println!("Lom build — 包依赖解析结果");
        println!("  项目: {} v{}", manifest.name, manifest.version);
    }
    if graph.packages.is_empty() {
        if !json {
            println!("  依赖: （无）");
        }
    } else {
        if !json {
            println!("  依赖: {} 个", graph.packages.len());
        }
        for (name, pkg) in &graph.packages {
            if !json {
                println!("    - {} (path: {})", name, pkg.root.display());
                println!("      源码文件: {} 个", pkg.source_files.len());
                println!("      公开符号: {} 个", pkg.public_symbols.len());
            }
            // 对包源码执行类型检查
            // v1.4.13 登记项 2（v1.4.9 登记）：此前逐文件 check_program
            // （无 externals），包源内跨包符号引用（含 as 别名 t3 与
            // 非别名真名）误报 NAM003 error。改为按包依赖闭包 externals
            // 检查（对齐带文件路径 entry_externals 的放行语义）。
            // 缺口 A（designs/0021 §1）收口：再并上「本包 public_symbols
            // − 当前文件自身顶层符号集」——多文件包内跨文件引用（文件 B
            // 用同包文件 A 的 fn）不再假阳性 NAM003。减法在
            // check_and_report_file 内做（必做——见其文档）。
            for file in &pkg.source_files {
                match check_and_report_file(
                    file,
                    &externals_map[name],
                    Some(&pkg.public_symbols),
                    !json,
                ) {
                    FileVerdict::NoError => {}
                    FileVerdict::HasError => any_error = true,
                    FileVerdict::Unreadable => any_unreadable = true,
                }
            }
        }
    }
    // 缺口 C（designs/0021 §1）：主文件诊断面——resolve_dfs 只对
    // manifest.dependencies 递归入图，根项目自身不入图，根目录源文件在
    // 无文件 build 中此前结构性不可见。与包级对称（包查全部文件），
    // 逐个检查根目录（lom.toml 所在目录）全部 .lom 文件；externals 传
    // 全图并集减去当前文件自身符号（R125 三十审：根路径减法与缺口 A
    // 包路径对称——防同文件重复 fn 撞包名 NAM002→NAM006 降级）。
    // 根目录无 .lom 文件则跳过（不报错）。
    let root_files = package::collect_lom_files(root_path);
    if !root_files.is_empty() {
        if !json {
            println!("  主项目源码: {} 个", root_files.len());
        }
        let root_externals: Vec<String> = graph
            .packages
            .values()
            .flat_map(|p| p.public_symbols.iter().cloned())
            .collect();
        for file in &root_files {
            match check_and_report_file(file, &root_externals, None, !json) {
                FileVerdict::NoError => {}
                FileVerdict::HasError => any_error = true,
                FileVerdict::Unreadable => any_unreadable = true,
            }
        }
    }
    if json {
        // JSON 输出：汇总每个包的源码检查结果（信封字段不随诊断变化——
        // designs/0021 schema 面稳定；R234 只让 rc 参与口径）。
        // designs/0029 §3.1（R243）：字符串字段一律经 json::escape_str——
        // 此前 path / error 未转义直拼，Windows 路径含反斜杠时输出不是
        // 合法 JSON（`"path":".\dep"`）。
        print!(
            "{{\"schema\":\"lom-build/v1\",\"ok\":true,\"package\":\"{}\",\"version\":\"{}\",\"dependencies\":[",
            json::escape_str(&manifest.name),
            json::escape_str(&manifest.version)
        );
        let mut first = true;
        for (name, pkg) in &graph.packages {
            if !first {
                print!(",");
            }
            first = false;
            print!(
                "{{\"name\":\"{}\",\"path\":\"{}\",\"symbols\":[",
                json::escape_str(name),
                json::escape_str(&pkg.root.display().to_string())
            );
            let mut sym_first = true;
            for sym in &pkg.public_symbols {
                if !sym_first {
                    print!(",");
                }
                sym_first = false;
                print!("\"{}\"", json::escape_str(sym));
            }
            print!("]}}");
        }
        println!("]}}");
    } else {
        println!("\n依赖解析成功，共 {} 个包。", graph.packages.len());
    }
    // R234 退出码：工具失败带（2）优先于诊断带（1）——与"无 lom.toml = 2"
    // 同带；不可读的说明行已由 check_and_report_file 打到 stderr。
    if any_unreadable {
        process::exit(2);
    }
    if any_error {
        process::exit(1);
    }
}

/// designs/0029 §3.1（R243）：lom-build/v1 失败信封——`error` 文本经
/// json::escape_str（清单/依赖错误的文本含 Windows 路径反斜杠）。
fn build_error_json(error: &str) -> String {
    format!(
        "{{\"schema\":\"lom-build/v1\",\"ok\":false,\"error\":\"{}\"}}",
        json::escape_str(error)
    )
}

/// designs/0029 §3.1（R234）：无文件 build 对单个源文件的检查结论
/// （check_and_report_file 的返回值——决定进程退出码）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileVerdict {
    /// 无诊断或仅 warning（warning 不置退出码）
    NoError,
    /// 有 error 级诊断（判据 `!diags.ok`——仅 error 级置 false）
    HasError,
    /// 源文件不可读
    Unreadable,
}

/// 缺口 A/C（designs/0021 §1）：单文件「读取 + 解析 + 类型检查 +
/// ✓/✗ 结果行」——包源文件循环与根目录主文件循环共用（输出格式一致）。
///
/// externals 构造（R125/R128 统一减法——v1.9.1 批交付 + 三十一审收口定型）：
/// - 基底 `base_externals`：包路径 = 该包依赖闭包公开符号并集
///   （externals_map[P]）；根路径 = 全图全部包 public_symbols 并集。
/// - `pkg_symbols = Some(本包 public_symbols)`（缺口 A，包路径）：再并上
///   本包 public_symbols——多文件包内兄弟文件符号放行。
/// - **统一减法（R128）**：对最终 externals 向量（base ∪ pkg）整体减去
///   当前文件自身顶层符号集（file_top_level_symbols）。**减法必做**：
///   当前文件符号若留在 externals，同文件重复 fn 名会命中 typechecker
///   的 external_symbols 分支（collect_fn_sig 收敛点），NAM002 error
///   降级为 NAM006 warning（翻转同文件重复定义负例）——base 侧同样
///   要减：包内文件重复 fn 撞**依赖闭包**符号时该符号恰在 base 里。
///   单文件包减法后为空集，行为零变化；根路径（`pkg_symbols = None`，
///   缺口 C）同受统一减法覆盖（R125：root_externals 减文件自身符号）
///   ——包/根两路径行为口径对称。
///
/// designs/0029 §3.1（R234）：返回该文件的检查结论（见 FileVerdict）——
/// 调用方据此决定退出码。`report = false`（JSON 模式）静默：不打 ✓/✗
/// 行与诊断，只回传结论；源文件不可读的 stderr 说明行两种模式都打
/// （它是 rc 2 的唯一解释）。warning-only 的文件照旧打 ✗（cosmetic，
/// 另议）但结论为 NoError。
fn check_and_report_file(
    file: &std::path::Path,
    base_externals: &[String],
    pkg_symbols: Option<&HashSet<String>>,
    report: bool,
) -> FileVerdict {
    let diags = match diagnose_file(file, base_externals, pkg_symbols) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("      读取 {} 失败: {}", file.display(), e);
            return FileVerdict::Unreadable;
        }
    };
    if report {
        if diags.diagnostics.is_empty() {
            println!(
                "      ✓ {} — 通过",
                file.file_name().unwrap().to_string_lossy()
            );
        } else {
            println!(
                "      ✗ {} — {} 个诊断",
                file.file_name().unwrap().to_string_lossy(),
                diags.diagnostics.len()
            );
            print!("{}", diags.to_human());
        }
    }
    if diags.ok {
        FileVerdict::NoError
    } else {
        FileVerdict::HasError
    }
}

/// 单文件的「读取 + 解析 + 类型检查」——只产出诊断，不打印；缺口 A/C 的
/// 「本包 public_symbols − 当前文件自身符号」减法在下面的类型检查分支里做。
/// 调用方：[`check_and_report_file`]（无文件 build 的逐文件检查）与
/// [`dependency_diag_notice`]（designs/0029 §3.3，入口文件路径的依赖包可见性）。
fn diagnose_file(
    file: &std::path::Path,
    base_externals: &[String],
    pkg_symbols: Option<&HashSet<String>>,
) -> Result<diagnostics::Diagnostics, std::io::Error> {
    let src = fs::read_to_string(file)?;
    let path_str = file.display().to_string();
    let mut diags = diagnostics::Diagnostics::from_parse_result(&src, &path_str);
    if diags.ok {
        let program = parser::Parser::parse_recover(&src).program;
        let own = file_top_level_symbols(&program);
        let mut externals: Vec<String> = base_externals.to_vec();
        if let Some(symbols) = pkg_symbols {
            externals.extend(symbols.iter().cloned());
        }
        // R128（三十一审）：统一减法——对最终 externals 向量（base ∪ pkg）
        // 整体减去当前文件自身顶层符号，而非仅对 pkg 追加部分减。修复包
        // 路径 base 侧边角：包内文件重复 fn 撞依赖闭包符号时 base_externals
        // 含该符号（仅对 pkg 部分减法不去除）→ NAM002 降级为 NAM006。
        // 根路径（None 分支）同受此统一减法覆盖（R125 三十审行为维持）。
        externals.retain(|s| !own.contains(s.as_str()));
        typechecker::check_program_with_externals(
            &program, &src, &path_str, &mut diags, &externals,
        );
    }
    Ok(diags)
}

/// 缺口 A（designs/0021 §1）：单文件顶层符号集——fn 名 + enum 名 +
/// 枚举变体名（与 package.rs collect_public_symbols 同口径），供
/// 「本包 public_symbols − 当前文件符号」减法用。
fn file_top_level_symbols(program: &ast::Program) -> HashSet<String> {
    let mut syms = HashSet::new();
    for item in &program.items {
        match item {
            ast::Item::Fn(f) => {
                syms.insert(f.name.clone());
            }
            ast::Item::Enum(e) => {
                syms.insert(e.name.clone());
                for v in &e.variants {
                    syms.insert(v.name.clone());
                }
            }
            ast::Item::Import(_) => {}
        }
    }
    syms
}

/// designs/0029 §3.3（R236）：入口文件路径（`--json` / `--check` / 运行）的
/// 依赖包可见性——用无文件 build 的同一套逐文件 externals 算法静默检查各依赖
/// 包源文件，有诊断则返回 stderr 提示行（条数按 error / warning 分列）；无
/// 诊断返回 None。只提示、不合并：读者仍需在项目根目录运行 `lom build` 才能
/// 看到全文。源文件不可读计为一条错误（`lom build` 对应工具失败带 rc 2）。
/// 入口文件自己清单所在目录的其它根文件不在提示范围内（与设计的范围一致）。
fn dependency_diag_notice(graph: &package::DependencyGraph) -> Option<String> {
    if graph.packages.is_empty() {
        return None;
    }
    let externals_map = package_closure_externals(graph);
    let mut errors = 0usize;
    let mut warnings = 0usize;
    for (name, pkg) in &graph.packages {
        for file in &pkg.source_files {
            match diagnose_file(file, &externals_map[name], Some(&pkg.public_symbols)) {
                Ok(diags) => {
                    for d in &diags.diagnostics {
                        if matches!(d.severity, diagnostics::Severity::Error) {
                            errors += 1;
                        } else {
                            warnings += 1;
                        }
                    }
                }
                Err(_) => errors += 1,
            }
        }
    }
    if errors == 0 && warnings == 0 {
        return None;
    }
    Some(format!(
        "依赖包里有 {} 条诊断未在此显示（{} 错误 / {} 警告）——在项目根目录运行 lom build 查看全部。",
        errors + warnings,
        errors,
        warnings
    ))
}

/// v1.4.13 登记项 2（v1.4.9 登记）：`lom build` 无文件流程的包级
/// externals——包 P 检查源文件时的可见外部符号 = P 的
/// manifest.dependencies 出发的传递闭包（不含 P 自己）各包
/// public_symbols 并集。带文件路径（run_build_wasm）走
/// entry_externals 的全依赖图并集（合并单元整体检查），本流程
/// 逐文件检查故按各包自身闭包收敛——依赖未声明的包符号不并入
/// （负向面：仍 NAM003）。
fn package_closure_externals(graph: &package::DependencyGraph) -> HashMap<String, Vec<String>> {
    let mut memo: HashMap<String, Vec<String>> = HashMap::new();
    for name in graph.packages.keys() {
        closure_symbols(graph, name, &mut memo);
    }
    memo
}

/// closure_externals 的 DFS 主体：先占位再递归（防御性环保护——正常
/// 路径图已过 resolve_dependencies 的环检测/PKG003，为 DAG，不命中）；
/// 依赖边指向未入图的包名时按空集处理（正常解析不出现）。
fn closure_symbols(
    graph: &package::DependencyGraph,
    pkg_name: &str,
    memo: &mut HashMap<String, Vec<String>>,
) {
    if memo.contains_key(pkg_name) {
        return;
    }
    memo.insert(pkg_name.to_string(), Vec::new());
    let mut syms: Vec<String> = Vec::new();
    if let Some(pkg) = graph.packages.get(pkg_name) {
        for (dep_name, _) in &pkg.manifest.dependencies {
            if let Some(dep) = graph.packages.get(dep_name) {
                syms.extend(dep.public_symbols.iter().cloned());
            }
            closure_symbols(graph, dep_name, memo);
            syms.extend(memo[dep_name].iter().cloned());
        }
    }
    memo.insert(pkg_name.to_string(), syms);
}

/// Phase 2.7: 执行 `lom fix` 子命令
///
/// 为诊断集合生成修复计划，或应用修复到源文件：
/// - 默认 / `--plan`：生成修复计划（lom-fix/v1）
/// - `--apply`：应用高置信度修复到源文件（lom-apply/v1；M2 起迭代至收敛，上限 5 轮）
///   - `--dry-run`：只预览不写文件
///   - `--json`：输出 lom-apply/v1 schema
///
/// 流程：
///   1. 词法 + 语法诊断（Phase 2.2 容错解析器）
///   2. 若解析通过，执行类型检查（Phase 2.4），收集 TYPE/MAT/NAM/EFF 诊断
///   3. 调用 fix::generate_plan 生成修复计划
///   4. 若 --apply：调用 apply::apply_plan 应用修复，写回文件（或 --dry-run 只预览）
///   5. 输出结果
///
/// designs/0028（v1.17.0）L2 `--run`（可选 `--run-budget <ms>`、`--allow-io`）：
///   - `--plan --run`：闸门 → 执行一次程序 → 观察到的运行时诊断（L1 带位置）
///     追加进计划数组（stage "runtime"，排在静态计划之后，全是 hint）；
///     lom-fix/v1 追加可选顶层字段 `run`。
///   - `--apply --run`：静态迭代轮照旧收敛后对收敛源码做一次运行时观察，
///     **只报告不应用**；lom-apply/v1 追加 `runtime_plans` 与 `run` 字段。
///   - 不带 --run 的两条路径不经 fix_run（输出与改前逐字节一致）。
///
/// 退出码：
///   0 — 计划生成成功 / apply 应用成功（含 0 个修复应用）；--run 观察到运行时
///       错误 / 运行通过 / 超时 / 闸门拒绝均属"分析完成"，同为 0
///   2 — apply 写文件失败（β-2 起：工具自身失败带，此前 1；
///       文件读取错误在 main 入口统一 rc 2）；--run 观察环节自身失败
///       （trace 出口打不开 / 解释器线程异常终止）
fn run_fix(src: &str, path: &str, cli: &CliArgs) {
    // designs/0030 §3.2（R244）/ §3.1（R246）：fix 面与 run / --check 同源的
    // 包上下文——一次解析、逐层下传（首段诊断 / apply 迭代轮 / 终检 / --run
    // 的静态诊断）。此前这几处调的是 externals 恒空的 check_program：包项目的
    // 入口文件对包符号的引用被报成假 NAM003（fix 顺着它给出错误的替换建议、
    // `--run` 被假 error 拒跑）。无清单单文件 = 空上下文：检查器输入与此前
    // 逐字相同，输出逐字节不变。fix 面不打印 PKG007（选边 A2：stderr 维持为空）。
    let pkg = PkgCtx::resolve(path);

    // 词法 + 语法 + 类型诊断（与 --json 模式一致；R55：同时提取顶层函数摘要）
    let (diags, fns) = diagnose_with_pkg(src, path, &pkg);

    // 生成修复计划
    let plan = fix::generate_plan(&diags, src, &fns);

    // designs/0028 L2：--run 的运行时观察选项（不带 --run = None，以下两条
    // 输出路径与改前逐字节一致）。--trace + --run：观察运行期间 trace 照常
    // 产出（设计 §4.8）。
    let run_opts = cli.run.then(|| fix_run::RunOpts {
        budget_ms: cli.run_budget.unwrap_or(fix_run::DEFAULT_RUN_BUDGET_MS),
        allow_io: cli.allow_io,
        trace: cli.trace.then(|| fix_run::TraceOpts {
            out_path: cli.trace_out.clone(),
            max_events: cli
                .trace_max_events
                .unwrap_or(interpreter::TRACE_DEFAULT_MAX_EVENTS),
            builtins: cli.trace_builtins,
        }),
    });

    // Phase 3.1: --apply 模式 — 应用修复到源文件
    // 修复引擎深化 M2：从单趟升级为**迭代闭环**——应用后重新诊断再修，
    // 直到无高置信度可应用项（applied==0）或源码不再变化，上限 5 轮防震荡死循环。
    if cli.apply {
        const MAX_ROUNDS: usize = 5;
        let (current, results) = apply_iterative(src, path, MAX_ROUNDS, &pkg);

        // R55（九审）：对修复后的源码重跑完整诊断，ok 含最终状态
        let final_diag = apply::FinalDiag::check(&current, path, &pkg);

        // designs/0028 L2 --apply --run（设计 §4.8）：静态迭代轮照旧收敛后，
        // 对收敛源码做一次运行时观察——只报告不应用（运行时计划全是 hint）。
        let observation = run_opts.as_ref().map(|opts| {
            let (cur_diags, _cur_fns) = diagnose_with_pkg(&current, path, &pkg);
            let obs = fix_run::observe(&current, path, &cur_diags, opts);
            let plans = obs
                .runtime_diag
                .as_ref()
                .map(|d| fix_run::runtime_plans(d, &current, path))
                .unwrap_or_default();
            (obs, plans)
        });

        if cli.json {
            match &observation {
                Some((obs, plans)) => print!(
                    "{}",
                    apply::rounds_to_json_with(
                        &results,
                        path,
                        &final_diag,
                        &fix_run::apply_extra_json(&obs.run, plans)
                    )
                ),
                None => print!("{}", apply::rounds_to_json(&results, path, &final_diag)),
            }
        } else {
            match &observation {
                Some((obs, plans)) => print!(
                    "{}",
                    apply::rounds_to_human_with(
                        &results,
                        path,
                        &final_diag,
                        &fix_run::run_to_human(&obs.run, obs.runtime_diag.as_ref(), plans, None)
                    )
                ),
                None => print!("{}", apply::rounds_to_human(&results, path, &final_diag)),
            }
        }

        let total_applied: usize = results.iter().map(|r| r.applied).sum();

        // --dry-run 不写文件；否则把迭代收敛后的源码写回
        if !cli.dry_run && total_applied > 0 {
            if let Err(e) = fs::write(path, &current) {
                eprintln!("apply 写文件失败: {}", e);
                // β-2：IO 错误 = 工具失败带 rc 2（此前 1）
                process::exit(2);
            }

            // Phase 4.1.3: 追加修复历史记录到 .lom/fix-history.jsonl
            // 供 LLM 学习"过去修了什么"，辅助后续修复决策
            // M2：每一轮各写一条 entry（带 round 字段；旧格式无 round 读取时按 1 处理）
            let history_path = std::path::Path::new(".lom/fix-history.jsonl");
            for (i, result) in results.iter().enumerate() {
                if result.applied == 0 {
                    continue;
                }
                let changes: Vec<fix_history::HistoryChange> = result
                    .changes
                    .iter()
                    .map(|c| fix_history::HistoryChange {
                        line: c.line,
                        col: c.col,
                        action: apply::action_str(c.action).to_string(),
                        description: c.description.clone(),
                        diagnostic_code: c.diagnostic_code.clone(),
                    })
                    .collect();
                let entry = fix_history::FixHistoryEntry {
                    timestamp: fix_history::current_timestamp(),
                    file: path.to_string(),
                    applied: result.applied,
                    skipped: result.skipped,
                    changes,
                    round: i + 1,
                };
                if let Err(e) = fix_history::append_history(&entry, history_path) {
                    eprintln!("警告：修复历史记录写入失败: {}", e);
                    // 历史记录失败不阻止 apply 成功
                }
            }
        }

        // designs/0028：观察环节自身失败 = 工具失败带 rc 2（静态 apply 的
        // 结果已照常输出与落盘）；其余情形维持 rc 0
        if let Some((obs, _)) = &observation {
            exit_for_run(&obs.run);
        }
        process::exit(0);
    }

    // designs/0028 L2 --plan --run：闸门 → 执行一次 → 观察到的运行时诊断追加
    // 进计划数组（排在静态计划之后，stage "runtime"）；lom-fix/v1 追加 run
    // 字段。ok = 计划为空（运行通过且静态无诊断）。
    if let Some(opts) = &run_opts {
        let obs = fix_run::observe(src, path, &diags, opts);
        let runtime_plans = obs
            .runtime_diag
            .as_ref()
            .map(|d| fix_run::runtime_plans(d, src, path))
            .unwrap_or_default();
        let first_runtime = plan.plans.len() + 1;
        let mut plans = plan.plans;
        plans.extend(runtime_plans);
        let merged = fix::FixPlan {
            file: plan.file,
            ok: plans.is_empty(),
            plans,
        };
        if cli.json {
            let run_fields = fix_run::run_fields_json(&obs.run);
            print!("{}", fix::to_json_with(&merged, Some(&run_fields)));
        } else {
            print!("{}", fix::to_human(&merged));
            // plan 模式：运行时计划已并入上方诊断列表（首条序号 first_runtime）
            print!(
                "{}",
                fix_run::run_to_human(
                    &obs.run,
                    obs.runtime_diag.as_ref(),
                    &[],
                    Some(first_runtime)
                )
            );
        }
        exit_for_run(&obs.run);
        process::exit(0);
    }

    // 默认 / --plan 模式：输出修复计划
    if cli.json {
        print!("{}", fix::to_json(&plan));
    } else {
        print!("{}", fix::to_human(&plan));
    }

    // fix 子命令的退出码语义：计划生成成功即 0（无论是否有诊断）
    process::exit(0);
}

/// designs/0028 L2：--run 观察环节自身失败（trace 出口打不开 / 解释器线程
/// 异常终止）时的退出——stderr 说明 + 工具失败带 rc 2；分析完成则直接返回
/// （调用方按既有语义 exit 0）。
fn exit_for_run(run: &fix_run::RunReport) {
    let code = run.exit_code();
    if code != 0 {
        if let Some(d) = &run.detail {
            eprintln!("{}", d);
        }
        process::exit(code);
    }
}

/// designs/0030 §3.1 / §3.2（R244 / R246）：带文件路径的 fix / apply 面所需的
/// 包上下文——与 run / `--check` 同源（[`resolve_entry_packages`] 的结果）。
#[derive(Clone, Default)]
pub(crate) struct PkgCtx {
    /// 依赖包公开符号（入口文件单文件视图的 externals；无清单 = 空集）
    pub externals: Vec<String>,
    /// 清单 / 依赖解析失败时的那条结构化 PKG 诊断（error 级）——fix 面据此
    /// 走 PKG 的提示臂并跳过类型检查（带着空 externals 继续检查只会产出
    /// 使用点的假 NAM003）；`--run` 的闸门因静态 error 不执行程序。
    pub error: Option<diagnostics::Diagnostic>,
}

impl PkgCtx {
    /// 不打印、不退出（PKG007 的打印只在本文件带文件三路径与无文件 build
    /// 的调用点——fix / apply 面不打，stderr 维持为空）
    fn resolve(path: &str) -> PkgCtx {
        match resolve_entry_packages(path) {
            EntryPackages::Resolved(g) => PkgCtx {
                externals: entry_externals(&g),
                error: None,
            },
            EntryPackages::NoManifest => PkgCtx::default(),
            EntryPackages::Failed(err, manifest) => PkgCtx {
                externals: Vec::new(),
                error: Some(package::pkg_error_diagnostic(&err, manifest.as_deref())),
            },
        }
    }
}

/// designs/0030 §3.2（R244）：一份源码的静态诊断（词法 + 语法 + 类型）——
/// fix 首段、apply 的每一轮、apply 后的终检、`--apply --run` 的闸门判定共用。
/// 解析通过且包解析成功时做类型检查（externals 见 [`PkgCtx`]）；包解析失败
/// 时追加那条 PKG 诊断并跳过类型检查。无清单单文件 = 空上下文：与改前的
/// `check_program` 是同一函数、同一输入。
pub(crate) fn diagnose_with_pkg(
    src: &str,
    path: &str,
    pkg: &PkgCtx,
) -> (diagnostics::Diagnostics, Vec<fix::FnInfo>) {
    let mut diags = diagnostics::Diagnostics::from_parse_result(src, path);
    let mut fns: Vec<fix::FnInfo> = Vec::new();
    if diags.ok {
        if let Some(e) = &pkg.error {
            diags.diagnostics.push(e.clone());
            diags.ok = false;
        } else {
            let program = parser::Parser::parse_recover(src).program;
            fns = fix::fn_infos(&program, src);
            typechecker::check_program_with_externals(
                &program,
                src,
                path,
                &mut diags,
                &pkg.externals,
            );
        }
    }
    (diags, fns)
}

/// Phase 4.2: 启动交互式 REPL
///
/// 读入一行 → 判断完整性（多行模式累积）→ 执行 → 打印结果 → 循环
/// 特殊命令：:q/:quit/:exit 退出，:help 显示帮助，:reset 重置会话，:show 显示源码
fn run_repl() {
    use std::io::{self, BufRead, Write};

    let mut session = repl::ReplSession::new();
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    println!("Lom REPL (Phase 4.2) — 输入 :help 查看帮助，:q 退出");

    let mut buffer = String::new(); // 多行输入累积缓冲

    loop {
        // 提示符：缓冲为空时用 "lom> "，多行累积时用 "  ..> "
        let prompt = if buffer.is_empty() { "lom> " } else { "  ..> " };
        print!("{}", prompt);
        let _ = stdout.flush();

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => {
                // EOF (Ctrl+D)
                println!();
                break;
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("读取输入失败: {}", e);
                break;
            }
        }

        // 去掉行尾换行
        let line = line.trim_end_matches('\n').trim_end_matches('\r');

        // 空行处理：缓冲为空时跳过，累积中时追加（保留空行上下文）
        if line.is_empty() && buffer.is_empty() {
            continue;
        }

        // 累积到缓冲
        if !buffer.is_empty() {
            buffer.push('\n');
        }
        buffer.push_str(line);

        // 检查输入完整性
        if !repl::is_input_complete(&buffer) {
            // 不完整：继续等待下一行
            continue;
        }

        // 完整：取出缓冲执行，然后清空
        let input = std::mem::take(&mut buffer);

        match session.exec_line(&input) {
            Ok(result) => {
                if !result.output.is_empty() {
                    println!("{}", result.output);
                }
                if !result.should_continue {
                    break;
                }
            }
            Err(e) => {
                // exec_line 内部已捕获大部分错误，这里防御性处理
                eprintln!("错误: {}", e);
            }
        }
    }
}

/// Phase 4.3: 启动 LSP 服务器（stdio JSON-RPC 2.0）
///
/// 支持的 LSP 方法：
///   - initialize / initialized：握手
///   - shutdown / exit：退出
///   - textDocument/didOpen：打开文件，推送诊断
///   - textDocument/didChange：修改文件，重新解析推送诊断
///   - textDocument/hover：悬停显示类型信息
///   - textDocument/completion：代码补全
///
/// 消息格式：Content-Length: N\r\n\r\n + JSON payload
///
/// R63（十审）：Content-Length 上限 16MB——声称超限的 header 拒绝分配并
/// 断连。v1.2.2 按声称长度直接 `vec![0u8; len]`，`Content-Length: 999999999999`
/// 单 header 即 `memory allocation failed` abort（rc=0xC0000409），损坏/
/// 恶意对端可用 20 字节杀死服务器。
const MAX_LSP_CONTENT_LENGTH: usize = 16 * 1024 * 1024;

fn run_lsp() {
    use std::io::{self, BufRead, Read, Write};

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut buffer = String::new();

    // 文档状态：uri -> 源码文本
    let mut docs: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    // R72（十一审）：shutdown 是否已收到——exit 分支的退出码依据
    // （LSP 3.17：未经 shutdown 的 exit 建议以 error code 1 退出）
    let mut shutdown_seen = false;

    loop {
        // 1. 读取 Content-Length header
        let mut content_len: Option<usize> = None;
        loop {
            buffer.clear();
            match stdin.lock().read_line(&mut buffer) {
                Ok(0) => return, // EOF
                Ok(_) => {}
                Err(_) => return,
            }
            let line = buffer.trim_end();
            if line.is_empty() {
                // header 结束，空行分隔符
                break;
            }
            if let Some(pos) = line.find("Content-Length: ") {
                // R72（十一审）：重复的 Content-Length 头说明帧格式损坏——
                // 无法确定该读多少字节，流已不可信，拒绝并断连（LSP 规范
                // 建议拒收重复头）。v1.2.3 的 last-wins 会错读 payload 长度。
                if content_len.is_some() {
                    eprintln!("[lsp] 重复的 Content-Length 头——帧格式损坏，拒绝并断连");
                    process::exit(1);
                }
                let len_str = &line[pos + 16..];
                if let Ok(n) = len_str.trim().parse::<usize>() {
                    content_len = Some(n);
                }
            }
        }

        let len = match content_len {
            Some(n) if n <= MAX_LSP_CONTENT_LENGTH => n,
            Some(n) => {
                // 拒绝分配即断连：payload 未读，流已不可信，无法继续会话
                eprintln!(
                    "[lsp] Content-Length {} 超过上限 {} 字节——拒绝分配并断连",
                    n, MAX_LSP_CONTENT_LENGTH
                );
                process::exit(1);
            }
            None => continue,
        };

        // 2. 读取 JSON payload
        let mut payload = vec![0u8; len];
        if stdin.lock().read_exact(&mut payload).is_err() {
            return;
        }
        let json = match String::from_utf8(payload) {
            Ok(s) => s,
            Err(_) => {
                // R71（十一审）：非 UTF-8 payload 回 -32700（id:null 规范形态）
                // 后继续服务——v1.2.3 静默 continue 丢弃，客户端无从得知。
                // 与 R63 的畸形 JSON 同款处理路径。
                let resp = lsp::make_null_id_error_response(-32700, "Parse error");
                let msg = lsp::make_lsp_message(&resp);
                let _ = stdout.write_all(msg.as_bytes());
                let _ = stdout.flush();
                continue;
            }
        };

        // 3. 解析 JSON-RPC 消息
        // R63：解析失败回 JSON-RPC 错误响应（-32700/-32600，id 为 null——
        // 规范形态）。v1.2.2 静默 continue，客户端无从得知请求未被执行。
        let (id, method, params) = match lsp::parse_rpc_message(&json) {
            Ok(m) => m,
            Err(e) => {
                let resp = lsp::make_null_id_error_response(e.json_rpc_code(), e.message());
                let msg = lsp::make_lsp_message(&resp);
                let _ = stdout.write_all(msg.as_bytes());
                let _ = stdout.flush();
                continue;
            }
        };

        // 4. 处理请求
        let response = handle_lsp_method(&method, id, &params, &mut docs, &mut shutdown_seen);

        // 5. 输出响应（如果有 id 才发响应，通知无响应）
        if let Some(resp) = response {
            let msg = lsp::make_lsp_message(&resp);
            let _ = stdout.write_all(msg.as_bytes());
            let _ = stdout.flush();
        }
    }
}

/// 处理单个 LSP 方法，返回可选的响应 JSON
fn handle_lsp_method(
    method: &str,
    id: Option<u64>,
    params: &str,
    docs: &mut std::collections::HashMap<String, String>,
    shutdown_seen: &mut bool,
) -> Option<String> {
    match method {
        "initialize" => {
            let id = id?;
            // 返回服务器能力声明
            let result = "{\"capabilities\":{\
                \"textDocumentSync\":1,\
                \"hoverProvider\":true,\
                \"completionProvider\":{\"triggerCharacters\":[\".\",\" \"]},\
                \"definitionProvider\":false\
            }}";
            Some(lsp::make_response(id, result))
        }
        "initialized" => {
            // 通知，无响应
            None
        }
        "shutdown" => {
            let id = id?;
            *shutdown_seen = true;
            Some(lsp::make_response(id, "null"))
        }
        "exit" => {
            // R72（十一审）：LSP 3.17 建议——未经 shutdown 的 exit 以
            // error code 1 退出（v1.2.3 恒 0）
            process::exit(if *shutdown_seen { 0 } else { 1 });
        }
        "textDocument/didOpen" => {
            // 提取 uri 和 text
            if let Some((uri, text)) = extract_did_open_params(params) {
                docs.insert(uri.clone(), text.clone());
                // 推送诊断
                let diags = lsp::compute_diagnostics(&text, &uri);
                Some(lsp::make_publish_diagnostics(&uri, &diags))
            } else {
                None
            }
        }
        "textDocument/didChange" => {
            // 提取 uri 和新文本（简化：取全量变更）
            if let Some((uri, text)) = extract_did_change_params(params) {
                docs.insert(uri.clone(), text.clone());
                let diags = lsp::compute_diagnostics(&text, &uri);
                Some(lsp::make_publish_diagnostics(&uri, &diags))
            } else {
                None
            }
        }
        "textDocument/hover" => {
            let id = id?;
            if let Some((uri, line, col)) = extract_hover_params(params)
                && let Some(src) = docs.get(&uri)
                && let Some(hover) = lsp::handle_hover(src, line, col)
            {
                // R58：转义统一走 escape_str（含反斜杠/控制字符）
                let result = format!(
                    "{{\"contents\":{{\"kind\":\"markdown\",\"value\":\"{}\"}}}}",
                    crate::json::escape_str(&hover.content)
                );
                return Some(lsp::make_response(id, &result));
            }
            // 无 hover 结果
            Some(lsp::make_response(id, "null"))
        }
        "textDocument/completion" => {
            let id = id?;
            // 补全不需要位置（简化：返回全局可见符号）
            let uri = extract_completion_uri(params).unwrap_or_default();
            let src = docs.get(&uri).map(|s| s.as_str()).unwrap_or("");
            let items = lsp::handle_completion(src);
            let items_json: Vec<String> = items
                .iter()
                .map(|item| {
                    let detail = item
                        .detail
                        .as_ref()
                        .map(|d| format!(",\"detail\":\"{}\"", crate::json::escape_str(d)))
                        .unwrap_or_default();
                    format!(
                        "{{\"label\":\"{}\",\"kind\":{}{}}}",
                        crate::json::escape_str(&item.label),
                        item.kind.as_lsp_number(),
                        detail
                    )
                })
                .collect();
            let result = format!("[{}]", items_json.join(","));
            Some(lsp::make_response(id, &result))
        }
        _ => {
            // 未知方法：返回 method not found 错误
            let id = id?;
            Some(lsp::make_error_response(
                id,
                -32601,
                &format!("方法未实现: {}", method),
            ))
        }
    }
}

// ===== 单元测试（修复引擎深化 M2：迭代闭环）=====
// R56 进程级回归（spawn 二进制断言退出码/stderr）在 tests/r56_process.rs
// 集成测试目录——CARGO_BIN_EXE_lom 仅在集成测试中由 cargo 注入。

#[cfg(test)]
mod tests {
    use super::*;

    /// 第四轮评审整改：eval 任务 ID 全局唯一性 + manifest 计数一致性回归
    ///
    /// 背景：09_modules 的 map 任务与 10_error_repair 首题曾同号 086
    /// （runner 按 `<id>.lom` 寻址候选文件，冲突会互相覆盖）。
    /// 该冲突存在了 12 天无人发现——必须有测试钉住。
    #[test]
    fn eval_task_ids_globally_unique() {
        let eval_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("eval");
        let mut ids: Vec<String> = Vec::new();
        let mut total = 0usize;
        let mut entries: Vec<_> = std::fs::read_dir(eval_dir.join("tasks"))
            .expect("eval/tasks 目录不存在")
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "json"))
            .collect();
        entries.sort();
        for path in entries {
            let src = std::fs::read_to_string(&path).unwrap();
            let v = crate::json::parse(&src).expect("任务 JSON 解析失败");
            let crate::interpreter::Value::List(items) = v else {
                panic!("{:?} 顶层不是数组", path)
            };
            for item in items.iter() {
                let crate::interpreter::Value::Record { fields } = item else {
                    panic!("{:?} 任务不是对象", path)
                };
                let id = fields
                    .iter()
                    .find(|(k, _)| &**k == "id")
                    .and_then(|(_, v)| match v {
                        crate::interpreter::Value::Str(s) => Some(s.to_string()),
                        _ => None,
                    })
                    .expect("任务缺 id 字段");
                ids.push(id);
                total += 1;
            }
        }
        let mut unique = ids.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(ids.len(), unique.len(), "eval 任务 ID 存在重复");
        // manifest 计数一致性
        let manifest_src = std::fs::read_to_string(eval_dir.join("manifest.json")).unwrap();
        let mv = crate::json::parse(&manifest_src).unwrap();
        let crate::interpreter::Value::Record { fields } = &mv else {
            panic!("manifest 顶层不是对象")
        };
        let declared = fields
            .iter()
            .find(|(k, _)| &**k == "total_tasks")
            .and_then(|(_, v)| match v {
                crate::interpreter::Value::Int(n) => Some(*n as usize),
                _ => None,
            })
            .expect("manifest 缺 total_tasks");
        assert_eq!(total, declared, "任务总数与 manifest.total_tasks 不一致");
    }
}
