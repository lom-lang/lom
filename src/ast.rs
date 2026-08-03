// Lom AST — Phase 2 抽象语法树定义
// Phase 2 新增：match, enum, Result/Option, 模式匹配, ?, |>, 结构记录, 元组, 显式导入
// Phase 2.2 新增：Hole 节点（容错解析器在错误处插入的占位符）
// Phase 3.2 新增：Span 类型 + FnDecl/EnumDecl span 字段（函数/枚举级诊断精确定位）
// Phase 2 仍不含：多文件模块（Phase 3）、表达式级 span（Phase 3.2b）

/// 源码位置跨度（1-based，与 lexer 的 SpannedToken 一致）
///
/// Phase 3.2a：仅 FnDecl/EnumDecl 携带 span（函数/枚举级诊断定位）。
/// Phase 3.2b：Expr 各变体将携带 span（表达式级诊断 + LSP）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

impl Span {
    /// 构造一个起始位置 span（end = start，用于单点定位）
    pub fn at(line: usize, col: usize) -> Self {
        Span { line, col, end_line: line, end_col: col }
    }
}

/// 程序：顶层声明列表
#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

/// 顶层 item
#[derive(Debug, Clone)]
pub enum Item {
    Fn(FnDecl),
    Enum(EnumDecl),
    /// 显式导入：from mod import { name1, name2 as alias }
    /// Phase 2.1.5：仅标准库模块（io/string/math），多文件模块留 Phase 3
    Import(ImportDecl),
}

/// 导入声明：from <module> import { <items> }
/// module 为点分路径（如 "utils.helpers"），Phase 2.1.5 仅识别标准库模块名
#[derive(Debug, Clone)]
pub struct ImportDecl {
    /// 模块路径（点分，如 "io"、"string"、"math"）
    pub module: String,
    /// 导入项列表
    pub items: Vec<ImportItem>,
}

/// 单个导入项：name 或 name as alias
#[derive(Debug, Clone)]
pub struct ImportItem {
    /// 模块中导出的原始符号名
    pub name: String,
    /// 导入后的本地别名（未指定 as 时与 name 相同）
    pub alias: String,
}

/// 函数声明：fn name(params) -> ret_type ! [effects] body
/// Phase 2.5 新增 effects 字段（显式效应系统）
/// Phase 3.2 新增 span 字段（函数签名位置，用于 EFF001/TYPE010/NAM002 诊断定位）
#[derive(Debug, Clone)]
pub struct FnDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub ret_type: Option<Type>,
    /// 显式效应列表：`! [IO, Clock]` 中的标识符列表
    /// 空 Vec 表示纯函数（无 `! [...]` 注解）
    pub effects: Vec<Effect>,
    pub body: Block,
    /// Phase 3.2: 函数签名 span（`fn` 关键字到签名行末）
    /// 用于 EFF001（效应注解插入位置）、TYPE010（返回类型不符）、NAM002（重复定义）诊断
    pub span: Span,
}

/// 效应名：标识符（如 IO、Clock、State、Network）
/// Phase 2.5 中效应仅为编译期注解，不做运行时跟踪
pub type Effect = String;

/// 函数参数：name: type
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

/// 枚举声明：enum Name<T, E> = Variant1(T) | Variant2(E) | Variant3
/// Phase 3.2 新增 span 字段（枚举名位置，用于 NAM002 重复定义诊断）
#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub name: String,
    pub type_params: Vec<String>,
    pub variants: Vec<EnumVariantDecl>,
    /// Phase 3.2: 枚举签名 span（`enum` 关键字到 `=` 前）
    pub span: Span,
}

/// 枚举变体：Name 或 Name(types)
#[derive(Debug, Clone)]
pub struct EnumVariantDecl {
    pub name: String,
    pub fields: Vec<Type>,
}

/// 类型注解
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    String,
    Unit,
    Named(String),                  // 用户定义类型名（含枚举名）
    Option(Box<Type>),              // Option<T>
    Result(Box<Type>, Box<Type>),   // Result<T, E>
    Generic(String, Vec<Type>),     // 用户泛型类型 Name<args>
    /// 结构记录类型：{x: Int, y: Int}
    Record(Vec<(String, Type)>),
    /// 元组类型：(Int, String)
    Tuple(Vec<Type>),
}

impl Type {
    /// 从标识符解析类型
    pub fn from_name(name: &str) -> Type {
        match name {
            "Int" => Type::Int,
            "Float" => Type::Float,
            "Bool" => Type::Bool,
            "String" => Type::String,
            "Unit" => Type::Unit,
            _ => Type::Named(name.to_string()),
        }
    }
}

/// 语句块：语句列表 + 可选尾表达式，由 end 闭合
#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    /// 尾表达式（块的值）；如果块以语句结尾则为 None
    pub tail: Option<Box<Expr>>,
}

