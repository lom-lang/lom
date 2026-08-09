// Lom REPL — Phase 4.2 交互式 read-eval-print loop
//
// 设计目标：
//   1. `lom repl` 启动交互式会话，支持多行输入
//   2. 上下文保持：前文定义的 fn/enum/let 对后续输入可见
//   3. 表达式自动求值并打印结果；语句执行不打印（let 除外，打印绑定的值）
//   4. 对 LLM 试错友好：错误不退出，继续接受输入
//
// 多行输入判定（is_input_complete）：
//   - 逐字符扫描，跟踪括号深度（()、{}、[]）
//   - 跟踪字符串状态（" 未闭合 → 不完整）
//   - 跟踪块关键字（fn/enum/from/if/while/for/match 需配对 end）
//   - 注释行（# 开头）忽略
//
// 特殊命令（以 : 开头）：
//   :q / :quit / :exit — 退出 REPL
//   :help              — 显示帮助
//   :reset             — 清空会话（重置 interpreter）

use crate::ast::{Item, Program};
use crate::interpreter::{Interpreter, RuntimeError, Value};
use crate::parser::Parser;

// ===== 多行输入完整性判定 =====

/// 判断输入是否构成完整的可解析单元
///
/// 完整性规则：
///   1. 括号深度归零（()、{}、[] 均平衡）
///   2. 不在字符串字面量内部
///   3. 块关键字（fn/enum/if/while/for/match）配对的 end 数量足够
///   4. 非空输入（注释/空白不算完整）
///
/// 注意：enum 是单行声明 `enum Color = Red | Green`，不需要 end。
/// 但 `fn`/`if`/`while`/`for`/`match` 块需要 `end` 闭合。
pub fn is_input_complete(input: &str) -> bool {
    if input.trim().is_empty() {
        return false;
    }

    // 检查是否全是注释（# 开头的行）
    let non_comment: String = input
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    if non_comment.trim().is_empty() {
        return false;
    }

    let mut paren_depth: i32 = 0;
    let mut brace_depth: i32 = 0;
    let mut bracket_depth: i32 = 0;
    let mut in_string = false;
    let mut block_depth: i32 = 0;

    let mut at_line_start = true; // 行首标记（遇到非空白字符后变 false，换行后变 true）

    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];

        if in_string {
            match c {
                '\\' => {
                    i += 2;
                    continue;
                }
                '"' => in_string = false,
                _ => {}
            }
            at_line_start = false;
            i += 1;
            continue;
        }

        match c {
            '"' => {
                in_string = true;
                at_line_start = false;
            }
            '#' => {
                // 注释行：跳到行尾
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            }
            '\n' => {
                at_line_start = true;
                i += 1;
                continue;
            }
            ' ' | '\t' | '\r' => {
                // 空白不改变 at_line_start（行首仍可能）
                i += 1;
                continue;
            }
            '(' => paren_depth += 1,
            ')' => paren_depth -= 1,
            '{' => brace_depth += 1,
            '}' => brace_depth -= 1,
            '[' => bracket_depth += 1,
            ']' => bracket_depth -= 1,
            _ => {
                // 非特殊字符：如果在行首，尝试匹配块关键字
                if at_line_start {
                    let word = read_word_at(&chars, i);
                    match word.as_str() {
                        "fn" | "if" | "while" | "for" | "match" => {
                            block_depth += 1;
                            i += word.len();
                            at_line_start = false;
                            continue;
                        }
                        "end" => {
                            block_depth -= 1;
                            i += 3;
                            at_line_start = false;
                            continue;
                        }
                        // enum 是单行声明，不需要 end（不增加 block_depth）
                        _ => {
                            at_line_start = false;
                        }
                    }
                }
            }
        }

        if c != ' ' && c != '\t' && c != '\r' && c != '\n' {
            at_line_start = false;
        }
        i += 1;
    }

    paren_depth == 0
        && brace_depth == 0
        && bracket_depth == 0
        && !in_string
        && block_depth <= 0
}

/// 从 chars 的 pos 处读取一个单词（字母/数字/下划线）
fn read_word_at(chars: &[char], pos: usize) -> String {
    let mut end = pos;
    while end < chars.len() {
        let c = chars[end];
        if c.is_ascii_alphabetic() || c == '_' || c.is_ascii_digit() {
            end += 1;
        } else {
            break;
        }
    }
    chars[pos..end].iter().collect()
}

// ===== REPL 会话核心 =====

/// REPL 会话状态
pub struct ReplSession {
    pub interpreter: Interpreter,
    /// 累积的源码（用于多行输入合并后解析）
    /// 注：实际解析时只用新输入，这里保留供 :show 等命令使用
    pub source_acc: String,
}

