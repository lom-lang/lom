// Lom Parser — Phase 1 手写递归下降 + Pratt 表达式解析
// Phase 2.2：容错解析器（带洞 AST），支持多错误收集与同步点恢复
// 语法见 LANGUAGE_SPEC.md §3 EBNF

use crate::ast::*;
use crate::lexer::{Lexer, SpannedToken, Token};
use std::fmt;

/// 解析错误
#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "语法错误 ({}:{}): {}", self.line, self.col, self.message)
    }
}

impl std::error::Error for ParseError {}

/// 容错解析结果（Phase 2.2）
///
/// `program` 始终非空（可能带 Hole 节点），`errors` 为收集到的全部错误。
/// - `is_ok()` 为 true 时，program 是干净的（无 Hole、无错误），可直接执行。
/// - `is_ok()` 为 false 时，program 是"带洞 AST"：错误处插入 Hole，其余部分正常解析。
///   带洞 AST 不可直接执行（解释器遇到 Hole 报运行时错误），但可用于：
///   - 一次性向 LLM 反馈全部错误（而非第一个）
///   - 后续 `lom info`/`lom fix` 工具消费部分 AST
pub struct ParseResult {
    pub program: Program,
    pub errors: Vec<ParseError>,
}

impl ParseResult {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// 块元素：一条语句或尾表达式（parse_block 循环体的产出）
enum BlockEl {
    Stmt(Stmt),
    Tail(Expr),
}

/// 解析器
pub struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
    /// 容错模式下收集的错误列表（严格模式下始终为空，错误经 ? 向上传播）
    errors: Vec<ParseError>,
    /// true = 容错模式（错误不终止，插入 Hole 继续）；false = 严格模式（首个错误即返回）
    recover: bool,
}

impl Parser {
    fn new(tokens: Vec<SpannedToken>) -> Self {
        Parser {
            tokens,
            pos: 0,
            errors: Vec::new(),
            recover: false,
        }
    }

    /// 严格解析入口：首个错误即返回 Err；无错误返回 Ok(program)。
    /// 内部走容错路径，取首个错误（若存在）。保持向后兼容。
    pub fn parse(src: &str) -> Result<Program, ParseError> {
        let r = Self::parse_recover(src);
        if let Some(e) = r.errors.into_iter().next() {
            Err(e)
        } else {
            Ok(r.program)
        }
    }

    /// 容错解析入口（Phase 2.2）：收集全部词法与语法错误，返回带洞 AST。
    /// 即使有错误也始终返回 ParseResult（program 可能含 Hole）。
    pub fn parse_recover(src: &str) -> ParseResult {
        let (tokens, lex_errors) = Lexer::new(src).tokenize_recover();
        let mut errors: Vec<ParseError> = lex_errors
            .into_iter()
            .map(|e| ParseError {
                message: e.message,
                line: e.line,
                col: e.col,
            })
            .collect();
        let mut p = Parser::new(tokens);
        p.recover = true;
        let program = match p.parse_program() {
            Ok(prog) => prog,
            // 容错模式下 parse_program 理论上不返回 Err（错误已收集），此分支为防御
            Err(e) => {
                errors.push(e);
                Program { items: Vec::new() }
            }
        };
        errors.extend(p.errors);
        ParseResult { program, errors }
    }

    // ===== 辅助方法 =====

    fn peek(&self) -> &Token {
        &self.tokens[self.pos].token
    }

    fn current(&self) -> &SpannedToken {
        &self.tokens[self.pos]
    }

    fn advance(&mut self) -> SpannedToken {
        let t = self.tokens[self.pos].clone();
        if !matches!(t.token, Token::Eof) {
            self.pos += 1;
        }
        t
    }

