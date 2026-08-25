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

use std::env;
use std::fs;
use std::process;

mod apply;
mod ast;
mod diagnostics;
mod doc;
mod fix;
mod fmt;
mod fix_history;
mod info;
mod interpreter;
mod json;
mod lexer;
mod lsp;
mod package;
mod parser;
mod repl;
mod typechecker;
mod wasm;
mod wasm_codegen;

#[derive(Debug, Default)]
struct CliArgs {
    /// 子命令：info（Phase 2.6）/ fix（Phase 2.7）；None 表示默认运行/检查模式
    subcommand: Option<String>,
    file: Option<String>,
    json: bool,
    check: bool,
    help: bool,
    /// Phase 6.1: --version 标志（打印编译器版本，来自 Cargo.toml）
    version: bool,
    /// Phase 2.7: --plan 标志（lom fix 专用，表示仅生成计划不应用；当前 --plan 是默认行为，标志仅作显式标记）
    plan: bool,
    /// Phase 3.1: --apply 标志（lom fix 专用，应用修复到源文件）
    apply: bool,
    /// Phase 3.1: --dry-run 标志（与 --apply 配合，只输出预览不写文件）
    dry_run: bool,
    /// Phase 4.1.3: --history 标志（lom fix 专用，查看修复历史记录）
    history: bool,
    /// Phase 7.2: --target <t>（lom build <file> --target wasm 编译到 WASM）
    target: Option<String>,
    /// Phase 7.2: -o/--output <path>（编译产物输出路径，默认 <file> 换 .wasm 后缀）
    output: Option<String>,
    /// Phase 3.5: -- 之后的参数，传递给 Lom 程序（通过 env::args() 读取）
    program_args: Vec<String>,
}

