// Lom CLI 入口 — Phase 2.7
// 用法:
//   lom <file.lom>                运行 .lom 程序（默认）
//   lom <file.lom> --json         仅诊断，输出 JSON（不执行）
//   lom <file.lom> --check        仅诊断，输出人类可读（不执行）
//   lom info <file.lom> [--json]  导出类型信息（Phase 2.6）
//   lom fix <file.lom> [--plan] [--json]  生成修复计划（Phase 2.7）
//   lom --json <file.lom>         等价（选项可前可后）
//   lom --help | -h               帮助

use std::env;
use std::fs;
use std::process;

mod ast;
mod diagnostics;
mod fix;
mod info;
mod interpreter;
mod lexer;
mod parser;
mod typechecker;

#[derive(Debug, Default)]
struct CliArgs {
    /// 子命令：info（Phase 2.6）/ fix（Phase 2.7）；None 表示默认运行/检查模式
    subcommand: Option<String>,
    file: Option<String>,
    json: bool,
    check: bool,
    help: bool,
    /// Phase 2.7: --plan 标志（lom fix 专用，表示仅生成计划不应用；当前 --plan 是默认行为，标志仅作显式标记）
    plan: bool,
}

fn print_help(prog: &str) {
    eprintln!("Lom 解释器 (Phase 2.7) — AI 原生编程语言");
    eprintln!();
    eprintln!("用法:");
    eprintln!("  {prog} <file.lom>                运行 .lom 程序（默认）");
    eprintln!("  {prog} <file.lom> --json         仅诊断，输出结构化 JSON（不执行）");
    eprintln!("  {prog} <file.lom> --check        仅诊断，输出人类可读格式（不执行）");
    eprintln!("  {prog} info <file.lom> [--json]  导出类型信息（函数/枚举/导入签名）");
    eprintln!("  {prog} fix <file.lom> [--plan] [--json]  生成 AI 修复计划（lom-fix/v1）");
    eprintln!("  {prog} --help | -h               显示帮助");
    eprintln!();
    eprintln!("子命令:");
    eprintln!("  info        导出类型信息（Phase 2.6）。默认人类可读；--json 输出 lom-info/v1 schema");
    eprintln!("  fix         生成修复计划（Phase 2.7）。默认人类可读；--json 输出 lom-fix/v1 schema");
    eprintln!("              --plan：仅生成计划不应用（当前为默认行为，--apply 留待 Phase 3）");
    eprintln!();
    eprintln!("选项:");
    eprintln!("  --json     结构化 JSON 输出（诊断用 lom-diag/v1；info 用 lom-info/v1；fix 用 lom-fix/v1），便于 LLM 消费");
    eprintln!("  --check    仅做词法/语法/类型检查，不执行；输出带源码上下文的人类可读诊断");
    eprintln!("  --plan     lom fix 子命令专用：仅生成修复计划（当前为默认）");
    eprintln!("  --help, -h 显示本帮助");
    eprintln!();
    eprintln!("退出码:");
    eprintln!("  0  程序成功执行 / 诊断无错误 / info 导出成功 / fix 计划生成成功");
    eprintln!("  1  读取/词法/语法/运行时错误");
}