    fn check(&self, t: &Token) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(t)
    }

    fn matches(&mut self, t: &Token) -> bool {
        if self.check(t) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, t: &Token, what: &str) -> Result<(), ParseError> {
        if self.check(t) {
            self.advance();
            Ok(())
        } else {
            Err(ParseError {
                message: format!("期望 {}，得到 {}", what, self.token_name(self.peek())),
                line: self.current().line,
                col: self.current().col,
            })
        }
    }

    fn token_name(&self, t: &Token) -> String {
        match t {
            Token::Eof => "文件结束".to_string(),
            Token::Int(n) => format!("整数 {}", n),
            Token::Float(n) => format!("浮点数 {}", n),
            Token::Str(s) => format!("字符串 \"{}\"", s),
            Token::Ident(s) => format!("标识符 '{}'", s),
            _ => format!("{:?}", t),
        }
    }

    fn err(&self, msg: String) -> ParseError {
        ParseError {
            message: msg,
            line: self.current().line,
            col: self.current().col,
        }
    }

    /// 检查当前 token 是否位于新行（即与前一个 token 不同行）
    /// 用于区分跨行的二元运算符和一元运算符
    fn is_newline_before(&self) -> bool {
        if self.pos == 0 {
            return false;
        }
        let cur_line = self.tokens[self.pos].line;
        let prev_line = self.tokens[self.pos - 1].line;
        cur_line != prev_line
    }

    /// Phase 3.2: 获取前一个已消费 token 的 (line, col)
    /// 用于构造 span 的 end 位置（签名行末 = body 前一个 token）
    fn prev_token_pos(&self) -> (usize, usize) {
        if self.pos == 0 {
            return (0, 0);
        }
        let prev = &self.tokens[self.pos - 1];
        (prev.line, prev.col)
    }

    /// v0.4.1 P0-3: 若当前 token 是复合赋值运算符（+= -= *= /=），消费并返回对应的二元运算符
    /// 换行守卫：复合赋值必须与左侧标识符同行（与跨行 `-` 不当二元减法的规则一致，
    /// 防止 `x\n+= 1` 被静默合并成一条语句）
    fn match_compound_assign(&mut self) -> Option<BinOp> {
        if self.is_newline_before() {
            return None;
        }
        let op = match self.peek() {
            Token::PlusEq => BinOp::Add,
            Token::MinusEq => BinOp::Sub,
            Token::StarEq => BinOp::Mul,
            Token::SlashEq => BinOp::Div,
            _ => return None,
        };
        self.advance();
        Some(op)
    }

    // ===== 容错恢复辅助（Phase 2.2）=====

    /// 同步到下一个顶层 item 起点：跳过 token 直到 fn/enum/from/EOF
    /// 用于 parse_program 中某个 item 解析失败后，跳到下一个可解析的顶层声明
    fn sync_to_top(&mut self) {
        while !matches!(
            self.peek(),
            Token::Eof | Token::Fn | Token::Enum | Token::From
        ) {
            self.advance();
        }
    }

    /// 同步到语句边界：跳过 token 直到块终止符（end/elif/else/eof）或
    /// 出现在新行的"语句/表达式起始 token"。
    /// 用于 parse_block 中某条语句解析失败后，跳到下一条可解析的语句。
    ///
    /// Lom 是换行敏感的：新的一行即新语句。因此新行上的语句关键字
    /// （let/if/while/for/return/match）或表达式起始 token
    /// （标识符/字面量/`(`/`{`/True/False）都视为下一条语句的起点。
    /// 注意：不包含 `fn`/`enum`/`from`——它们是顶层声明，且 `fn` 与闭包字面量
    /// 存在歧义，留给顶层 sync_to_top 处理（缺失 end 的已知限制）。
    fn is_block_stmt_start(&self) -> bool {
        matches!(
            self.peek(),
            Token::Let
                | Token::If
                | Token::While
                | Token::For
                | Token::Return
                | Token::Match
                | Token::Ident(_)
                | Token::Int(_)
                | Token::Float(_)
                | Token::Str(_)
                | Token::True
                | Token::False
                | Token::LParen
                | Token::LBrace
        )
    }

    fn sync_to_stmt_boundary(&mut self) {
        while !matches!(
            self.peek(),
            Token::Eof | Token::End | Token::Elif | Token::Else
        ) {
            if self.is_newline_before() && self.is_block_stmt_start() {
                break;
            }
            self.advance();
        }
    }

    /// 保证解析进度：若自 saved_pos 以来未前进，强制前进一个 token，避免无限循环
    fn ensure_progress(&mut self, saved_pos: usize) {
        if self.pos <= saved_pos {
            self.advance();
        }
    }

    // ===== 顶层 =====

    fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut items = Vec::new();
        while !self.check(&Token::Eof) {
            let saved_pos = self.pos;
            match self.parse_item() {
                Ok(item) => items.push(item),
                Err(e) => {
                    if self.recover {
                        self.errors.push(e);
                        self.sync_to_top();
                        self.ensure_progress(saved_pos);
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        Ok(Program { items })
    }

    fn parse_item(&mut self) -> Result<Item, ParseError> {
        match self.peek() {
            Token::Fn => Ok(Item::Fn(self.parse_fn_decl()?)),
            Token::Enum => Ok(Item::Enum(self.parse_enum_decl()?)),
            Token::From => Ok(Item::Import(self.parse_import_decl()?)),
            _ => Err(self.err(format!(
                "期望函数声明 'fn'、枚举声明 'enum' 或导入声明 'from'，得到 {}",
                self.token_name(self.peek())
            ))),
        }
    }

    /// 解析导入声明：from <module> import { name1, name2 as alias2, ... }
    /// module 为点分路径（如 io、string、math、utils.helpers）
    /// 每项可带别名：name as alias
    fn parse_import_decl(&mut self) -> Result<ImportDecl, ParseError> {
        self.advance(); // from
        let module = self.parse_module_path()?;
        self.expect(&Token::Import, "'import'")?;
        self.expect(&Token::LBrace, "'{' (导入列表开始)")?;
        let mut items = Vec::new();
        // 允许空导入列表 from m import {} （语义检查阶段报错，parser 不阻止）
        if !self.check(&Token::RBrace) {
            loop {
                // 允许尾随逗号
                if self.check(&Token::RBrace) {
                    break;
                }
                let name = self.parse_ident()?;
                let alias = if self.matches(&Token::As) {
                    self.parse_ident()?
                } else {
                    name.clone()
                };
                items.push(ImportItem { name, alias });
                if !self.matches(&Token::Comma) {
                    break;
                }
            }
        }
        self.expect(&Token::RBrace, "'}' (导入列表结束)")?;
        Ok(ImportDecl { module, items })
    }

    /// 解析点分模块路径：io / utils.helpers / a.b.c
    fn parse_module_path(&mut self) -> Result<String, ParseError> {
        let first = self.parse_ident()?;
        let mut path = first;
        while self.matches(&Token::Dot) {
            let seg = self.parse_ident()?;
            path.push('.');
            path.push_str(&seg);
        }
        Ok(path)
    }

    fn parse_fn_decl(&mut self) -> Result<FnDecl, ParseError> {
        let fn_tok = self.advance(); // fn（记录位置用于 span）
        let name = self.parse_ident()?;
        self.expect(&Token::LParen, "'('")?;
        let params = self.parse_params()?;
        let ret_type = if self.matches(&Token::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };
        // Phase 2.5: 可选效应注解 `! [Effect1, Effect2]`
        let effects = self.parse_effects()?;
        // Phase 3.2: 签名 span = fn 关键字位置 .. body 前一个 token 位置
        // body 前一个 token 是 effects 的 `]` 或 ret_type 或 `)`（无 ret/effects 时）
        // 简化：用 prev_token 的 line/col 作为 end
        let (end_line, end_col) = self.prev_token_pos();
        let body = self.parse_block()?;
        Ok(FnDecl {
            name,
            params,
            ret_type,
            effects,
            body,
            span: Span {
                line: fn_tok.line,
                col: fn_tok.col,
                end_line,
                end_col,
            },
        })
    }

    /// 解析可选的效应注解：`! [E1, E2, ...]`
    /// 若当前位置不是 `!`，返回空 Vec（纯函数）。
    /// 语法：`!` `[` ident { `,` ident } `]`
    /// 与一元 `!`（逻辑非）的歧义：函数签名位置不会出现表达式，
    /// 因此 `!` 在 ret_type 之后只能解读为效应注解。
    fn parse_effects(&mut self) -> Result<Vec<Effect>, ParseError> {
        if !self.matches(&Token::Bang) {
            return Ok(Vec::new());
        }
        self.expect(&Token::LBracket, "'[' (效应列表开始)")?;
        let mut effects = Vec::new();
        // 空列表 `! []` 允许（等价于无效应，纯函数显式写法）
        if !self.check(&Token::RBracket) {
            loop {
                let e = self.parse_ident()?;
                effects.push(e);
                if !self.matches(&Token::Comma) {
                    break;
                }
            }
        }
        self.expect(&Token::RBracket, "']' (效应列表结束)")?;
        Ok(effects)
    }

    fn parse_params(&mut self) -> Result<Vec<Param>, ParseError> {
        let mut params = Vec::new();
        if !self.check(&Token::RParen) {
            loop {
                let name = self.parse_ident()?;
                self.expect(&Token::Colon, "':'")?;
                let ty = self.parse_type()?;
                params.push(Param { name, ty });
                if !self.matches(&Token::Comma) {
                    break;
                }
            }
        }
        self.expect(&Token::RParen, "')'")?;
        Ok(params)
    }

    fn parse_ident(&mut self) -> Result<String, ParseError> {
        match self.advance().token {
            Token::Ident(s) => Ok(s),
            other => Err(self.err(format!("期望标识符，得到 {}", self.token_name(&other)))),
        }
    }

    fn parse_type(&mut self) -> Result<Type, ParseError> {
        match self.peek() {
            Token::LBrace => {
                // Record type: {x: Int, y: Int}
                self.advance();
                let mut fields = Vec::new();
                if !self.check(&Token::RBrace) {
                    loop {
                        let name = self.parse_ident()?;
                        self.expect(&Token::Colon, "':' (记录字段类型分隔)")?;
                        let ty = self.parse_type()?;
                        fields.push((name, ty));
                        if !self.matches(&Token::Comma) {
                            break;
                        }
                    }
                }
                self.expect(&Token::RBrace, "'}' (闭合记录类型)")?;
                Ok(Type::Record(fields))
            }
            Token::LParen => {
                // Tuple type: (Int, String) — 单元素 (Int) 退化为该类型本身；() 为 Unit
                self.advance();
                if self.matches(&Token::RParen) {
                    return Ok(Type::Unit);
                }
                let first = self.parse_type()?;
                if self.matches(&Token::Comma) {
                    let mut elems = vec![first];
                    loop {
                        elems.push(self.parse_type()?);
                        if !self.matches(&Token::Comma) {
                            break;
                        }
                    }
                    self.expect(&Token::RParen, "')' (闭合元组类型)")?;
                    Ok(Type::Tuple(elems))
                } else {
                    self.expect(&Token::RParen, "')'")?;
                    Ok(first)
                }
            }
            _ => {
                // Named type, possibly with generic args
                let name = self.parse_ident()?;
                let base = Type::from_name(&name);
                // 泛型：Name<arg1, arg2, ...>
                if self.check(&Token::Lt) {
                    self.advance(); // <
                    let mut args = vec![self.parse_type()?];
                    while self.matches(&Token::Comma) {
                        args.push(self.parse_type()?);
                    }
                    self.expect(&Token::Gt, "'>' (闭合泛型参数)")?;
                    match base {
                        Type::Named(n) if n == "Result" => {
                            if args.len() != 2 {
                                return Err(self.err(format!(
                                    "Result 期望 2 个类型参数，得到 {} 个",
                                    args.len()
                                )));
                            }
                            Ok(Type::Result(Box::new(args[0].clone()), Box::new(args[1].clone())))
                        }
                        Type::Named(n) if n == "Option" => {
                            if args.len() != 1 {
                                return Err(self.err(format!(
                                    "Option 期望 1 个类型参数，得到 {} 个",
                                    args.len()
                                )));
                            }
                            Ok(Type::Option(Box::new(args[0].clone())))
                        }
                        Type::Named(n) => Ok(Type::Generic(n, args)),
                        _ => Err(self.err(format!("类型 {:?} 不支持泛型参数", base))),
                    }
                } else {
                    Ok(base)
                }
            }
        }
    }

    /// 解析枚举声明
    /// 形式 1（单行）：enum Name<T, E> = V1(T) | V2(E) | V3
    /// 形式 2（多行）：enum Name<T, E>\n V1(T)\n V2(E)\n end（变体前可选 |）
    fn parse_enum_decl(&mut self) -> Result<EnumDecl, ParseError> {
        let enum_tok = self.advance(); // enum（记录位置用于 span）
        let name = self.parse_ident()?;
        // 可选类型参数：<T, E>
        let mut type_params = Vec::new();
        if self.check(&Token::Lt) {
            self.advance(); // <
            loop {
                type_params.push(self.parse_ident()?);
                if !self.matches(&Token::Comma) {
                    break;
                }
            }
            self.expect(&Token::Gt, "'>' (闭合类型参数)")?;
        }
        // Phase 3.2: enum 签名 span = enum 关键字 .. `=` 前一个 token（name 或 `>`）
        // 多行形式（无 `=`）时，用 name 或 `>` 位置作为 end
        let (end_line, end_col) = self.prev_token_pos();
        let mut variants = Vec::new();
        if self.matches(&Token::Assign) {
            // 单行形式：V1 | V2 | V3（无 end）
            loop {
                variants.push(self.parse_enum_variant()?);
                if self.matches(&Token::Bar) {
                    continue;
                } else {
                    break;
                }
            }
        } else {
            // 多行形式：每行一个变体（可选 | 前缀），以 end 闭合
            while !self.check(&Token::End) && !self.check(&Token::Eof) {
                self.matches(&Token::Bar); // 可选 | 前缀
                variants.push(self.parse_enum_variant()?);
            }
            self.expect(&Token::End, "'end' (闭合 enum)")?;
        }
        Ok(EnumDecl {
            name,
            type_params,
            variants,
            span: Span {
                line: enum_tok.line,
                col: enum_tok.col,
                end_line,
                end_col,
            },
        })
    }

    fn parse_enum_variant(&mut self) -> Result<EnumVariantDecl, ParseError> {
        let name = self.parse_ident()?;
        let fields = if self.check(&Token::LParen) && !self.is_newline_before() {
            self.advance(); // (
            let mut fs = Vec::new();
            if !self.check(&Token::RParen) {
                loop {
                    fs.push(self.parse_type()?);
                    if !self.matches(&Token::Comma) {
                        break;
                    }
                }
            }
            self.expect(&Token::RParen, "')' (闭合变体参数)")?;
            fs
        } else {
            Vec::new()
        };
        Ok(EnumVariantDecl { name, fields })
    }

    // ===== 块 =====

    /// 解析一个块元素
    /// `if_can_be_tail`：当遇到末尾的 if 表达式时是否允许作为 tail
    ///   （仅当此前无 tail 且无裸表达式语句时为真，与原逻辑一致）
    fn parse_block_el(&mut self, if_can_be_tail: bool) -> Result<BlockEl, ParseError> {
        // 语句关键字
        if matches!(
            self.peek(),
            Token::Let | Token::While | Token::For | Token::Return
        ) {
            return Ok(BlockEl::Stmt(self.parse_stmt()?));
        }
        if matches!(self.peek(), Token::If) {
            // if 可能是语句或 tail 表达式
            let if_stmt = self.parse_if()?;
            if if_can_be_tail
                && matches!(
                    self.peek(),
                    Token::End | Token::Elif | Token::Else | Token::Eof
                ) {
                // if 是块最后一个元素，作为 tail
                return Ok(BlockEl::Tail(Expr::If(Box::new(if_stmt))));
            }
            return Ok(BlockEl::Stmt(Stmt::If(if_stmt)));
        }
        // 表达式
        let e = self.parse_expr()?;
        // 检查是否是赋值：ident = expr
        if self.check(&Token::Assign) {
            self.advance(); // =
            let value = self.parse_expr()?;
            match e {
                Expr::Ident(name) => Ok(BlockEl::Stmt(Stmt::Assign {
                    target: name,
                    value,
                })),
                _ => Err(self.err("赋值目标必须是变量".to_string())),
            }
        } else if let Some(op) = self.match_compound_assign() {
            // v0.4.1 P0-3：复合赋值 ident += expr —— 去糖为 ident = ident op expr
            // 解释器/类型检查器复用 Assign + Binary 的现有检查(可变性、NAM003、TYPE001),无需改动
            let rhs = self.parse_expr()?;
            match e {
                Expr::Ident(name) => Ok(BlockEl::Stmt(Stmt::Assign {
                    value: Expr::Binary {
                        op,
                        left: Box::new(Expr::Ident(name.clone())),
                        right: Box::new(rhs),
                    },
                    target: name,
                })),
                _ => Err(self.err("复合赋值目标必须是变量".to_string())),
            }
        } else if matches!(
            self.peek(),
            Token::End | Token::Elif | Token::Else | Token::Eof
        ) {
            Ok(BlockEl::Tail(e))
        } else {
            Ok(BlockEl::Stmt(Stmt::Expr(e)))
        }
    }

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        let mut stmts = Vec::new();
        let mut tail = None;
        // block 终止于 end / elif / else / eof（elif/else 由 parse_if 处理）
        while !matches!(
            self.peek(),
            Token::End | Token::Elif | Token::Else | Token::Eof
        ) {
            // if 作为 tail 的条件：此前无 tail 且无裸表达式语句
            let if_can_be_tail = tail.is_none()
                && stmts.iter().all(|s| !matches!(s, Stmt::Expr(_)));
            let saved_pos = self.pos;
            match self.parse_block_el(if_can_be_tail) {
                Ok(BlockEl::Stmt(s)) => stmts.push(s),
                Ok(BlockEl::Tail(e)) => {
                    tail = Some(Box::new(e));
                    break;
                }
                Err(e) => {
                    if self.recover {
                        let line = e.line;
                        let col = e.col;
                        self.errors.push(e);
                        // 插入 Hole 占位，保持块结构完整
                        stmts.push(Stmt::Hole { line, col });
                        self.sync_to_stmt_boundary();
                        self.ensure_progress(saved_pos);
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        // 只消费 end；elif/else 留给 parse_if
        if self.check(&Token::End) {
            self.advance();
        }
        Ok(Block { stmts, tail })
    }

    // ===== 语句 =====

    fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        match self.peek() {
            Token::Let => self.parse_let(),
            Token::If => Ok(Stmt::If(self.parse_if()?)),
            Token::While => self.parse_while(),
            Token::For => self.parse_for(),
            Token::Return => self.parse_return(),
            _ => Ok(Stmt::Expr(self.parse_expr()?)),
        }
    }

    fn parse_let(&mut self) -> Result<Stmt, ParseError> {
        self.advance(); // let
        let mutable = self.matches(&Token::Mut);
        // Phase 5.1: let (a, b, ...) = expr 元组解构（不支持 mut 组合）
        if self.check(&Token::LParen) {
            if mutable {
                return Err(self.err("元组解构不支持 mut（解构绑定不可变）".to_string()));
            }
            return self.parse_let_destruct();
        }
        let name = self.parse_ident()?;
        let ty = if self.matches(&Token::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(&Token::Assign, "'='")?;
        let value = self.parse_expr()?;
        Ok(Stmt::Let {
            mutable,
            name,
            ty,
            value,
        })
    }

    /// Phase 5.1: 解析 let (a, b, ...) = expr
    /// 当前已消耗 let，下一个 token 是 LParen
    fn parse_let_destruct(&mut self) -> Result<Stmt, ParseError> {
        self.advance(); // (
        let mut names = Vec::new();
        loop {
            names.push(self.parse_ident()?);
            if !self.matches(&Token::Comma) {
                break;
            }
            // 尾逗号容错: let (a, b,) = ...
            if self.check(&Token::RParen) {
                break;
            }
        }
        self.expect(&Token::RParen, "')'")?;
        self.expect(&Token::Assign, "'='")?;
        let value = self.parse_expr()?;
        Ok(Stmt::LetDestruct { names, value })
    }

    fn parse_if(&mut self) -> Result<IfStmt, ParseError> {
        self.advance(); // if
        let cond = self.parse_expr()?;
        let body = self.parse_block()?;
        let mut branches = vec![(cond, body)];
        let mut else_branch = None;
        while self.matches(&Token::Elif) {
            let c = self.parse_expr()?;
            let b = self.parse_block()?;
            branches.push((c, b));
        }
        if self.matches(&Token::Else) {
            else_branch = Some(self.parse_block()?);
        }
        Ok(IfStmt {
            branches,
            else_branch,
        })
    }

    fn parse_while(&mut self) -> Result<Stmt, ParseError> {
        self.advance(); // while
        let cond = self.parse_expr()?;
        let body = self.parse_block()?;
        Ok(Stmt::While { cond, body })
    }

    fn parse_for(&mut self) -> Result<Stmt, ParseError> {
        self.advance(); // for
        let var = self.parse_ident()?;
        self.expect(&Token::In, "'in'")?;
        let iter = self.parse_expr()?;
        let body = self.parse_block()?;
        Ok(Stmt::For { var, iter, body })
    }

    fn parse_return(&mut self) -> Result<Stmt, ParseError> {
        self.advance(); // return
        // return 后面如果是 end 或块结束，则无返回值
        if self.check(&Token::End) || self.check(&Token::Eof) {
            Ok(Stmt::Return(None))
        } else {
            // 检查是否是表达式起始
            let e = self.parse_expr()?;
            Ok(Stmt::Return(Some(e)))
        }
    }

    // ===== 表达式（Pratt 解析）=====

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_range()
    }

    /// v0.4.2 P1-1: range 表达式 a..b —— 最低优先级（低于 or），非结合
    /// 带换行守卫：.. 必须与左操作数同行（与跨行 `-` 不当二元减法的规则一致）
    fn parse_range(&mut self) -> Result<Expr, ParseError> {
        let start = self.parse_or()?;
        if !self.is_newline_before() && self.matches(&Token::DotDot) {
            let end = self.parse_or()?;
            return Ok(Expr::Range {
                start: Box::new(start),
                end: Box::new(end),
            });
        }
        Ok(start)
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_and()?;
        while self.matches(&Token::Or) {
            let right = self.parse_and()?;
            left = Expr::Logical {
                op: LogicalOp::Or,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_comparison()?;
        while self.matches(&Token::And) {
            let right = self.parse_comparison()?;
            left = Expr::Logical {
                op: LogicalOp::And,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_pipeline()?;
        loop {
            let op = match self.peek() {
                Token::Eq => BinOp::Eq,
                Token::NotEq => BinOp::NotEq,
                Token::Lt => BinOp::Lt,
                Token::Gt => BinOp::Gt,
                Token::LtEq => BinOp::LtEq,
                Token::GtEq => BinOp::GtEq,
                _ => break,
            };
            self.advance();
            let right = self.parse_pipeline()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    /// `|>` pipeline：left |> right
    /// 优先级介于比较和算术之间（高于比较、低于 + -），左结合
    /// 语义：左侧值作为右侧函数的第一个参数
    ///   x |> f       => f(x)
    ///   x |> f(args) => f(x, args...)
    fn parse_pipeline(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_addition()?;
        while self.matches(&Token::Pipe) {
            let right = self.parse_addition()?;
            left = Expr::Pipe {
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_addition(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_multiplication()?;
        loop {
            let op = match self.peek() {
                Token::Plus => BinOp::Add,
                Token::Minus => {
                    // 跨行的 - 视为一元取负，而非二元减法
                    // 避免 "0\n-1" 被解析为 "0 - 1"
                    if self.is_newline_before() {
                        break;
                    }
                    BinOp::Sub
                }
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplication()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_multiplication(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                Token::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        match self.peek() {
            Token::Bang => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                })
            }
            Token::Minus => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                })
            }
            _ => self.parse_question(),
        }
    }

    /// `?` 后缀运算符：expr? — 比 postfix 低，比 unary 高
    /// `foo(x)?` → Try(Call(foo, x))；`-x?` → Unary(Neg, Try(x))
    fn parse_question(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_postfix()?;
        while self.matches(&Token::Question) {
            expr = Expr::Try(Box::new(expr));
        }
        Ok(expr)
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.peek() {
                Token::LParen => {
                    // 跨行的 ( 不视为函数调用
                    // 避免 "0\n(-1)" 被解析为 "0(-1)"
                    if self.is_newline_before() {
                        break;
                    }
                    self.advance();
                    let args = self.parse_args()?;
                    expr = Expr::Call {
                        callee: Box::new(expr),
                        args,
                    };
                }
                Token::LBracket => {
                    // 跨行的 [ 不视为索引
                    if self.is_newline_before() {
                        break;
                    }
                    self.advance();
                    let index = self.parse_expr()?;
                    self.expect(&Token::RBracket, "']'")?;
                    expr = Expr::Index {
                        expr: Box::new(expr),
                        index: Box::new(index),
                    };
                }
                Token::Dot => {
                    self.advance();
                    // .ident（记录字段）或 .0（元组索引）
                    let name = match self.peek() {
                        Token::Int(n) => {
                            let idx = *n;
                            if idx < 0 {
                                return Err(self.err("元组索引不能为负".to_string()));
                            }
                            self.advance();
                            idx.to_string()
                        }
                        _ => self.parse_ident()?,
                    };
                    expr = Expr::Field {
                        expr: Box::new(expr),
                        name,
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut args = Vec::new();
        if !self.check(&Token::RParen) {
            loop {
                args.push(self.parse_expr()?);
                if !self.matches(&Token::Comma) {
                    break;
                }
            }
        }
        self.expect(&Token::RParen, "')'")?;
        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let tok = self.current().clone();
        match &tok.token {
            Token::Int(n) => {
                self.advance();
                Ok(Expr::Int(*n))
            }
            Token::Float(f) => {
                self.advance();
                Ok(Expr::Float(*f))
            }
            Token::True => {
                self.advance();
                Ok(Expr::Bool(true))
            }
            Token::False => {
                self.advance();
                Ok(Expr::Bool(false))
            }
            Token::Str(s) => {
                self.advance();
                Ok(Expr::Str(s.clone()))
            }
            Token::Ident(s) => {
                self.advance();
                Ok(Expr::Ident(s.clone()))
            }
            Token::LParen => {
                self.advance();
                if self.matches(&Token::RParen) {
                    // () = Unit
                    Ok(Expr::Unit)
                } else {
                    let first = self.parse_expr()?;
                    if self.matches(&Token::Comma) {
                        // tuple: (e1, e2, ...)
                        let mut elems = vec![first];
                        loop {
                            // 允许尾随逗号: (a, b,)
                            if self.check(&Token::RParen) {
                                break;
                            }
                            elems.push(self.parse_expr()?);
                            if !self.matches(&Token::Comma) {
                                break;
                            }
                        }
                        self.expect(&Token::RParen, "')' (闭合元组)")?;
                        Ok(Expr::Tuple { elems })
                    } else {
                        self.expect(&Token::RParen, "')'")?;
                        Ok(Expr::Group(Box::new(first)))
                    }
                }
            }
            Token::LBrace => self.parse_record(),
            Token::Fn => self.parse_closure(),
            Token::If => {
                let if_stmt = self.parse_if()?;
                Ok(Expr::If(Box::new(if_stmt)))
            }
            Token::Match => self.parse_match(),
            _ => Err(self.err(format!(
                "期望表达式，得到 {}",
                self.token_name(self.peek())
            ))),
        }
    }

    fn parse_closure(&mut self) -> Result<Expr, ParseError> {
        self.advance(); // fn
        self.expect(&Token::LParen, "'('")?;
        let params = self.parse_params()?;
        let ret_type = if self.matches(&Token::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        Ok(Expr::Closure {
            params,
            ret_type,
            body: Box::new(body),
        })
    }

    /// 解析结构记录字面量：{x: 3, y: 4}
    /// 字段以逗号分隔，允许尾随逗号
    fn parse_record(&mut self) -> Result<Expr, ParseError> {
        self.advance(); // {
        let mut fields = Vec::new();
        if !self.check(&Token::RBrace) {
            loop {
                // 允许尾随逗号: {x: 1, y: 2,}
                if self.check(&Token::RBrace) {
                    break;
                }
                let name = self.parse_ident()?;
                self.expect(&Token::Colon, "':' (记录字段分隔)")?;
                let value = self.parse_expr()?;
                fields.push((name, value));
                if !self.matches(&Token::Comma) {
                    break;
                }
            }
        }
        self.expect(&Token::RBrace, "'}' (闭合记录)")?;
        Ok(Expr::Record { fields })
    }

    /// 解析 match 表达式：match SCRUTINEE \n PATTERN => BODY \n ... end
    /// match 自身消费 end（不像 if 那样由 parse_block 消费）
    fn parse_match(&mut self) -> Result<Expr, ParseError> {
        self.advance(); // match
        let scrutinee = self.parse_expr()?;
        let mut arms = Vec::new();
        // arms 直到 end / eof
        while !self.check(&Token::End) && !self.check(&Token::Eof) {
            let saved_pos = self.pos;
            match self.parse_match_arm() {
                Ok(arm) => arms.push(arm),
                Err(e) => {
                    if self.recover {
                        self.errors.push(e);
                        // 保守同步：跳到 match 的 end（或 eof），放弃后续臂。
                        // match 臂内含 Form B block（以 end 闭合），跨臂恢复易误吞 end，
                        // 故 Phase 2.2 选择只报该臂错误、丢弃剩余臂，保证 match 自闭合。
                        while !self.check(&Token::End) && !self.check(&Token::Eof) {
                            self.advance();
                        }
                        self.ensure_progress(saved_pos);
                        break;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        // 消费 end，记录其行号（Phase 4.1.2：用于 MAT001 fix 在 end 前插入缺失变体分支）
        let end_line = if self.check(&Token::End) {
            let end_tok = self.advance();
            end_tok.line
        } else if self.recover {
            self.errors
                .push(self.err("期望 'end' 闭合 match".to_string()));
            0 // 缺失 end，无定位
        } else {
            return Err(self.err("期望 'end' 闭合 match".to_string()));
        };
        Ok(Expr::Match(Box::new(MatchExpr {
            scrutinee: Box::new(scrutinee),
            arms,
            end_line,
        })))
    }

    /// 解析 match 分支：PATTERN => BODY
    /// Form A：=> 后同行单表达式
    /// Form B：=> 后跨行 block（以 end 闭合）
    fn parse_match_arm(&mut self) -> Result<MatchArm, ParseError> {
        let pattern = self.parse_pattern()?;
        self.expect(&Token::FatArrow, "'=>'")?;
        let body = if self.is_newline_before() {
            // Form B：跨行 block
            MatchArmBody::Block(self.parse_block()?)
        } else {
            // Form A：同行单表达式
            MatchArmBody::Expr(self.parse_expr()?)
        };
        Ok(MatchArm { pattern, body })
    }

    /// 解析模式
    /// Pattern := Literal | _ | Binder | Variant '(' Patterns ')'
    fn parse_pattern(&mut self) -> Result<Pattern, ParseError> {
        let tok = self.current().clone();
        match &tok.token {
            Token::Int(n) => {
                self.advance();
                Ok(Pattern::Lit(Expr::Int(*n)))
            }
            Token::Float(f) => {
                self.advance();
                Ok(Pattern::Lit(Expr::Float(*f)))
            }
            Token::Str(s) => {
                self.advance();
                Ok(Pattern::Lit(Expr::Str(s.clone())))
            }
            Token::True => {
                self.advance();
                Ok(Pattern::Lit(Expr::Bool(true)))
            }
            Token::False => {
                self.advance();
                Ok(Pattern::Lit(Expr::Bool(false)))
            }
            Token::Ident(s) => {
                self.advance();
                if s == "_" {
                    Ok(Pattern::Wildcard)
                } else if self.check(&Token::LParen) && !self.is_newline_before() {
                    // 变体模式：Name(sub1, sub2, ...)
                    self.advance(); // (
                    let mut subs = Vec::new();
                    if !self.check(&Token::RParen) {
                        loop {
                            subs.push(self.parse_pattern()?);
                            if !self.matches(&Token::Comma) {
                                break;
                            }
                        }
                    }
                    self.expect(&Token::RParen, "')' (闭合变体模式)")?;
                    Ok(Pattern::Variant {
                        name: s.clone(),
                        sub: subs,
                    })
                } else {
                    // 绑定模式（也可能是无参数变体如 None，由解释器区分）
                    Ok(Pattern::Binder(s.clone()))
                }
            }
            _ => Err(self.err(format!(
                "期望模式，得到 {}",
                self.token_name(self.peek())
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(src: &str) -> Program {
        Parser::parse(src).unwrap_or_else(|e| panic!("解析失败: {}", e))
    }

    #[test]
    fn test_fn_decl() {
        let p = parse_ok("fn add(x: Int, y: Int) -> Int\n    x + y\nend");
        assert_eq!(p.items.len(), 1);
    }

    #[test]
    fn test_let_and_call() {
        let src = "fn main() -> Unit\n    let x = add(1, 2)\n    println(x)\nend";
        parse_ok(src);
    }

    #[test]
    fn test_if_elif_else() {
        let src = "fn grade(s: Int) -> Int\n    if s >= 90\n        1\n    elif s >= 80\n        2\n    else\n        3\n    end\nend";
        parse_ok(src);
    }

    #[test]
    fn test_while() {
        let src = "fn main() -> Unit\n    let mut i = 0\n    while i < 10\n        i = i + 1\n    end\nend";
        parse_ok(src);
    }

    #[test]
    fn test_closure() {
        let src = "fn main() -> Unit\n    let f = fn(x: Int) -> Int\n        x * 2\n    end\n    println(f(5))\nend";
        parse_ok(src);
    }

    #[test]
    fn test_if_as_expression() {
        let src = "fn f(n: Int) -> Int\n    let x = if n > 0\n        1\n    else\n        0\n    end\n    x\nend";
        parse_ok(src);
    }

    #[test]
    fn test_newline_unary_minus() {
        // 跨行的 - 应为一元取负，而非二元减法
        // 回归测试：确保 "0\n-1" 不被解析为 "0 - 1"
        let src = "fn sign(n: Int) -> Int\n    if n > 0\n        1\n    elif n < 0\n        -1\n    else\n        0\n    end\nend";
        parse_ok(src);
    }

    #[test]
    fn test_newline_not_call() {
        // 跨行的 ( 不应视为函数调用
        // 回归测试：确保 "0\n(-1)" 不被解析为 "0(-1)"
        let src = "fn f(n: Int) -> Int\n    if n > 0\n        (-1)\n    else\n        0\n    end\nend";
        parse_ok(src);
    }

    #[test]
    fn test_match_literal() {
        let src = "fn f(n: Int) -> Int\n    match n\n        0 => 1\n        _ => 2\n    end\nend";
        parse_ok(src);
    }

    #[test]
    fn test_match_variant() {
        let src = "fn f(x: Result<Int, String>) -> Int\n    match x\n        Ok(n) => n\n        Err(e) => 0\n    end\nend";
        parse_ok(src);
    }

    #[test]
    fn test_match_form_b() {
        // Form B：=> 后跨行 block，以 end 闭合
        let src = "fn f(x: Result<Int, String>) -> Unit\n    match x\n        Ok(n) =>\n            println(n)\n        end\n        Err(e) =>\n            println(e)\n        end\n    end\nend";
        parse_ok(src);
    }

    #[test]
    fn test_enum_single_line() {
        let src = "enum Color = Red | Green | Blue\nfn main() -> Unit\n    println(Red)\nend";
        parse_ok(src);
    }

    #[test]
    fn test_enum_multiline() {
        let src = "enum Shape\n    | Circle(Float)\n    | Square(Float)\nend\nfn main() -> Unit\n    println(Circle(1.0))\nend";
        parse_ok(src);
    }

    #[test]
    fn test_enum_generic() {
        let src = "enum Result<T, E> = Ok(T) | Err(E)\nfn main() -> Unit\n    println(Ok(1))\nend";
        parse_ok(src);
    }

    #[test]
    fn test_pipeline_basic() {
        // x |> f |> g
        let src = "fn f(x: Int) -> Int\n    x + 1\nend\nfn g(x: Int) -> Int\n    x * 2\nend\nfn main() -> Unit\n    let r = 5 |> f |> g\n    println(r)\nend";
        parse_ok(src);
    }

    #[test]
    fn test_pipeline_with_args() {
        // x |> f(y) => f(x, y)
        let src = "fn add(x: Int, y: Int) -> Int\n    x + y\nend\nfn main() -> Unit\n    let r = 10 |> add(3)\n    println(r)\nend";
        parse_ok(src);
    }

    #[test]
    fn test_pipeline_precedence_arithmetic() {
        // 1 + 2 |> f 应解析为 (1 + 2) |> f
        let src = "fn f(x: Int) -> Int\n    x\nend\nfn main() -> Int\n    1 + 2 |> f\nend";
        let p = parse_ok(src);
        // main 的 body tail 应为 Pipe { left: Binary(1+2), right: Ident(f) }
        if let Item::Fn(main) = &p.items[1] {
            if let Some(tail) = &main.body.tail {
                match tail.as_ref() {
                    Expr::Pipe { left, right } => {
                        assert!(matches!(left.as_ref(), Expr::Binary { op: BinOp::Add, .. }));
                        assert!(matches!(right.as_ref(), Expr::Ident(_)));
                    }
                    other => panic!("期望 Pipe，得到 {:?}", other),
                }
            } else {
                panic!("期望有 tail 表达式");
            }
        } else {
            panic!("期望第二个 item 是 fn main");
        }
    }

    #[test]
    fn test_pipeline_precedence_comparison() {
        // x |> f == 3 应解析为 (x |> f) == 3
        let src = "fn f(x: Int) -> Int\n    x\nend\nfn main() -> Bool\n    5 |> f == 3\nend";
        let p = parse_ok(src);
        if let Item::Fn(main) = &p.items[1] {
            if let Some(tail) = &main.body.tail {
                match tail.as_ref() {
                    Expr::Binary { op: BinOp::Eq, left, .. } => {
                        assert!(matches!(left.as_ref(), Expr::Pipe { .. }));
                    }
                    other => panic!("期望 Binary(Eq, Pipe, ...)，得到 {:?}", other),
                }
            } else {
                panic!("期望有 tail 表达式");
            }
        } else {
            panic!("期望第二个 item 是 fn main");
        }
    }

    // ===== Phase 2.1.5 显式导入测试 =====

    #[test]
    fn test_import_basic() {
        // from io import { println }
        let src = "from io import { println }\nfn main() -> Unit\n    println(1)\nend";
        let p = parse_ok(src);
        assert_eq!(p.items.len(), 2);
        match &p.items[0] {
            Item::Import(imp) => {
                assert_eq!(imp.module, "io");
                assert_eq!(imp.items.len(), 1);
                assert_eq!(imp.items[0].name, "println");
                assert_eq!(imp.items[0].alias, "println"); // 无 as 时 alias == name
            }
            other => panic!("期望 Item::Import，得到 {:?}", other),
        }
    }

    #[test]
    fn test_import_with_alias() {
        // from io import { println as log }
        let src = "from io import { println as log }\nfn main() -> Unit\n    log(1)\nend";
        let p = parse_ok(src);
        match &p.items[0] {
            Item::Import(imp) => {
                assert_eq!(imp.module, "io");
                assert_eq!(imp.items.len(), 1);
                assert_eq!(imp.items[0].name, "println");
                assert_eq!(imp.items[0].alias, "log");
            }
            other => panic!("期望 Item::Import，得到 {:?}", other),
        }
    }

    #[test]
    fn test_import_multiple_items() {
        // from string import { len, upper, lower }
        let src =
            "from string import { len, upper, lower }\nfn main() -> Unit\n    println(1)\nend";
        let p = parse_ok(src);
        match &p.items[0] {
            Item::Import(imp) => {
                assert_eq!(imp.module, "string");
                assert_eq!(imp.items.len(), 3);
                assert_eq!(imp.items[0].name, "len");
                assert_eq!(imp.items[1].name, "upper");
                assert_eq!(imp.items[2].name, "lower");
            }
            other => panic!("期望 Item::Import，得到 {:?}", other),
        }
    }

    #[test]
    fn test_import_mixed_alias() {
        // 混合：部分有别名，部分无
        let src = "from math import { sqrt, abs as absolute, max }\nfn main() -> Unit\n    println(1)\nend";
        let p = parse_ok(src);
        match &p.items[0] {
            Item::Import(imp) => {
                assert_eq!(imp.module, "math");
                assert_eq!(imp.items.len(), 3);
                assert_eq!(imp.items[0].name, "sqrt");
                assert_eq!(imp.items[0].alias, "sqrt");
                assert_eq!(imp.items[1].name, "abs");
                assert_eq!(imp.items[1].alias, "absolute");
                assert_eq!(imp.items[2].name, "max");
                assert_eq!(imp.items[2].alias, "max");
            }
            other => panic!("期望 Item::Import，得到 {:?}", other),
        }
    }

    #[test]
    fn test_import_dotted_module() {
        // 点分模块路径：from utils.helpers import { format_date }
        // Phase 2.1.5 parser 接受任意点分路径；语义检查（模块是否存在）由 interpreter 负责
        let src = "from utils.helpers import { format_date }\nfn main() -> Unit\n    println(1)\nend";
        let p = parse_ok(src);
        match &p.items[0] {
            Item::Import(imp) => {
                assert_eq!(imp.module, "utils.helpers");
                assert_eq!(imp.items[0].name, "format_date");
            }
            other => panic!("期望 Item::Import，得到 {:?}", other),
        }
    }

    #[test]
    fn test_import_trailing_comma() {
        // 尾随逗号允许
        let src = "from io import { println, print, }\nfn main() -> Unit\n    println(1)\nend";
        let p = parse_ok(src);
        match &p.items[0] {
            Item::Import(imp) => {
                assert_eq!(imp.items.len(), 2);
            }
            other => panic!("期望 Item::Import，得到 {:?}", other),
        }
    }

    #[test]
    fn test_import_before_fn() {
        // import 必须能出现在 fn 之前
        let src = "from io import { println }\nfrom string import { len }\nfn main() -> Unit\n    println(len(\"hi\"))\nend";
        let p = parse_ok(src);
        assert_eq!(p.items.len(), 3);
        assert!(matches!(p.items[0], Item::Import(_)));
        assert!(matches!(p.items[1], Item::Import(_)));
        assert!(matches!(p.items[2], Item::Fn(_)));
    }

    // ===== Phase 2.2 容错解析器测试 =====

    fn recover(src: &str) -> ParseResult {
        Parser::parse_recover(src)
    }

    #[test]
    fn test_recover_valid_no_errors() {
        // 合法程序：容错模式应返回 0 错误、干净 AST
        let r = recover("fn add(x: Int, y: Int) -> Int\n    x + y\nend");
        assert!(r.is_ok(), "合法程序不应有错误，得到: {:?}", r.errors);
        assert_eq!(r.program.items.len(), 1);
    }

    #[test]
    fn test_recover_strict_returns_first_error() {
        // 严格模式 parse() 仍返回首个错误（向后兼容）
        let src = "fn main() -> Unit\n    let x =\nend";
        assert!(Parser::parse(src).is_err());
    }

    #[test]
    fn test_recover_bad_statement_inserts_hole_and_keeps_fn() {
        // 函数体中一条语句错误：插入 Hole，函数仍在 items 中，后续语句继续解析
        // `let x = +` 中 + 后无操作数 → 错误；之后的 println(1) 应被恢复解析
        let src = "fn main() -> Unit\n    let x = +\n    println(1)\nend";
        let r = recover(src);
        assert!(!r.is_ok(), "应收集到错误，errors={:?}", r.errors);
        assert_eq!(r.program.items.len(), 1, "函数应保留在 items 中");
        match &r.program.items[0] {
            Item::Fn(f) => {
                // 应在错误处插入 Hole
                let has_hole = f.body.stmts.iter().any(|s| matches!(s, Stmt::Hole { .. }));
                assert!(has_hole, "应在错误处插入 Stmt::Hole，stmts={:?}", f.body.stmts);
                // println(1) 应被恢复（作为语句或 tail）
                let has_println_stmt = f.body.stmts.iter().any(|s| {
                    matches!(s, Stmt::Expr(Expr::Call { .. }))
                });
                let tail_is_call = matches!(
                    f.body.tail.as_ref().map(|t| t.as_ref()),
                    Some(Expr::Call { .. })
                );
                assert!(
                    has_println_stmt || tail_is_call,
                    "println(1) 应被恢复解析，stmts={:?} tail={:?}",
                    f.body.stmts,
                    f.body.tail
                );
            }
            other => panic!("期望 Item::Fn，得到 {:?}", other),
        }
    }

    #[test]
    fn test_recover_multiple_bad_items_collects_all_errors() {
        // 两个顶层 item 均非法：应收集到错误（≥1），且解析不崩溃
        // 第一行 `fn 123` 非法（函数名不是标识符），第二行 `enum` 无名
        let src = "fn 123 () -> Unit\n    1\nend\nenum\nend";
        let r = recover(src);
        assert!(
            !r.is_ok(),
            "应收集到至少一个错误，errors={:?}",
            r.errors
        );
        // 程序结构仍存在（可能 items 为空或部分）
        let _ = &r.program.items;
    }

    #[test]
    fn test_recover_bad_item_keeps_surrounding_valid_items() {
        // 中间一个非法 item，前后合法 item 应被保留
        let src = "fn a() -> Unit\n    1\nend\n@@@\nfn b() -> Unit\n    2\nend";
        let r = recover(src);
        assert!(!r.is_ok(), "应收集到词法/语法错误");
        // a 和 b 两个函数应被保留
        let fn_names: Vec<&str> = r
            .program
            .items
            .iter()
            .filter_map(|it| match it {
                Item::Fn(f) => Some(f.name.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            fn_names.contains(&"a") && fn_names.contains(&"b"),
            "a 和 b 应被保留，实际: {:?}",
            fn_names
        );
    }

    #[test]
    fn test_recover_lex_error_collected() {
        // 非法字符 @ 触发词法错误，应被收集而非终止
        let src = "fn main() -> Unit\n    let x = @\n    println(1)\nend";
        let r = recover(src);
        assert!(!r.is_ok(), "应收集到词法错误");
        // 函数结构仍保留
        assert_eq!(r.program.items.len(), 1);
    }

    #[test]
    fn test_recover_match_bad_arm_keeps_match_self_closed() {
        // match 臂错误：match 应自闭合（消费自己的 end），后续语句继续解析
        // 第二条臂缺 => ：错误，丢弃剩余臂
        let src = "fn main() -> Unit\n    match 1\n        0 => println(\"z\")\n        1\n    end\n    println(\"after\")\nend";
        let r = recover(src);
        assert!(!r.is_ok(), "应收集到 match 臂错误");
        match &r.program.items[0] {
            Item::Fn(f) => {
                // match 之后应有 println("after") 语句被恢复
                let has_after = f.body.stmts.iter().any(|s| {
                    matches!(
                        s,
                        Stmt::Expr(Expr::Call { .. })
                    )
                });
                assert!(
                    has_after || f.body.tail.is_some(),
                    "match 之后的语句应被恢复，stmts={:?} tail={:?}",
                    f.body.stmts,
                    f.body.tail
                );
            }
            other => panic!("期望 Item::Fn，得到 {:?}", other),
        }
    }
}