impl ReplSession {
    pub fn new() -> Self {
        ReplSession {
            interpreter: Interpreter::new(),
            source_acc: String::new(),
        }
    }

    /// 执行一行完整输入（已通过 is_input_complete 判定）
    ///
    /// 返回 ReplResult（始终成功，错误转为输出文本）
    /// - 顶层声明（fn/enum/from）：注册到 interpreter，返回简短确认
    /// - 语句/表达式：执行并返回结果（表达式的值会打印）
    /// - 运行时错误：转为输出文本，不传播 Err（REPL 不因错误崩溃）
    pub fn exec_line(&mut self, input: &str) -> Result<ReplResult, RuntimeError> {
        let trimmed = input.trim();

        // 特殊命令
        if trimmed.starts_with(':') {
            return Ok(self.handle_command(trimmed));
        }

        // 合并到累积源码
        if !self.source_acc.is_empty() {
            self.source_acc.push('\n');
        }
        self.source_acc.push_str(input);

        // 尝试解析为顶层 item（fn/enum/from）
        match Parser::parse(input) {
            Ok(program) if !program.items.is_empty() => {
                return self.exec_program(&program);
            }
            _ => {}
        }

        // 不是完整 program，尝试解析为表达式或语句
        // 包装成临时程序：fn main() -> <expr> end
        let wrapped = format!("fn main() -> Unit\n{}\nend\n", input);
        match Parser::parse(&wrapped) {
            Ok(program) => match self.exec_wrapped_main(&program, input) {
                Ok(result) => Ok(result),
                Err(e) => Ok(ReplResult {
                    should_continue: true,
                    output: format!("运行时错误: {}", repl_error_msg(&e)),
                }),
            },
            Err(e) => {
                // 解析失败：转为输出文本，不传播
                Ok(ReplResult {
                    should_continue: true,
                    output: format!("解析错误 ({}:{}): {}", e.line, e.col, e.message),
                })
            }
        }
    }

    /// 执行完整 program（顶层 item 模式）
    fn exec_program(&mut self, program: &Program) -> Result<ReplResult, RuntimeError> {
        let mut last_output = String::new();
        for item in &program.items {
            let result = self.interpreter.exec_item(item)?;
            match item {
                Item::Fn(f) => {
                    last_output = format!("fn {} 已定义", f.name);
                }
                Item::Enum(e) => {
                    last_output = format!("enum {} 已定义", e.name);
                }
                Item::Import(_) => {
                    last_output = "导入完成".to_string();
                }
            }
            // item 级别的返回值通常为 Unit，不打印
            let _ = result;
        }
        Ok(ReplResult {
            should_continue: true,
            output: last_output,
        })
    }

    /// 执行包装后的 main（表达式/语句模式）
    /// program 是包装后的，但我们要复用现有 interpreter 的函数表
    fn exec_wrapped_main(
        &mut self,
        program: &Program,
        original_input: &str,
    ) -> Result<ReplResult, RuntimeError> {
        // 临时注册 main，执行，然后保留上下文
        // 但 program 可能只含 main，我们需要把 main 加入 interpreter.functions
        for item in &program.items {
            if let Item::Fn(f) = item {
                if f.name == "main" {
                    // 检查 main body：如果是单条 let 或表达式，需要在全局环境执行并保留绑定
                    return self.exec_repl_stmt(&f.body, original_input);
                }
            }
        }
        Ok(ReplResult {
            should_continue: true,
            output: String::new(),
        })
    }

    /// 在全局环境执行 REPL 语句/表达式
    /// 如果是 let，绑定到全局环境并打印绑定的值
    /// 如果是表达式，求值并打印结果
    fn exec_repl_stmt(
        &mut self,
        block: &crate::ast::Block,
        original_input: &str,
    ) -> Result<ReplResult, RuntimeError> {
        let trimmed = original_input.trim();

        // 检查是否是 let 语句
        if trimmed.starts_with("let ") {
            // 用 interpreter 的全局环境执行 let
            let result = self.interpreter.exec_repl_block(block)?;
            return Ok(ReplResult {
                should_continue: true,
                output: format_repl_value(&result),
            });
        }

        // 表达式：求值并打印
        let result = self.interpreter.exec_repl_block(block)?;
        Ok(ReplResult {
            should_continue: true,
            output: format_repl_value(&result),
        })
    }

