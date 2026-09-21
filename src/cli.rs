// src/cli.rs — CLI 可测纯函数层（Q2 拆分自 main.rs，2026-09-08）
//
// 从 main.rs 迁出的可直测逻辑：参数解析、帮助文本、LSP JSON 参数提取、
// WASM 包合并、fix 迭代闭环。注（2026-09-14 R20 如实口径）：parse_args
// 的参数错误路径与 print_help 仍含 stderr 输出/直接退出（与拆分前
// main.rs 行为一致，非纯函数）；merge_packages_for_wasm/apply_iterative/
// LSP 提取为无 IO 纯逻辑。main.rs 保留入口（256MB 栈线程）、子命令分发
// 与其余终态退出码。

use std::fs;
use std::process;

use crate::apply;
use crate::ast;
use crate::diagnostics;
use crate::fix;
use crate::package;
use crate::parser;
use crate::typechecker;

#[derive(Debug, Default)]
pub(crate) struct CliArgs {
    /// 子命令：info（Phase 2.6）/ fix（Phase 2.7）；None 表示默认运行/检查模式
    pub(crate) subcommand: Option<String>,
    pub(crate) file: Option<String>,
    pub(crate) json: bool,
    pub(crate) check: bool,
    pub(crate) help: bool,
    /// Phase 6.1: --version 标志（打印编译器版本，来自 Cargo.toml）
    pub(crate) version: bool,
    /// Phase 2.7: --plan 标志（lom fix 专用，表示仅生成计划不应用；当前 --plan 是默认行为，标志仅作显式标记）
    pub(crate) plan: bool,
    /// Phase 3.1: --apply 标志（lom fix 专用，应用修复到源文件）
    pub(crate) apply: bool,
    /// Phase 3.1: --dry-run 标志（与 --apply 配合，只输出预览不写文件）
    pub(crate) dry_run: bool,
    /// Phase 4.1.3: --history 标志（lom fix 专用，查看修复历史记录）
    pub(crate) history: bool,
    /// Phase 7.2: --target <t>（lom build <file> --target wasm 编译到 WASM）
    pub(crate) target: Option<String>,
    /// Phase 7.2: -o/--output <path>（编译产物输出路径，默认 <file> 换 .wasm 后缀）
    pub(crate) output: Option<String>,
    /// Phase 8 前置（RFC-0003 §8.1）: --dump-ast 打印 AST 结构树到 stdout（不执行、不类型检查）
    pub(crate) dump_ast: bool,
    /// Phase 8.1（RFC-0003）: --dump-tokens 打印容错词法的 token 流到 stdout（自举 lexer 对账工具）
    pub(crate) dump_tokens: bool,
    /// Phase 3.5: -- 之后的参数，传递给 Lom 程序（通过 env::args() 读取）
    pub(crate) program_args: Vec<String>,
}