/// 语句
#[derive(Debug, Clone)]
pub enum Stmt {
    /// let [mut] name [: type] = expr
    Let {
        mutable: bool,
        name: String,
        ty: Option<Type>,
        value: Expr,
    },
    /// 赋值：name = expr（name 必须是已声明的 mut 变量）
    Assign { target: String, value: Expr },
    /// if/elif/else 块（作为语句；也可是表达式）
    If(IfStmt),
    /// while expr block end
    While { cond: Expr, body: Block },
    /// for x in expr block end
    For { var: String, iter: Expr, body: Block },
    /// return [expr]
    Return(Option<Expr>),
    /// 裸表达式语句
    Expr(Expr),
    /// 代码洞（Phase 2.2 容错解析）：该处解析失败，解析器插入占位符
    /// 携带错误发生的位置，便于诊断；解释器遇到时报运行时错误
    Hole { line: usize, col: usize },
}

/// if 语句/表达式
#[derive(Debug, Clone)]
pub struct IfStmt {
    pub branches: Vec<(Expr, Block)>, // if + elif 分支
    pub else_branch: Option<Block>,
}

/// 表达式
#[derive(Debug, Clone)]
pub enum Expr {
    /// 整数字面量
    Int(i64),
    /// 浮点字面量
    Float(f64),
    /// 布尔字面量
    Bool(bool),
    /// 字符串字面量
    Str(String),
    /// Unit 字面量 ()
    Unit,
    /// 标识符引用
    Ident(String),
    /// 二元运算：a op b
    Binary { op: BinOp, left: Box<Expr>, right: Box<Expr> },
    /// 一元运算：op a
    Unary { op: UnaryOp, expr: Box<Expr> },
    /// 逻辑运算：and / or（短路）
    Logical { op: LogicalOp, left: Box<Expr>, right: Box<Expr> },
    /// 函数调用：callee(args)
    Call { callee: Box<Expr>, args: Vec<Expr> },
    /// 索引：expr[index]
    Index { expr: Box<Expr>, index: Box<Expr> },
    /// 字段访问：expr.name
    Field { expr: Box<Expr>, name: String },
    /// 分组：(expr)
    Group(Box<Expr>),
    /// if 表达式（作为值使用）
    If(Box<IfStmt>),
    /// 闭包：fn(params) -> type block
    Closure { params: Vec<Param>, ret_type: Option<Type>, body: Box<Block> },
    /// match 表达式
    Match(Box<MatchExpr>),
    /// `?` 错误传播：expr? — Ok/Some 解包，Err/None 提前返回
    Try(Box<Expr>),
    /// `|>` pipeline：left |> right
    /// 语义：把 left 求值后作为 right 的第一个参数
    /// - left |> f       => f(left)
    /// - left |> f(args) => f(left, args...)
    /// 左结合，优先级介于比较和算术之间（高于比较、低于 + -）
    Pipe { left: Box<Expr>, right: Box<Expr> },
    /// 结构记录字面量：{x: 3, y: 4}
    /// 字段顺序保留（用 Vec 而非 HashMap），便于显示和结构等价比较
    Record { fields: Vec<(String, Expr)> },
    /// 元组字面量：(1, "hello")
    /// 单元素不是元组：(x) 是 Group；空元组 () 是 Unit
    Tuple { elems: Vec<Expr> },
}

/// match 表达式：match scrutinee { arms }
#[derive(Debug, Clone)]
pub struct MatchExpr {
    pub scrutinee: Box<Expr>,
    pub arms: Vec<MatchArm>,
}

/// match 分支：pattern => body
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: MatchArmBody,
}

/// match 分支体
#[derive(Debug, Clone)]
pub enum MatchArmBody {
    /// Form A：单表达式（=> 后同行）
    Expr(Expr),
    /// Form B：块（=> 后跨行，以 end 闭合）
    Block(Block),
}

/// 模式
#[derive(Debug, Clone)]
pub enum Pattern {
    /// 字面量模式：0, "hi", True, 3.14
    Lit(Expr),
    /// 变量绑定：name（匹配任意值并绑定）
    Binder(String),
    /// 通配符：_
    Wildcard,
    /// 枚举变体模式：Name(sub1, sub2) 或 Name（无参数）
    Variant { name: String, sub: Vec<Pattern> },
}

/// 二元运算符
#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,    // +
    Sub,    // -
    Mul,    // *
    Div,    // /
    Mod,    // %
    Eq,     // ==
    NotEq,  // !=
    Lt,     // <
    Gt,     // >
    LtEq,   // <=
    GtEq,   // >=
}

/// 一元运算符
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg, // -
    Not, // !
}

/// 逻辑运算符
#[derive(Debug, Clone, PartialEq)]
pub enum LogicalOp {
    And,
    Or,
}