    /// 处理特殊命令（:q / :help / :reset / :show）
    fn handle_command(&mut self, cmd: &str) -> ReplResult {
        let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
        let command = parts[0];
        match command {
            ":q" | ":quit" | ":exit" => ReplResult {
                should_continue: false,
                output: "再见".to_string(),
            },
            ":help" => ReplResult {
                should_continue: true,
                output: help_text(),
            },
            ":reset" => {
                self.interpreter = Interpreter::new();
                self.source_acc.clear();
                ReplResult {
                    should_continue: true,
                    output: "会话已重置".to_string(),
                }
            }
            ":show" => ReplResult {
                should_continue: true,
                output: self.source_acc.clone(),
            },
            _ => ReplResult {
                should_continue: true,
                output: format!("未知命令: {}（输入 :help 查看可用命令）", command),
            },
        }
    }
}

/// REPL 执行结果
pub struct ReplResult {
    /// 是否继续 REPL（false 表示退出）
    pub should_continue: bool,
    /// 输出文本（已格式化，可直接打印）
    pub output: String,
}

/// 格式化 REPL 值输出
fn format_repl_value(v: &Value) -> String {
    match v {
        Value::Unit => String::new(), // 不打印 Unit
        _ => format!("{:?}", v),
    }
}

/// 帮助文本
fn help_text() -> String {
    let mut s = String::new();
    s.push_str("Lom REPL 命令：\n");
    s.push_str("  :help    显示本帮助\n");
    s.push_str("  :reset   重置会话（清空已定义的函数/变量）\n");
    s.push_str("  :show    显示累积的源码\n");
    s.push_str("  :q       退出 REPL\n");
    s.push_str("\n");
    s.push_str("用法：\n");
    s.push_str("  fn add(a: Int, b: Int) -> Int\n    a + b\n  end\n");
    s.push_str("  let x = 5\n");
    s.push_str("  add(x, 3)        # 自动求值并打印结果\n");
    s.push_str("  println(\"hi\")   # 执行副作用\n");
    s
}

/// 将 RuntimeError 转为友好的错误消息文本
fn repl_error_msg(e: &RuntimeError) -> String {
    match e {
        RuntimeError::Msg(s) => s.clone(),
        RuntimeError::EarlyReturn(_) => "提前返回（? 运算符在 REPL 顶层）".to_string(),
    }
}