pub(crate) fn print_help(prog: &str) {
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
    eprintln!("  --dump-ast 打印 AST 结构树到 stdout（不执行、不类型检查；Phase 8 自举验收工具）");
    eprintln!("  --dump-tokens 打印 token 流到 stdout（Phase 8.1 自举 lexer 对账工具）");
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

pub(crate) fn parse_args(args: &[String]) -> CliArgs {
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
            "--dump-ast" => out.dump_ast = true,
            "--dump-tokens" => out.dump_tokens = true,
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
                if let Some(f) = &out.file {
                    eprintln!("只能指定一个文件，但收到多个: {} {}", f, a);
                    eprintln!("使用 --help 查看用法");
                    process::exit(1);
                }
                out.file = Some(a.clone());
            }
        }
    }
    out
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
/// 7.8 包链接：把依赖包源码合并进编译单元（包内 item 在前，主文件在后；
/// 重名函数后主文件覆盖——对齐解释器 load_packages 语义）。
///
/// `base_dir` = 被编译的 main.lom 所在目录（对齐解释器路径的包发现规则：
/// main.rs 按文件所在目录发现 lom.toml——build 曾按 cwd 发现，同一文件在
/// 不同 cwd 下编译结果漂移且与解释器路径不对称，D 包二期 2026-09-14 修复）。
pub(crate) fn merge_packages_for_wasm(
    mut program: ast::Program,
    base_dir: &std::path::Path,
) -> (ast::Program, Vec<String>) {
    let toml_path = base_dir.join("lom.toml");
    if !toml_path.exists() {
        return (program, Vec::new());
    }
    let manifest = match package::load_manifest_file(&toml_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("lom.toml 解析失败: {}", e);
            process::exit(1);
        }
    };
    let graph = match package::resolve_dependencies(&manifest, base_dir) {
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

/// 修复引擎深化 M2：迭代应用修复直到收敛
///
/// 每轮：重新诊断（词法+语法+类型）→ 生成计划 → 应用高置信度修复。
/// 收敛条件：一轮 applied==0（无可自动修复项）或修补后源码不再变化；
/// `max_rounds` 是震荡死循环的最后防线（修复 A 引入诊断 B、修 B 又引入 A 的场景）。
/// 返回 (最终源码, 各轮结果)。
pub(crate) fn apply_iterative(src: &str, path: &str, max_rounds: usize) -> (String, Vec<apply::ApplyResult>) {
    let mut current = src.to_string();
    let mut results: Vec<apply::ApplyResult> = Vec::new();

    for _round in 1..=max_rounds {
        let mut round_diags = diagnostics::Diagnostics::from_parse_result(&current, path);
        let mut round_fns: Vec<fix::FnInfo> = Vec::new();
        if round_diags.ok {
            let program = parser::Parser::parse_recover(&current).program;
            round_fns = fix::fn_infos(&program, &current);
            typechecker::check_program(&program, &current, path, &mut round_diags);
        }
        let round_plan = fix::generate_plan(&round_diags, &current, &round_fns);
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

// ===== LSP JSON 参数提取（serve 于 main.rs 的 LSP 粘合层）=====

/// 从 didOpen params 提取 uri 和 text
pub(crate) fn extract_did_open_params(params: &str) -> Option<(String, String)> {
    let uri = extract_json_string_field(params, "uri")?;
    let text = extract_json_string_field(params, "text")?;
    Some((uri, text))
}

/// 从 didChange params 提取 uri 和新文本（简化：取最后一个全量变更）
pub(crate) fn extract_did_change_params(params: &str) -> Option<(String, String)> {
    let uri = extract_json_string_field(params, "uri")?;
    // didChange 的 text 在 changes[].text 中，简化提取最后一个 "text":"..." 的值
    let text = extract_last_text_field(params)?;
    Some((uri, text))
}

/// 从 hover params 提取 uri, line, col
pub(crate) fn extract_hover_params(params: &str) -> Option<(String, usize, usize)> {
    let uri = extract_json_string_field(params, "uri")?;
    let line = extract_json_number_field(params, "line")?;
    let col = extract_json_number_field(params, "character")?;
    Some((uri, line, col))
}

/// 从 completion params 提取 uri
pub(crate) fn extract_completion_uri(params: &str) -> Option<String> {
    extract_json_string_field(params, "uri")
}

/// 简单提取 JSON 字符串字段
pub(crate) fn extract_json_string_field(json: &str, key: &str) -> Option<String> {
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
pub(crate) fn extract_json_number_field(json: &str, key: &str) -> Option<usize> {
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
pub(crate) fn extract_last_text_field(json: &str) -> Option<String> {
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

    /// M4：fix 语料端到端回归——每对 (bad, fixed) 跑 apply_iterative 后逐字一致。
    /// 语料在 eval/fix_corpus/，新增修复规则时应同步加语料对。
    /// 注意 04_spelling_medium：fixed 与 bad 逐字相同是**有意为之**
    /// （Medium 猜测性修复不被自动应用，源码必须保持不变）。
    /// D 包二期（2026-09-14）：包发现规则 = base_dir（main.lom 所在目录）而非
    /// cwd——build 曾按 cwd 发现导致同一文件在不同目录下编译结果漂移、且与
    /// 解释器路径（file_dir 发现）不对称。本测试锁定修复后的发现语义。
    #[test]
    fn merge_packages_discovers_manifest_from_base_dir_not_cwd() {
        let tmp = std::env::temp_dir().join(format!("lom_merge_test_{}", std::process::id()));
        std::fs::create_dir_all(tmp.join("mylib")).expect("建包目录失败");
        std::fs::write(
            tmp.join("lom.toml"),
            "name = \"app\"
version = \"0.1.0\"

[dependencies]
mylib = { path = \"mylib\" }
",
        )
        .expect("写主清单失败");
        std::fs::write(tmp.join("mylib").join("lom.toml"), "name = \"mylib\"
version = \"0.1.0\"
")
            .expect("写包清单失败");
        std::fs::write(
            tmp.join("mylib").join("lib.lom"),
            "fn pkg_double(x: Int) -> Int
    x * 2
end
",
        )
        .expect("写包源码失败");

        // 主程序零 item：合并后应只有包的 1 个 fn，包名表含 mylib
        let program = ast::Program { items: Vec::new() };
        let (merged, names) = merge_packages_for_wasm(program, &tmp);
        assert_eq!(merged.items.len(), 1, "包源码应合并进编译单元");
        assert_eq!(names, vec!["mylib".to_string()], "包名表应含依赖包");

        // base_dir 无 lom.toml：零合并（不受 cwd 是否有 lom.toml 影响）
        let empty_dir = tmp.join("no_manifest_here");
        std::fs::create_dir_all(&empty_dir).expect("建空目录失败");
        let program2 = ast::Program { items: Vec::new() };
        let (merged2, names2) = merge_packages_for_wasm(program2, &empty_dir);
        assert_eq!(merged2.items.len(), 0, "无清单目录不应合并任何包");
        assert!(names2.is_empty());

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn fix_corpus_end_to_end() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("eval/fix_corpus");
        let mut entries: Vec<_> = std::fs::read_dir(&dir)
            .expect("fix_corpus 目录不存在")
            .map(|e| e.unwrap().path())
            .filter(|p| p.to_string_lossy().ends_with(".bad.lom"))
            .collect();
        entries.sort();
        assert!(!entries.is_empty(), "fix_corpus 为空");
        for bad_path in entries {
            let fixed_path = bad_path.with_file_name(
                bad_path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .replace(".bad.lom", ".fixed.lom"),
            );
            let bad = std::fs::read_to_string(&bad_path).unwrap();
            let fixed = std::fs::read_to_string(&fixed_path)
                .unwrap_or_else(|_| panic!("缺少配对 fixed 文件: {:?}", fixed_path));
            let (final_src, _results) = apply_iterative(&bad, &bad_path.to_string_lossy(), 5);
            // CI 的 Windows runner autocrlf 检出为 CRLF，而修复动作的插入文本是 LF
            // （M3 的 05/06 是首批含换行插入的语料，CI #74 因此挂过）——行尾是
            // 环境噪声不是语义差异，比对前归一化（对齐 CI golden diff 的 tr -d '\r' 防线）
            assert_eq!(
                final_src.replace("\r\n", "\n"),
                fixed.replace("\r\n", "\n"),
                "语料 {:?} 修复结果不符",
                bad_path
            );
        }
    }

    /// R55（九审）端到端 1：MUT001 跨作用域探针穿过完整迭代闭环——
    /// 参数 x 重赋值 + 另一函数唯一 let x，apply_iterative 全程一字不改。
    #[test]
    fn r55_mut001_cross_scope_apply_iterative_is_noop() {
        let src = "fn f(x: Int) -> Int\n    x = x + 1\n    x\nend\n\
                   fn unrelated() -> Int\n    let x = 10\n    x\nend\n\
                   fn main() -> Unit\n    println(f(unrelated()))\nend\n";
        let (final_src, results) = apply_iterative(src, "test.lom", 5);
        let total: usize = results.iter().map(|r| r.applied).sum();
        assert_eq!(total, 0, "跨作用域不得有任何应用: {:?}", results);
        assert_eq!(final_src, src, "源码必须逐字不变");
    }

    /// R55（九审）端到端 2：EFF001 多行签名 + 同函数双效应探针穿过完整
    /// 迭代闭环——修复后源码可解析、注解落在签名结束行、最终零 error。
    #[test]
    fn r55_eff001_probes_apply_iterative_clean() {
        // 多行签名探针：注解必须插在 `) -> Int` 行末，而非 `fn helper(` 行
        let multiline = "fn helper(\n    x: Int\n) -> Int\n    println(x)\n    x\nend\n\
                         fn main() -> Unit\n    println(helper(7))\nend\n";
        let (src1, _r1) = apply_iterative(multiline, "test.lom", 5);
        assert!(
            src1.lines().nth(2).unwrap().contains(") -> Int ! [IO]"),
            "注解必须在第 3 行行末: {:?}",
            src1
        );
        let final1 = apply::FinalDiag::check(&src1, "test.lom");
        assert_eq!(final1.errors, 0, "修复后不得有 error: {:?}", src1);

        // 同函数双效应探针：必须是一段合并注解，不是两段叠加
        let dup = "fn io_work() -> Unit ! [IO]\n    println(\"io\")\nend\n\
                   fn clock_work() -> Unit ! [Clock]\n    ()\nend\n\
                   fn helper() -> Unit\n    io_work()\n    clock_work()\nend\n\
                   fn main() -> Unit\n    helper()\nend\n";
        let (src2, _r2) = apply_iterative(dup, "test.lom", 5);
        assert!(
            src2.contains("fn helper() -> Unit ! [IO, Clock]"),
            "必须合并为单条注解: {:?}",
            src2
        );
        assert!(
            !src2.contains("] ! ["),
            "不得出现两段叠加注解: {:?}",
            src2
        );
        let final2 = apply::FinalDiag::check(&src2, "test.lom");
        assert_eq!(final2.errors, 0);
        // EFF001 修复闭环：注解补齐后 warning 也应清零
        assert_eq!(final2.warnings, 0, "注解补齐后不应残留 EFF001: {:?}", src2);
    }
}