fn print_help(prog: &str) {
    eprintln!("Lom 解释器 v{} — AI 原生编程语言", env!("CARGO_PKG_VERSION"));
    eprintln!();
    eprintln!("用法:");
    eprintln!("  {prog} <file.lom>                运行 .lom 程序（默认）");
    eprintln!("  {prog} <file.lom> -- <args...>   运行 .lom 程序，传递参数（通过 env::args() 读取）");
    eprintln!("  {prog} <file.lom> --json         仅诊断，输出结构化 JSON（不执行）");
    eprintln!("  {prog} <file.lom> --check        仅诊断，输出人类可读格式（不执行）");
    eprintln!("  {prog} info <file.lom> [--json]  导出类型信息（函数/枚举/导入签名）");
    eprintln!("  {prog} doc <file.lom> [--json]   生成 API 文档（Markdown 或 lom-doc/v1）");
    eprintln!("  {prog} fmt <file.lom> [--apply|--check]  格式化源码（默认预览到 stdout）");
    eprintln!("  {prog} fix <file.lom> [--plan] [--json]  生成 AI 修复计划（lom-fix/v1）");
    eprintln!("  {prog} fix <file.lom> --apply [--dry-run] [--json]  应用修复到源文件");
    eprintln!("  {prog} fix --history [--json]    查看修复历史记录");
    eprintln!("  {prog} repl                       启动交互式 REPL（Phase 4.2）");
    eprintln!("  {prog} lsp                        启动 LSP 服务器（Phase 4.3，stdio JSON-RPC）");
    eprintln!("  {prog} build [--json]             解析 lom.toml 依赖并对包源码类型检查（Phase 4.4）");
    eprintln!("  {prog} build <file> --target wasm [-o out.wasm]  编译为 WASM 二进制（Phase 7.2）");
    eprintln!("  {prog} --help | -h               显示帮助");
    eprintln!("  {prog} --version | -V            显示版本");
    eprintln!();
    eprintln!("子命令:");
    eprintln!("  info        导出类型信息（Phase 2.6）。默认人类可读；--json 输出 lom-info/v1 schema");
    eprintln!("  doc         生成 API 文档（Phase 6.4）。默认 Markdown；--json 输出 lom-doc/v1 schema。文档注释 = 签名上方连续的 # 行");
    eprintln!("  fmt         格式化源码（Phase 6.5）。token 流驱动，注释/字符串内容保留，只规范化缩进（4 空格/层）");
    eprintln!("              默认预览到 stdout；--apply 就地改写；--check 用于 CI 门禁");
    eprintln!("  fix         生成/应用修复计划（Phase 2.7/3.1）。默认人类可读；--json 输出 lom-fix/v1 或 lom-apply/v1 schema");
    eprintln!("              --plan：仅生成计划不应用（默认行为）");
    eprintln!("              --apply：应用高置信度修复到源文件（Phase 3.1；M2 起迭代至收敛，上限 5 轮）");
    eprintln!("              --dry-run：与 --apply 配合，只输出预览不写文件");
    eprintln!("              --history：查看修复历史记录（Phase 4.1.3，存储于 .lom/fix-history.jsonl）");
    eprintln!("  repl        启动交互式 REPL（Phase 4.2）。支持多行输入、上下文保持、:help/:reset/:q 命令");
    eprintln!("  lsp         启动 LSP 服务器（Phase 4.3）。stdio JSON-RPC 2.0，支持 hover/completion/diagnostics");
    eprintln!("  build       解析 lom.toml 并对依赖包源码做类型检查（Phase 4.4）。--json 输出结构化结果");
    eprintln!();
    eprintln!("选项:");
    eprintln!("  --          参数分隔符：之后的所有参数传递给 Lom 程序（通过 env::args() 读取，Phase 3.5）");
    eprintln!("  --json     结构化 JSON 输出（诊断用 lom-diag/v1；info 用 lom-info/v1；fix 用 lom-fix/v1；apply 用 lom-apply/v1），便于 LLM 消费");
    eprintln!("  --check    仅做词法/语法/类型检查，不执行；输出带源码上下文的人类可读诊断");
    eprintln!("  --plan     lom fix 子命令专用：仅生成修复计划（默认）");
    eprintln!("  --apply    lom fix 子命令专用：应用修复到源文件（Phase 3.1；M2 起迭代至收敛）");
    eprintln!("  --dry-run  lom fix --apply 子命令专用：只预览不写文件");
    eprintln!("  --history  lom fix 子命令专用：查看修复历史记录（Phase 4.1.3）");
    eprintln!("  --help, -h 显示本帮助");
    eprintln!("  --version, -V 显示版本号（Phase 6.1）");
    eprintln!();
    eprintln!("退出码:");
    eprintln!("  0  程序成功执行 / 诊断无错误 / info 导出成功 / fix 计划生成成功 / apply 应用成功");
    eprintln!("  1  读取/词法/语法/运行时错误 / apply 应用失败");
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
            "doc" => {
                out.subcommand = Some("doc".to_string());
                iter.next();
            }
            "fmt" => {
                out.subcommand = Some("fmt".to_string());
                iter.next();
            }
            "fix" => {
                out.subcommand = Some("fix".to_string());
                iter.next();
            }
            "repl" => {
                out.subcommand = Some("repl".to_string());
                iter.next();
            }
            "lsp" => {
                out.subcommand = Some("lsp".to_string());
                iter.next();
            }
            "build" => {
                out.subcommand = Some("build".to_string());
                iter.next();
            }
            _ => {}
        }
    }

    while let Some(a) = iter.next() {
        // Phase 3.5: 遇到 `--` 后，剩余所有参数传递给 Lom 程序
        if a == "--" {
            out.program_args = iter.by_ref().cloned().collect();
            break;
        }
        match a.as_str() {
            "--json" => out.json = true,
            "--check" => out.check = true,
            "--plan" => out.plan = true,
            "--apply" => out.apply = true,
            "--dry-run" => out.dry_run = true,
            "--history" => out.history = true,
            "--help" | "-h" => out.help = true,
            "--version" | "-V" => out.version = true,
            // Phase 7.2: 带值选项
            "--target" => {
                out.target = Some(match iter.next() {
                    Some(v) => v.clone(),
                    None => {
                        eprintln!("--target 需要一个值（如 --target wasm）");
                        process::exit(1);
                    }
                });
            }
            "-o" | "--output" => {
                out.output = Some(match iter.next() {
                    Some(v) => v.clone(),
                    None => {
                        eprintln!("-o/--output 需要一个路径值");
                        process::exit(1);
                    }
                });
            }
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
    // Phase 5.0: 用大栈线程运行解释器，缓解树遍历解释器递归深度限制
    // （自举验证发现：非尾递归的 Lom 程序在长输入时栈溢出 Rust 默认 1MB 栈）
    let child = std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024) // 256MB
        .spawn(main_inner)
        .expect("failed to spawn interpreter thread");
    // process::exit 在线程内会直接终止进程，join 仅作同步点
    let _ = child.join();
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
                process::exit(1);
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

    // ===== 诊断阶段（词法 + 语法，Phase 2.2 容错解析器一次性收集全部错误）=====
    let mut diags = diagnostics::Diagnostics::from_parse_result(&src, path);

    if cli.json {
        // JSON 模式：仅诊断，不执行；解析通过后执行类型检查（Phase 2.4）
        if diags.ok {
            let program = parser::Parser::parse_recover(&src).program;
            let externals = collect_package_symbols(path);
            typechecker::check_program_with_externals(&program, &src, path, &mut diags, &externals);
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
    if cli.check {
        let program = parser::Parser::parse_recover(&src).program;
        let externals = collect_package_symbols(path);
        typechecker::check_program_with_externals(&program, &src, path, &mut diags, &externals);
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
    {
        let program = parser::Parser::parse_recover(&src).program;
        let externals = collect_package_symbols(path);
        typechecker::check_program_with_externals(&program, &src, path, &mut diags, &externals);
        if !diags.diagnostics.is_empty() {
            eprint!("{}", diags.to_human());
        }
    }

    // ===== 执行阶段 =====
    let program = parser::Parser::parse_recover(&src).program;
    let mut interp = interpreter::Interpreter::new();
    // Phase 4.4: 若文件所在目录有 lom.toml，加载依赖包
    // 使 `from pkg import { ... }` 能解析外部包符号
    let file_dir = std::path::Path::new(path)
        .parent()
        .unwrap_or(std::path::Path::new("."));
    let toml_path = file_dir.join("lom.toml");
    if toml_path.exists() {
        match package::load_manifest_file(&toml_path) {
            Ok(manifest) => {
                match package::resolve_dependencies(&manifest, file_dir) {
                    Ok(graph) => interp.load_packages(&graph),
                    Err(e) => {
                        eprintln!("依赖解析失败: {}", e);
                        process::exit(1);
                    }
                }
            }
            Err(e) => {
                eprintln!("清单解析失败: {}", e);
                process::exit(1);
            }
        }
    }
    // Phase 3.5: 传递程序参数（-- 之后的参数），供 env::args() 读取
    // 第一个参数约定为 .lom 文件路径（与大多数 CLI 约定一致：argv[0] = 程序名）
    let mut full_args = vec![path.to_string()];
    full_args.extend(cli.program_args.iter().cloned());
    interp.set_program_args(full_args);
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

/// Phase 6.7 评审整改：收集 lom.toml 依赖包的公开符号（fn/enum/变体名）
/// 供 typechecker 免于对包符号误报 NAM003（此前 pkg_demo 在默认运行路径喷 5 条假 error）
fn collect_package_symbols(path: &str) -> Vec<String> {
    let file_dir = std::path::Path::new(path)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let toml_path = file_dir.join("lom.toml");
    if !toml_path.exists() {
        return Vec::new();
    }
    match package::load_manifest_file(&toml_path) {
        Ok(manifest) => match package::resolve_dependencies(&manifest, &file_dir) {
            Ok(graph) => graph
                .packages
                .values()
                .flat_map(|p| p.public_symbols.iter().cloned())
                .collect(),
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    }
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
                            process::exit(1);
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

/// Phase 4.4: 执行 `lom build` 子命令
///
/// 读取当前目录的 lom.toml，解析依赖图，对每个依赖包的源码文件执行
/// 词法+语法+类型检查，输出诊断结果。
///
/// 流程：
///   1. 加载 ./lom.toml
///   2. resolve_dependencies 解析依赖图（含循环检测）
///   3. 对每个包的每个 .lom 文件执行 parse + typecheck
///   4. 汇总诊断输出
///
/// 退出码：
///   0 — 清单解析成功且所有包源码无错误
///   1 — 清单解析失败 / 依赖解析失败 / 包源码有错误
/// Phase 7.8: 包合并（WASM 编译用）。当前目录有 lom.toml 时解析依赖图，
/// 把每个依赖包的源码 item 合并到主程序前面（重名后主文件覆盖，对齐解释器语义）。
/// 无 lom.toml 时原样返回。
fn merge_packages_for_wasm(mut program: ast::Program) -> (ast::Program, Vec<String>) {
    let toml_path = std::path::Path::new("lom.toml");
    if !toml_path.exists() {
        return (program, Vec::new());
    }
    let manifest = match package::load_manifest_file(toml_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("lom.toml 解析失败: {}", e);
            process::exit(1);
        }
    };
    let graph = match package::resolve_dependencies(&manifest, std::path::Path::new(".")) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("依赖解析失败: {}", e);
            process::exit(1);
        }
    };
    let mut dep_items = Vec::new();
    // 依赖顺序排序保确定性（HashMap 遍历序不稳定）
    let mut pkgs: Vec<_> = graph.packages.values().collect();
    pkgs.sort_by(|a, b| a.root.cmp(&b.root));
    for pkg in pkgs {
        for file in &pkg.source_files {
            match fs::read_to_string(file) {
                Ok(src) => {
                    let items = parser::Parser::parse_recover(&src).program.items;
                    dep_items.extend(items);
                }
                Err(e) => {
                    eprintln!("无法读取包源码 '{}': {}", file.display(), e);
                    process::exit(1);
                }
            }
        }
    }
    dep_items.append(&mut program.items);
    let names = graph.packages.keys().cloned().collect();
    (ast::Program { items: dep_items }, names)
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
            process::exit(1);
        }
        None => {
            eprintln!("lom build <file> 需要 --target wasm（不带文件的 lom build 是包管理流程）");
            process::exit(1);
        }
    }
    // 2. 读源文件
    let src = match fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("无法读取文件 '{}': {}", file, e);
            process::exit(1);
        }
    };
    // 3. 解析（容错模式收集全部诊断；有 error 则不编译）
    let diags = diagnostics::Diagnostics::from_parse_result(&src, file);
    if !diags.ok {
        eprint!("{}", diags.to_human());
        process::exit(1);
    }
    let program = parser::Parser::parse_recover(&src).program;
    // 7.8 包链接：当前目录有 lom.toml 时，把依赖包源码合并进编译单元
    // （包内 item 在前，主文件在后；重名函数后主文件覆盖——对齐解释器 load_packages 语义）
    let (program, pkg_names) = merge_packages_for_wasm(program);
    // 7.9：类型检查可见性对齐——编译前跑检查器，诊断走 stderr，不拦截编译（渐进式承诺与解释器一致）
    {
        let mut tdiags = diagnostics::Diagnostics::new(file);
        typechecker::check_program(&program, &src, file, &mut tdiags);
        if !tdiags.ok {
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
            process::exit(1);
        }
    }
}