fn parse_args(args: &[String]) -> CliArgs {
    let mut out = CliArgs::default();
    let mut iter = args.iter().skip(1).peekable();

    // 检查第一个参数是否是子命令
    if let Some(first) = iter.peek() {
        match first.as_str() {
            "info" => {
                out.subcommand = Some("info".to_string());
                iter.next();
            }
            "fix" => {
                out.subcommand = Some("fix".to_string());
                iter.next();
            }
            _ => {}
        }
    }

    for a in iter {
        match a.as_str() {
            "--json" => out.json = true,
            "--check" => out.check = true,
            "--plan" => out.plan = true,
            "--help" | "-h" => out.help = true,
            _ => {
                if a.starts_with('-') {
                    eprintln!("未知选项: {}", a);
                    eprintln!("使用 --help 查看用法");
                    process::exit(1);
                }
                if out.file.is_some() {
                    eprintln!("只能指定一个文件，但收到多个: {} {}", out.file.as_ref().unwrap(), a);
                    eprintln!("使用 --help 查看用法");
                    process::exit(1);
                }
                out.file = Some(a.clone());
            }
        }
    }
    out
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let cli = parse_args(&args);

    if cli.help {
        print_help(&args[0]);
        return;
    }

    let path = match &cli.file {
        Some(p) => p,
        None => {
            print_help(&args[0]);
            process::exit(1);
        }
    };

    let src = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            // 文件读取错误也输出 JSON 诊断（如果 --json），保持输出一致性
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
            process::exit(1);
        }
    };

    // ===== 子命令：info（Phase 2.6 类型信息导出）=====
    if cli.subcommand.as_deref() == Some("info") {
        run_info(&src, path, cli.json);
        return;
    }

    // ===== 子命令：fix（Phase 2.7 AI 修复计划）=====
    if cli.subcommand.as_deref() == Some("fix") {
        run_fix(&src, path, cli.json);
        return;
    }

    // ===== 诊断阶段（词法 + 语法，Phase 2.2 容错解析器一次性收集全部错误）=====
    let mut diags = diagnostics::Diagnostics::from_parse_result(&src, path);

    if cli.json {
        // JSON 模式：仅诊断，不执行；解析通过后执行类型检查（Phase 2.4）
        if diags.ok {
            let program = parser::Parser::parse_recover(&src).program;
            typechecker::check_program(&program, &src, path, &mut diags);
        }
        print!("{}", diags.to_json());
        process::exit(if diags.ok { 0 } else { 1 });
    }

    if !diags.ok {
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
    // --check 模式：执行类型检查，收集 TYPE/MAT/NAM 诊断
    // 默认运行模式：跳过类型检查（渐进式：动态可跑）
    if cli.check {
        let program = parser::Parser::parse_recover(&src).program;
        typechecker::check_program(&program, &src, path, &mut diags);
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

    // ===== 执行阶段 =====
    let program = parser::Parser::parse_recover(&src).program;
    let mut interp = interpreter::Interpreter::new();
    if let Err(e) = interp.run(&program) {
        // 运行时错误：构造诊断输出
        // 注：Phase 2.3 阶段 AST 节点尚未携带位置信息（Phase 3 改造），
        // 因此运行时错误暂以 (0, 0) 报告位置；消息本身已包含足够上下文。
        diags.add_runtime(&e, &src, 0, 0);
        eprint!("{}", diags.to_human());
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
        let mut diags = diagnostics::Diagnostics::from_parse_result(src, path);
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

/// Phase 2.7: 执行 `lom fix` 子命令
///
/// 为诊断集合生成修复计划：
/// - `--json`：输出 lom-fix/v1 schema（给 LLM 消费）
/// - 默认：人类可读格式（终端浏览）
///
/// 流程：
///   1. 词法 + 语法诊断（Phase 2.2 容错解析器）
///   2. 若解析通过，执行类型检查（Phase 2.4），收集 TYPE/MAT/NAM/EFF 诊断
///   3. 调用 fix::generate_plan 生成修复计划
///   4. 输出计划（不应用修复 — --apply 留待 Phase 3 有 span 后）
///
/// 退出码：
///   0 — 计划生成成功（无论是否有诊断/修复）
///   1 — 文件读取错误（已在 main 上层处理）
fn run_fix(src: &str, path: &str, json: bool) {
    // 收集全部诊断：词法 + 语法
    let mut diags = diagnostics::Diagnostics::from_parse_result(src, path);

    // 解析通过后追加类型检查诊断（与 --json 模式一致）
    if diags.ok {
        let program = parser::Parser::parse_recover(src).program;
        typechecker::check_program(&program, src, path, &mut diags);
    }

    // 生成修复计划
    let plan = fix::generate_plan(&diags, src);

    if json {
        print!("{}", fix::to_json(&plan));
    } else {
        print!("{}", fix::to_human(&plan));
    }

    // fix 子命令的退出码语义：计划生成成功即 0（无论是否有诊断）
    // 这样 LLM 可以无障碍消费 JSON 输出，无需区分"有诊断/无诊断"
    process::exit(0);
}