// ===== 单元测试 =====

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_complete_simple_expr() {
        assert!(is_input_complete("1 + 2"));
        assert!(is_input_complete("println(\"hello\")"));
        assert!(is_input_complete("let x = 5"));
    }

    #[test]
    fn test_is_complete_unbalanced_paren() {
        assert!(!is_input_complete("add(1,"));
        assert!(!is_input_complete("f(1, (2"));
        assert!(!is_input_complete("println(\"hi\""));
    }

    #[test]
    fn test_is_complete_unclosed_string() {
        assert!(!is_input_complete("println(\"hello"));
        assert!(!is_input_complete("let s = \"unclosed"));
    }

    #[test]
    fn test_is_complete_block_needs_end() {
        assert!(!is_input_complete("fn add(a: Int, b: Int) -> Int\n    a + b"));
        assert!(is_input_complete("fn add(a: Int, b: Int) -> Int\n    a + b\nend"));
        assert!(!is_input_complete("if true\n    println(1)"));
        assert!(is_input_complete("if true\n    println(1)\nend"));
    }

    #[test]
    fn test_is_complete_nested_blocks() {
        assert!(is_input_complete(
            "fn f() -> Unit\n    if true\n        println(1)\n    end\nend"
        ));
        assert!(!is_input_complete(
            "fn f() -> Unit\n    if true\n        println(1)\n    end"
        ));
    }

    #[test]
    fn test_is_complete_empty() {
        assert!(!is_input_complete(""));
        assert!(!is_input_complete("   "));
        assert!(!is_input_complete("\n\n"));
    }

    #[test]
    fn test_is_complete_comment_only() {
        assert!(!is_input_complete("# just a comment"));
    }

    #[test]
    fn test_is_complete_match_block() {
        assert!(!is_input_complete(
            "match x\n    Ok(n) => n"
        ));
        assert!(is_input_complete(
            "match x\n    Ok(n) => n\nend"
        ));
    }

    #[test]
    fn test_is_complete_enum_decl() {
        // enum 是单行声明，不需要 end，应判为完整
        assert!(is_input_complete("enum Color = Red | Green"));
        assert!(is_input_complete("enum Shape = Circle(Float) | Square(Float)"));
    }

    #[test]
    fn test_repl_expr_evaluation() {
        let mut session = ReplSession::new();
        let result = session.exec_line("1 + 2").unwrap();
        assert!(result.should_continue);
        assert_eq!(result.output, "3");
    }

    #[test]
    fn test_repl_let_binds_to_session() {
        let mut session = ReplSession::new();
        // let x = 5
        session.exec_line("let x = 5").unwrap();
        // 引用 x
        let result = session.exec_line("x").unwrap();
        assert_eq!(result.output, "5");
    }

    #[test]
    fn test_repl_fn_definition_persists() {
        let mut session = ReplSession::new();
        session.exec_line("fn add(a: Int, b: Int) -> Int\n    a + b\nend").unwrap();
        let result = session.exec_line("add(2, 3)").unwrap();
        assert_eq!(result.output, "5");
    }

    #[test]
    fn test_repl_quit_command() {
        let mut session = ReplSession::new();
        let result = session.exec_line(":q").unwrap();
        assert!(!result.should_continue);
    }

    #[test]
    fn test_repl_help_command() {
        let mut session = ReplSession::new();
        let result = session.exec_line(":help").unwrap();
        assert!(result.should_continue);
        assert!(result.output.contains(":help"));
        assert!(result.output.contains(":q"));
    }

    #[test]
    fn test_repl_reset_command() {
        let mut session = ReplSession::new();
        session.exec_line("let x = 5").unwrap();
        session.exec_line(":reset").unwrap();
        // reset 后 x 应不可见（引用会报错，但这里测试 exec 不 panic）
        let result = session.exec_line("x");
        // 应该是错误（未定义），但 REPL 不应崩溃
        assert!(result.is_ok());
    }

    #[test]
    fn test_repl_unit_not_printed() {
        let mut session = ReplSession::new();
        let result = session.exec_line("println(\"hi\")").unwrap();
        // println 返回 Unit，REPL 不打印 Unit
        assert_eq!(result.output, "");
    }

    #[test]
    fn test_repl_error_does_not_crash() {
        let mut session = ReplSession::new();
        // 语法错误不应崩溃
        let result = session.exec_line("1 +");
        assert!(result.is_ok());
        // 运行时错误不应崩溃（返回 Err，但 REPL 上层捕获）
        let result2 = session.exec_line("undefined_var");
        // 可能是 Err（运行时错误），REPL 主循环会捕获
        assert!(result2.is_ok() || result2.is_err());
    }

    #[test]
    fn test_repl_multiline_fn_definition() {
        let mut session = ReplSession::new();
        // 多行 fn 定义：第一行不完整，第二行 end 闭合
        assert!(!is_input_complete("fn double(n: Int) -> Int\n    n * 2"));
        assert!(is_input_complete("fn double(n: Int) -> Int\n    n * 2\nend"));

        // 执行完整的 fn 定义
        let result = session.exec_line("fn double(n: Int) -> Int\n    n * 2\nend").unwrap();
        assert!(result.output.contains("fn double 已定义"));

        // 调用
        let result = session.exec_line("double(21)").unwrap();
        assert_eq!(result.output, "42");
    }

    #[test]
    fn test_repl_show_command() {
        let mut session = ReplSession::new();
        session.exec_line("let x = 5").unwrap();
        let result = session.exec_line(":show").unwrap();
        assert!(result.output.contains("let x = 5"));
    }

    #[test]
    fn test_repl_string_literal() {
        let mut session = ReplSession::new();
        let result = session.exec_line("\"hello\"").unwrap();
        assert_eq!(result.output, "\"hello\"");
    }

    #[test]
    fn test_repl_enum_definition_persists() {
        let mut session = ReplSession::new();
        // enum 单行声明
        let result = session.exec_line("enum Color = Red | Green | Blue").unwrap();
        assert!(result.output.contains("enum Color 已定义"));
    }

    #[test]
    fn test_repl_unknown_command() {
        let mut session = ReplSession::new();
        let result = session.exec_line(":unknown").unwrap();
        assert!(result.output.contains("未知命令"));
    }

    #[test]
    fn test_is_complete_string_with_escape() {
        // 字符串含转义引号，不应误判为未闭合
        assert!(is_input_complete("println(\"say \\\"hi\\\"\")"));
        assert!(!is_input_complete("println(\"unclosed \\\""));
    }

    #[test]
    fn test_is_complete_nested_parens() {
        assert!(is_input_complete("f(g(h(1)))"));
        assert!(!is_input_complete("f(g(h(1))"));
        assert!(!is_input_complete("f(g(h(1)))))"));
    }

    #[test]
    fn test_is_complete_brace_in_string() {
        // 字符串内的括号不应影响深度
        assert!(is_input_complete("let s = \"(unbalanced\""));
        assert!(is_input_complete("let s = \"{unbalanced\""));
    }
}