fn run_build(json: bool) {
    let toml_path = std::path::Path::new("lom.toml");
    if !toml_path.exists() {
        if json {
            println!(r#"{{"schema":"lom-build/v1","ok":false,"error":"当前目录无 lom.toml"}}"#);
        } else {
            eprintln!("错误：当前目录无 lom.toml（Phase 4.4 包管理清单）");
            eprintln!("创建最小清单：");
            eprintln!("  name = \"myapp\"");
            eprintln!("  version = \"0.1.0\"");
            eprintln!();
            eprintln!("  [dependencies]");
            eprintln!("  lib = {{ path = \"../lib\" }}");
        }
        process::exit(1);
    }

    let manifest = match package::load_manifest_file(toml_path) {
        Ok(m) => m,
        Err(e) => {
            if json {
                println!(r#"{{"schema":"lom-build/v1","ok":false,"error":"{}"}}"#, e);
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
                println!(r#"{{"schema":"lom-build/v1","ok":false,"error":"{}"}}"#, e);
            } else {
                eprintln!("依赖解析失败: {}", e);
            }
            process::exit(1);
        }
    };

    if json {
        // JSON 输出：汇总每个包的源码检查结果
        print!("{{\"schema\":\"lom-build/v1\",\"ok\":true,\"package\":\"{}\",\"version\":\"{}\",\"dependencies\":[", manifest.name, manifest.version);
        let mut first = true;
        for (name, pkg) in &graph.packages {
            if !first { print!(","); }
            first = false;
            print!("{{\"name\":\"{}\",\"path\":\"{}\",\"symbols\":[", name, pkg.root.display());
            let mut sym_first = true;
            for sym in &pkg.public_symbols {
                if !sym_first { print!(","); }
                sym_first = false;
                print!("\"{}\"", sym);
            }
            print!("]}}");
        }
        println!("]}}");
        return;
    }

    // 人类可读输出
    println!("Lom build — 包依赖解析结果");
    println!("  项目: {} v{}", manifest.name, manifest.version);
    if graph.packages.is_empty() {
        println!("  依赖: （无）");
    } else {
        println!("  依赖: {} 个", graph.packages.len());
        for (name, pkg) in &graph.packages {
            println!("    - {} (path: {})", name, pkg.root.display());
            println!("      源码文件: {} 个", pkg.source_files.len());
            println!("      公开符号: {} 个", pkg.public_symbols.len());
            // 对包源码执行类型检查
            for file in &pkg.source_files {
                let src = match fs::read_to_string(file) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("      读取 {} 失败: {}", file.display(), e);
                        continue;
                    }
                };
                let path_str = file.display().to_string();
                let mut diags = diagnostics::Diagnostics::from_parse_result(&src, &path_str);
                if diags.ok {
                    let program = parser::Parser::parse_recover(&src).program;
                    typechecker::check_program(&program, &src, &path_str, &mut diags);
                }
                if diags.diagnostics.is_empty() {
                    println!("      ✓ {} — 通过", file.file_name().unwrap().to_string_lossy());
                } else {
                    println!("      ✗ {} — {} 个诊断", file.file_name().unwrap().to_string_lossy(), diags.diagnostics.len());
                    print!("{}", diags.to_human());
                }
            }
        }
    }
    println!("\n依赖解析成功，共 {} 个包。", graph.packages.len());
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
/// 退出码：
///   0 — 计划生成成功 / apply 应用成功（含 0 个修复应用）
///   1 — 文件读取错误 / apply 写文件失败
fn run_fix(src: &str, path: &str, cli: &CliArgs) {
    // 收集全部诊断：词法 + 语法
    let mut diags = diagnostics::Diagnostics::from_parse_result(src, path);

    // 解析通过后追加类型检查诊断（与 --json 模式一致）
    if diags.ok {
        let program = parser::Parser::parse_recover(src).program;
        typechecker::check_program(&program, src, path, &mut diags);
    }

    // 生成修复计划
    let plan = fix::generate_plan(&diags, src);

    // Phase 3.1: --apply 模式 — 应用修复到源文件
    // 修复引擎深化 M2：从单趟升级为**迭代闭环**——应用后重新诊断再修，
    // 直到无高置信度可应用项（applied==0）或源码不再变化，上限 5 轮防震荡死循环。
    if cli.apply {
        const MAX_ROUNDS: usize = 5;
        let (current, results) = apply_iterative(src, path, MAX_ROUNDS);

        if cli.json {
            print!("{}", apply::rounds_to_json(&results, path));
        } else {
            print!("{}", apply::rounds_to_human(&results, path));
        }

        let total_applied: usize = results.iter().map(|r| r.applied).sum();

        // --dry-run 不写文件；否则把迭代收敛后的源码写回
        if !cli.dry_run && total_applied > 0 {
            if let Err(e) = fs::write(path, &current) {
                eprintln!("apply 写文件失败: {}", e);
                process::exit(1);
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

/// 修复引擎深化 M2：迭代应用修复直到收敛
///
/// 每轮：重新诊断（词法+语法+类型）→ 生成计划 → 应用高置信度修复。
/// 收敛条件：一轮 applied==0（无可自动修复项）或修补后源码不再变化；
/// `max_rounds` 是震荡死循环的最后防线（修复 A 引入诊断 B、修 B 又引入 A 的场景）。
/// 返回 (最终源码, 各轮结果)。
fn apply_iterative(src: &str, path: &str, max_rounds: usize) -> (String, Vec<apply::ApplyResult>) {
    let mut current = src.to_string();
    let mut results: Vec<apply::ApplyResult> = Vec::new();

    for _round in 1..=max_rounds {
        let mut round_diags = diagnostics::Diagnostics::from_parse_result(&current, path);
        if round_diags.ok {
            let program = parser::Parser::parse_recover(&current).program;
            typechecker::check_program(&program, &current, path, &mut round_diags);
        }
        let round_plan = fix::generate_plan(&round_diags, &current);
        let result = apply::apply_plan(&round_plan, &current);
        let no_progress = result.applied == 0 || result.patched_source == current;
        current = result.patched_source.clone();
        results.push(result);
        if no_progress {
            break;
        }
    }
    (current, results)
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
fn run_lsp() {
    use std::io::{self, BufRead, Read, Write};

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut buffer = String::new();

    // 文档状态：uri -> 源码文本
    let mut docs: std::collections::HashMap<String, String> = std::collections::HashMap::new();

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
                let len_str = &line[pos + 16..];
                if let Ok(n) = len_str.trim().parse::<usize>() {
                    content_len = Some(n);
                }
            }
        }

        let len = match content_len {
            Some(n) => n,
            None => continue,
        };

        // 2. 读取 JSON payload
        let mut payload = vec![0u8; len];
        if stdin.lock().read_exact(&mut payload).is_err() {
            return;
        }
        let json = match String::from_utf8(payload) {
            Ok(s) => s,
            Err(_) => continue,
        };

        // 3. 解析 JSON-RPC 消息
        let (id, method, params) = match lsp::parse_rpc_message(&json) {
            Some(m) => m,
            None => continue,
        };

        // 4. 处理请求
        let response = handle_lsp_method(&method, id, &params, &mut docs);

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
            Some(lsp::make_response(id, "null"))
        }
        "exit" => {
            process::exit(0);
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
            if let Some((uri, line, col)) = extract_hover_params(params) {
                if let Some(src) = docs.get(&uri) {
                    if let Some(hover) = lsp::handle_hover(src, line, col) {
                        let result = format!(
                            "{{\"contents\":{{\"kind\":\"markdown\",\"value\":\"{}\"}}}}",
                            hover.content.replace('"', "\\\"").replace('\n', "\\n")
                        );
                        return Some(lsp::make_response(id, &result));
                    }
                }
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
                        .map(|d| format!(",\"detail\":\"{}\"", d.replace('"', "\\\"")))
                        .unwrap_or_default();
                    format!(
                        "{{\"label\":\"{}\",\"kind\":{}{}}}",
                        item.label,
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
            Some(lsp::make_error_response(id, -32601, &format!("方法未实现: {}", method)))
        }
    }
}

/// 从 didOpen params 提取 uri 和 text
fn extract_did_open_params(params: &str) -> Option<(String, String)> {
    let uri = extract_json_string_field(params, "uri")?;
    let text = extract_json_string_field(params, "text")?;
    Some((uri, text))
}

/// 从 didChange params 提取 uri 和新文本（简化：取最后一个全量变更）
fn extract_did_change_params(params: &str) -> Option<(String, String)> {
    let uri = extract_json_string_field(params, "uri")?;
    // didChange 的 text 在 changes[].text 中，简化提取最后一个 "text":"..." 的值
    let text = extract_last_text_field(params)?;
    Some((uri, text))
}

/// 从 hover params 提取 uri, line, col
fn extract_hover_params(params: &str) -> Option<(String, usize, usize)> {
    let uri = extract_json_string_field(params, "uri")?;
    let line = extract_json_number_field(params, "line")?;
    let col = extract_json_number_field(params, "character")?;
    Some((uri, line, col))
}

/// 从 completion params 提取 uri
fn extract_completion_uri(params: &str) -> Option<String> {
    extract_json_string_field(params, "uri")
}

/// 简单提取 JSON 字符串字段
fn extract_json_string_field(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\":\"", key);
    let start = json.find(&needle)? + needle.len();
    let bytes = json.as_bytes();
    let mut end = start;
    let mut i = start;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == b'"' {
            end = i;
            break;
        }
        i += 1;
    }
    Some(json[start..end].to_string())
}

/// 简单提取 JSON 数字字段
fn extract_json_number_field(json: &str, key: &str) -> Option<usize> {
    let needle = format!("\"{}\":", key);
    let start = json.find(&needle)? + needle.len();
    let bytes = json.as_bytes();
    let mut end = start;
    while end < bytes.len() && bytes[end].is_ascii_whitespace() {
        end += 1;
    }
    let num_start = end;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    json[num_start..end].parse().ok()
}

/// 提取最后一个 "text":"..." 字段的值（用于 didChange 的 changes 数组）
fn extract_last_text_field(json: &str) -> Option<String> {
    let needle = "\"text\":\"";
    let mut last_text = None;
    let mut search_from = 0;
    while let Some(pos) = json[search_from..].find(needle) {
        let start = search_from + pos + needle.len();
        let bytes = json.as_bytes();
        let mut end = start;
        let mut i = start;
        while i < bytes.len() {
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                end = i;
                break;
            }
            i += 1;
        }
        last_text = Some(json[start..end].to_string());
        search_from = end + 1;
    }
    last_text
}

// ===== 单元测试（修复引擎深化 M2：迭代闭环）=====

#[cfg(test)]
mod tests {
    use super::*;

    /// 两轮收敛案例：第 1 轮删意外字符（LEX005，语法期），
    /// 第 2 轮解析通过后类型检查暴露 EFF001（插效应注解），第 3 轮收敛。
    const TWO_ROUND_SRC: &str = "fn helper(x: Int) -> Int\n    println(x)\n    x\nend\n\nfn main() -> Unit\n    println@(helper(1))\nend\n";

    #[test]
    fn iterative_apply_converges_in_two_fix_rounds() {
        let (final_src, results) = apply_iterative(TWO_ROUND_SRC, "test.lom", 5);
        let total_applied: usize = results.iter().map(|r| r.applied).sum();
        assert_eq!(total_applied, 2, "应修 2 处（@ 和效应注解）");
        assert_eq!(results.len(), 3, "两轮修复 + 一轮收敛判定");
        assert_eq!(results.last().unwrap().applied, 0, "末轮应无可修项");
        assert!(final_src.contains("! [IO]"), "final: {:?}", final_src);
        assert!(!final_src.contains('@'), "final: {:?}", final_src);
        // 修复后的源码应能干净通过诊断
        let diags = diagnostics::Diagnostics::from_parse_result(&final_src, "test.lom");
        assert!(diags.ok, "修复后仍有诊断: {:?}", diags.to_human());
    }

    #[test]
    fn iterative_apply_respects_max_rounds() {
        // 上限 1 轮：只修掉 @，EFF001 留给下一轮（被上限截断）
        let (final_src, results) = apply_iterative(TWO_ROUND_SRC, "test.lom", 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].applied, 1);
        assert!(!final_src.contains('@'));
        assert!(!final_src.contains("! [IO]"), "上限截断，效应注解未修");
    }

    #[test]
    fn iterative_apply_clean_source_single_round() {
        let (final_src, results) = apply_iterative(
            "fn main() -> Unit\n    println(1)\nend\n",
            "test.lom",
            5,
        );
        assert_eq!(results.len(), 1, "干净源码一轮即收敛");
        assert_eq!(results[0].applied, 0);
        assert_eq!(final_src, "fn main() -> Unit\n    println(1)\nend\n");
    }

    #[test]
    fn iterative_apply_medium_fixes_not_applied() {
        // 拼写修复是 Medium（M1 用户裁决）——迭代闭环不会自动改，一轮收敛
        let src = "fn main() -> Unit\n    let length = 5\n    println(lenght)\nend\n";
        let (final_src, results) = apply_iterative(src, "test.lom", 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].applied, 0);
        assert_eq!(final_src, src, "Medium 修复不被自动应用");
    }
}
