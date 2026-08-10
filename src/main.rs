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
mod fix;
mod fix_history;
mod info;
mod interpreter;
mod json;
mod lexer;
mod lsp;
mod parser;
mod repl;
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
    /// Phase 3.1: --apply 标志（lom fix 专用，应用修复到源文件）
    apply: bool,
    /// Phase 3.1: --dry-run 标志（与 --apply 配合，只输出预览不写文件）
    dry_run: bool,
    /// Phase 4.1.3: --history 标志（lom fix 专用，查看修复历史记录）
    history: bool,
    /// Phase 3.5: -- 之后的参数，传递给 Lom 程序（通过 env::args() 读取）
    program_args: Vec<String>,
}

fn print_help(prog: &str) {
    eprintln!("Lom 解释器 (Phase 3.5) — AI 原生编程语言");
    eprintln!();
    eprintln!("用法:");
    eprintln!("  {prog} <file.lom>                运行 .lom 程序（默认）");
    eprintln!("  {prog} <file.lom> -- <args...>   运行 .lom 程序，传递参数（通过 env::args() 读取）");
    eprintln!("  {prog} <file.lom> --json         仅诊断，输出结构化 JSON（不执行）");
    eprintln!("  {prog} <file.lom> --check        仅诊断，输出人类可读格式（不执行）");
    eprintln!("  {prog} info <file.lom> [--json]  导出类型信息（函数/枚举/导入签名）");
    eprintln!("  {prog} fix <file.lom> [--plan] [--json]  生成 AI 修复计划（lom-fix/v1）");
    eprintln!("  {prog} fix <file.lom> --apply [--dry-run] [--json]  应用修复到源文件");
    eprintln!("  {prog} fix --history [--json]    查看修复历史记录");
    eprintln!("  {prog} repl                       启动交互式 REPL（Phase 4.2）");
    eprintln!("  {prog} lsp                        启动 LSP 服务器（Phase 4.3，stdio JSON-RPC）");
    eprintln!("  {prog} --help | -h               显示帮助");
    eprintln!();
    eprintln!("子命令:");
    eprintln!("  info        导出类型信息（Phase 2.6）。默认人类可读；--json 输出 lom-info/v1 schema");
    eprintln!("  fix         生成/应用修复计划（Phase 2.7/3.1）。默认人类可读；--json 输出 lom-fix/v1 或 lom-apply/v1 schema");
    eprintln!("              --plan：仅生成计划不应用（默认行为）");
    eprintln!("              --apply：应用高置信度修复到源文件（Phase 3.1）");
    eprintln!("              --dry-run：与 --apply 配合，只输出预览不写文件");
    eprintln!("              --history：查看修复历史记录（Phase 4.1.3，存储于 .lom/fix-history.jsonl）");
    eprintln!("  repl        启动交互式 REPL（Phase 4.2）。支持多行输入、上下文保持、:help/:reset/:q 命令");
    eprintln!("  lsp         启动 LSP 服务器（Phase 4.3）。stdio JSON-RPC 2.0，支持 hover/completion/diagnostics");
    eprintln!();
    eprintln!("选项:");
    eprintln!("  --          参数分隔符：之后的所有参数传递给 Lom 程序（通过 env::args() 读取，Phase 3.5）");
    eprintln!("  --json     结构化 JSON 输出（诊断用 lom-diag/v1；info 用 lom-info/v1；fix 用 lom-fix/v1；apply 用 lom-apply/v1），便于 LLM 消费");
    eprintln!("  --check    仅做词法/语法/类型检查，不执行；输出带源码上下文的人类可读诊断");
    eprintln!("  --plan     lom fix 子命令专用：仅生成修复计划（默认）");
    eprintln!("  --apply    lom fix 子命令专用：应用修复到源文件（Phase 3.1）");
    eprintln!("  --dry-run  lom fix --apply 子命令专用：只预览不写文件");
    eprintln!("  --history  lom fix 子命令专用：查看修复历史记录（Phase 4.1.3）");
    eprintln!("  --help, -h 显示本帮助");
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
/// 为诊断集合生成修复计划，或应用修复到源文件：
/// - 默认 / `--plan`：生成修复计划（lom-fix/v1）
/// - `--apply`：应用高置信度修复到源文件（lom-apply/v1）
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
    if cli.apply {
        let result = apply::apply_plan(&plan, src);

        if cli.json {
            print!("{}", apply::to_json(&result, path));
        } else {
            print!("{}", apply::to_human(&result, path));
        }

        // --dry-run 不写文件；否则写回源文件
        if !cli.dry_run && result.applied > 0 {
            if let Err(e) = fs::write(path, &result.patched_source) {
                eprintln!("apply 写文件失败: {}", e);
                process::exit(1);
            }

            // Phase 4.1.3: 追加修复历史记录到 .lom/fix-history.jsonl
            // 供 LLM 学习"过去修了什么"，辅助后续修复决策
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
            };
            let history_path = std::path::Path::new(".lom/fix-history.jsonl");
            if let Err(e) = fix_history::append_history(&entry, history_path) {
                eprintln!("警告：修复历史记录写入失败: {}", e);
                // 历史记录失败不阻止 apply 成功
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
