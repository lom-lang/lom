// Lom Interpreter — Phase 2 树遍历解释器
// 动态类型，运行时检查；不做编译时类型检查（Phase 2.4 才加）
// Phase 2 新增：match, enum, Result/Option, 模式匹配

use crate::ast::*;
use crate::parser::Parser;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::rc::Rc;

/// 运行时值
#[derive(Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Unit,
    /// 闭包：参数 + 函数体 + 捕获的环境
    Closure {
        params: Vec<Param>,
        body: Block,
        env: ScopeRef,
    },
    /// 枚举变体值：Ok(v), Err(e), Some(v), None, 用户变体
    Enum {
        variant: String,
        args: Vec<Value>,
    },
    /// 结构记录值：{x: 3, y: 4}
    /// 字段顺序保留（结构等价比较时顺序不敏感，但显示时按声明顺序）
    Record {
        fields: Vec<(String, Value)>,
    },
    /// 元组值：(1, "hello")
    Tuple {
        elems: Vec<Value>,
    },
    /// Phase 3.3: 列表值（不可变语义，函数返回新 List）
    /// 用于 JSON 数组映射和未来集合模块的基础
    List {
        elems: Vec<Value>,
    },
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Str(s) => write!(f, "\"{}\"", s),
            Value::Unit => write!(f, "()"),
            Value::Closure { .. } => write!(f, "<闭包>"),
            Value::Enum { variant, args } => {
                if args.is_empty() {
                    write!(f, "{}", variant)
                } else {
                    write!(f, "{}(", variant)?;
                    for (i, a) in args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{:?}", a)?;
                    }
                    write!(f, ")")
                }
            }
            Value::Record { fields } => {
                write!(f, "{{")?;
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {:?}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Tuple { elems } => {
                write!(f, "(")?;
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", e)?;
                }
                // 单元素元组需要尾随逗号以区分于 Group：(1,)
                if elems.len() == 1 {
                    write!(f, ",")?;
                }
                write!(f, ")")
            }
            Value::List { elems } => {
                write!(f, "[")?;
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", e)?;
                }
                write!(f, "]")
            }
        }
    }
}

impl Value {
    /// 类型名（用于错误信息）
    fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "Int",
            Value::Float(_) => "Float",
            Value::Bool(_) => "Bool",
            Value::Str(_) => "String",
            Value::Unit => "Unit",
            Value::Closure { .. } => "闭包",
            Value::Enum { .. } => "枚举变体",
            Value::Record { .. } => "记录",
            Value::Tuple { .. } => "元组",
            Value::List { .. } => "List",
        }
    }

    /// 真值（用于 if/while 条件）
    fn is_truthy(&self) -> Result<bool, RuntimeError> {
        match self {
            Value::Bool(b) => Ok(*b),
            _ => Err(RuntimeError::Msg(format!(
                "期望 Bool，得到 {}",
                self.type_name()
            ))),
        }
    }

    /// 转字符串（用于 println 和字符串拼接）
    fn to_display(&self) -> String {
        match self {
            Value::Int(n) => n.to_string(),
            Value::Float(n) => {
                // 整数浮点显示为 x.0
                let s = n.to_string();
                if s.contains('.') {
                    s
                } else {
                    format!("{}.0", s)
                }
            }
            Value::Bool(b) => b.to_string(),
            Value::Str(s) => s.clone(),
            Value::Unit => "()".to_string(),
            Value::Closure { .. } => "<闭包>".to_string(),
            Value::Enum { variant, args } => {
                if args.is_empty() {
                    variant.clone()
                } else {
                    let parts: Vec<String> = args.iter().map(|a| a.to_display()).collect();
                    format!("{}({})", variant, parts.join(", "))
                }
            }
            Value::Record { fields } => {
                let parts: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v.to_display()))
                    .collect();
                format!("{{{}}}", parts.join(", "))
            }
            Value::Tuple { elems } => {
                let parts: Vec<String> = elems.iter().map(|e| e.to_display()).collect();
                if elems.len() == 1 {
                    format!("({},)", parts[0])
                } else {
                    format!("({})", parts.join(", "))
                }
            }
            Value::List { elems } => {
                let parts: Vec<String> = elems.iter().map(|e| e.to_display()).collect();
                format!("[{}]", parts.join(", "))
            }
        }
    }
}

/// 作用域：变量绑定 + 父作用域
type ScopeRef = Rc<RefCell<Scope>>;

#[derive(Default)]
struct Scope {
    vars: HashMap<String, Value>,
    parent: Option<ScopeRef>,
}

impl Scope {
    fn new(parent: Option<ScopeRef>) -> ScopeRef {
        Rc::new(RefCell::new(Scope {
            vars: HashMap::new(),
            parent,
        }))
    }

    fn get(&self, name: &str) -> Option<Value> {
        if let Some(v) = self.vars.get(name) {
            Some(v.clone())
        } else if let Some(p) = &self.parent {
            p.borrow().get(name)
        } else {
            None
        }
    }

    fn set_existing(&mut self, name: &str, val: Value) -> bool {
        if self.vars.contains_key(name) {
            self.vars.insert(name.to_string(), val);
            true
        } else if let Some(p) = &self.parent {
            p.borrow_mut().set_existing(name, val)
        } else {
            false
        }
    }

    fn define(&mut self, name: String, val: Value) {
        self.vars.insert(name, val);
    }
}

/// 运行时错误
#[derive(Debug)]
pub enum RuntimeError {
    /// 普通错误信息
    Msg(String),
    /// `?` 触发的提前返回：携带应从函数返回的值（Err(e) 或 None）
    EarlyReturn(Value),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::Msg(s) => write!(f, "运行时错误: {}", s),
            RuntimeError::EarlyReturn(v) => write!(f, "提前返回: {}", v.to_display()),
        }
    }
}

impl std::error::Error for RuntimeError {}

/// 控制流信号（return）
enum ControlFlow {
    Normal(Value),
    Return(Value),
}

/// 解释器
pub struct Interpreter {
    /// 全局函数表
    functions: HashMap<String, FnDecl>,
    /// 全局作用域（内置函数和全局变量）
    globals: ScopeRef,
    /// 所有枚举变体名（内置 Ok/Err/Some/None + 用户定义）
    /// 用于在 Call/Ident 中区分"变体构造"与"函数调用/变量引用"
    variants: HashSet<String>,
    /// 无参数变体名（如 None）：作为 Ident 直接求值为 Enum
    nullary_variants: HashSet<String>,
    /// 标准库模块表：模块名 → 导出符号列表
    /// Phase 2.1.5 仅静态注册，多文件用户模块留 Phase 3
    modules: HashMap<&'static str, Vec<&'static str>>,
    /// 当前可用的内置函数名/别名（prelude 自动注入 + 显式导入注入）
    /// 不在此集合中的内置函数调用时报"未导入"错误
    available_builtins: HashSet<String>,
    /// 导入别名 → 真实内置函数名
    /// 例：导入 `println as log` 后，import_aliases["log"] = "println"
    import_aliases: HashMap<String, String>,
    /// Phase 4.4: 外部包表（已加载）
    /// key = 包名（lom.toml dependencies 中的键名）
    packages: HashMap<String, crate::package::ResolvedPackage>,
    /// Phase 3.5: 程序参数（argv），供 env::args() 读取
    /// argv[0] = .lom 文件路径，argv[1..] = -- 之后的用户参数
    program_args: Vec<String>,
}

/// 内置变体
const BUILTIN_VARIANTS: &[(&str, usize)] = &[
    ("Ok", 1),
    ("Err", 1),
    ("Some", 1),
    ("None", 0),
];

/// Prelude：自动导入的符号（无需显式 from io import {println}）
/// 保持向后兼容：现有代码不写 import 也能用 println/print
const PRELUDE: &[&str] = &["println", "print"];

/// 标准库模块定义：模块名 → 导出符号
/// Phase 2.1.5：io / string / math 三个模块
/// Phase 3.3：新增 list / json 模块
/// Phase 3.4：string 扩展（split/contains/replace/starts_with/ends_with），新增 file 模块
/// Phase 3.5：新增 env 模块（args 函数读取命令行参数）
const STDL_MODULES: &[(&str, &[&str])] = &[
    ("io", &["println", "print"]),
    (
        "string",
        &[
            "len",
            "int_to_string",
            "string_to_int",
            "trim",
            "upper",
            "lower",
            "split",
            "contains",
            "replace",
            "starts_with",
            "ends_with",
        ],
    ),
    ("math", &["sqrt", "abs", "min", "max"]),
    (
        "list",
        &[
            "list_empty",
            "list_length",
            "list_get",
            "list_is_empty",
            "list_head",
            "list_tail",
            "list_cons",
        ],
    ),
    ("json", &["json_parse", "json_stringify"]),
    (
        "file",
        &["file_read", "file_write", "file_append", "file_exists"],
    ),
    ("env", &["args"]),
];

impl Interpreter {
    pub fn new() -> Self {
        let globals = Scope::new(None);
        let mut variants = HashSet::new();
        let mut nullary_variants = HashSet::new();
        for (name, arity) in BUILTIN_VARIANTS {
            variants.insert(name.to_string());
            if *arity == 0 {
                nullary_variants.insert(name.to_string());
            }
        }
        // 注册标准库模块
        let mut modules = HashMap::new();
        for (name, exports) in STDL_MODULES {
            modules.insert(*name, exports.to_vec());
        }
        // prelude 自动可用
        let mut available_builtins = HashSet::new();
        for name in PRELUDE {
            available_builtins.insert(name.to_string());
        }
        Interpreter {
            functions: HashMap::new(),
            globals,
            variants,
            nullary_variants,
            modules,
            available_builtins,
            import_aliases: HashMap::new(),
            packages: HashMap::new(),
            program_args: Vec::new(),
        }
    }

    /// Phase 3.5: 设置程序参数（供 env::args() 读取）
    /// argv[0] 约定为 .lom 文件路径，argv[1..] 为用户参数
    pub fn set_program_args(&mut self, args: Vec<String>) {
        self.program_args = args;
    }

    /// Phase 4.4: 加载依赖图中的所有外部包
    ///
    /// 对每个依赖包：
    ///   1. 解析包内所有 .lom 源码文件
    ///   2. 把顶层 fn 注册到 self.functions
    ///   3. 把枚举变体注册到 self.variants / self.nullary_variants
    ///   4. 把包名 + public_symbols 存入 self.packages，供 process_import 查找
    ///
    /// 符号冲突策略：外部包符号直接注册，与本地符号同命名空间。
    /// 冲突时后注册者覆盖（与本地重复定义行为一致）。LLM 责任保证不同包不重名。
    pub fn load_packages(&mut self, graph: &crate::package::DependencyGraph) {
        for (name, pkg) in &graph.packages {
            // 解析并注册包内每个 .lom 文件的 fn/enum
            for file in &pkg.source_files {
                if let Ok(src) = std::fs::read_to_string(file) {
                    let result = Parser::parse_recover(&src);
                    for item in &result.program.items {
                        match item {
                            Item::Fn(f) => {
                                self.functions.insert(f.name.clone(), f.clone());
                            }
                            Item::Enum(e) => {
                                for v in &e.variants {
                                    self.variants.insert(v.name.clone());
                                    if v.fields.is_empty() {
                                        self.nullary_variants.insert(v.name.clone());
                                    }
                                }
                            }
                            Item::Import(_) => {} // 包内 import 暂不传递
                        }
                    }
                }
            }
            // 注册包元数据（供 process_import 查找符号）
            self.packages.insert(name.clone(), pkg.clone());
        }
    }

    /// 运行程序
    pub fn run(&mut self, program: &Program) -> Result<(), RuntimeError> {
        // 先处理所有导入声明（在注册函数/枚举之前，确保符号可用性语义清晰）
        for item in &program.items {
            if let Item::Import(imp) = item {
                self.process_import(imp)?;
            }
        }
        // 注册所有顶层函数 + 收集枚举变体
        for item in &program.items {
            match item {
                Item::Fn(f) => {
                    self.functions.insert(f.name.clone(), f.clone());
                }
                Item::Enum(e) => {
                    for v in &e.variants {
                        self.variants.insert(v.name.clone());
                        if v.fields.is_empty() {
                            self.nullary_variants.insert(v.name.clone());
                        }
                    }
                }
                Item::Import(_) => {} // 已在前面的循环处理
            }
        }
        // 查找并执行 main
        if let Some(main) = self.functions.get("main").cloned() {
            let env = Scope::new(Some(self.globals.clone()));
            self.exec_block(&main.body, env)?;
        }
        Ok(())
    }

    /// Phase 4.2: REPL 增量执行单个 item
    ///
    /// 注册 fn/enum/import 到 interpreter，不立即执行。
    /// 返回 Unit（顶层声明无求值结果）。
    /// 后续 REPL 调用 exec_repl_block 执行语句/表达式时，已注册的函数/枚举可见。
    pub fn exec_item(&mut self, item: &Item) -> Result<Value, RuntimeError> {
        match item {
            Item::Import(imp) => {
                self.process_import(imp)?;
                Ok(Value::Unit)
            }
            Item::Fn(f) => {
                self.functions.insert(f.name.clone(), f.clone());
                Ok(Value::Unit)
            }
            Item::Enum(e) => {
                for v in &e.variants {
                    self.variants.insert(v.name.clone());
                    if v.fields.is_empty() {
                        self.nullary_variants.insert(v.name.clone());
                    }
                }
                Ok(Value::Unit)
            }
        }
    }

    /// Phase 4.2: REPL 在全局环境执行块（语句 + 尾表达式）
    ///
    /// 用于 REPL 模式执行 let 语句或表达式：
    ///   - let x = 5 → 绑定 x 到全局环境，返回绑定的值
    ///   - 1 + 2 → 求值返回 3
    ///   - println("hi") → 执行副作用返回 Unit
    ///
    /// 与 exec_block 的区别：在 globals 作用域直接执行，let 绑定持久保留。
    pub fn exec_repl_block(&mut self, block: &Block) -> Result<Value, RuntimeError> {
        // 在全局环境上创建子作用域执行，let 绑定通过 define 写入 globals
        // 但子作用域的 let 会在子作用域，不持久 — 需要直接在 globals 上执行
        let env = self.globals.clone();
        match self.exec_block(block, env)? {
            ControlFlow::Normal(v) => Ok(v),
            ControlFlow::Return(v) => Ok(v),
        }
    }

    /// 处理一条导入声明：把导入符号（含别名）注入可用集合
    /// - 模块不存在 → 报错
    /// - 符号不在模块导出列表 → 报错
    /// - 重复导入（同别名） → 静默覆盖（与显式重新声明一致，简化处理）
    fn process_import(&mut self, imp: &ImportDecl) -> Result<(), RuntimeError> {
        // 优先查标准库模块
        if let Some(exports) = self.modules.get(imp.module.as_str()) {
            for item in &imp.items {
                // 检查符号是否在该模块导出
                if !exports.iter().any(|e| *e == item.name) {
                    return Err(RuntimeError::Msg(format!(
                        "模块 '{}' 不导出符号 '{}'",
                        imp.module, item.name
                    )));
                }
                // 注册别名映射（alias → 真实名）
                if item.alias != item.name {
                    self.import_aliases
                        .insert(item.alias.clone(), item.name.clone());
                }
                // 标记别名/本名为可用
                self.available_builtins.insert(item.alias.clone());
            }
            return Ok(());
        }

        // Phase 4.4: 查外部包
        if let Some(pkg) = self.packages.get(imp.module.as_str()) {
            for item in &imp.items {
                // 检查符号是否在包的公开符号中
                if !pkg.public_symbols.contains(&item.name) {
                    return Err(RuntimeError::Msg(format!(
                        "包 '{}' 不导出符号 '{}'（PKG006）",
                        imp.module, item.name
                    )));
                }
                // 外部包符号已在 load_packages 注册到 functions/variants
                // 这里只需标记为可用（alias → 本名映射）
                if item.alias != item.name {
                    self.import_aliases
                        .insert(item.alias.clone(), item.name.clone());
                }
                self.available_builtins.insert(item.alias.clone());
            }
            return Ok(());
        }

        // 既不是标准库也不是外部包
        Err(RuntimeError::Msg(format!(
            "未知模块/包 '{}'（标准库：io/string/math/list/json/file；外部包需在 lom.toml dependencies 声明，PKG005）",
            imp.module
        )))
    }

    /// 执行块，返回块的值
    /// 捕获 `?` 触发的 EarlyReturn，转为 ControlFlow::Return（穿透嵌套 block 到函数边界）
    fn exec_block(&mut self, block: &Block, env: ScopeRef) -> Result<ControlFlow, RuntimeError> {
        for stmt in &block.stmts {
            match self.exec_stmt(stmt, env.clone()) {
                Ok(ControlFlow::Return(v)) => return Ok(ControlFlow::Return(v)),
                Ok(ControlFlow::Normal(_)) => {}
                Err(RuntimeError::EarlyReturn(v)) => return Ok(ControlFlow::Return(v)),
                Err(e) => return Err(e),
            }
        }
        // 尾表达式
        let val = match &block.tail {
            Some(e) => match self.eval_expr(e, env) {
                Ok(v) => v,
                Err(RuntimeError::EarlyReturn(v)) => return Ok(ControlFlow::Return(v)),
                Err(e) => return Err(e),
            },
            None => Value::Unit,
        };
        Ok(ControlFlow::Normal(val))
    }

    /// 执行语句
    fn exec_stmt(&mut self, stmt: &Stmt, env: ScopeRef) -> Result<ControlFlow, RuntimeError> {
        match stmt {
            Stmt::Let {
                name, value, ..
            } => {
                let v = self.eval_expr(value, env.clone())?;
                env.borrow_mut().define(name.clone(), v);
                Ok(ControlFlow::Normal(Value::Unit))
            }
            Stmt::LetDestruct { names, value } => {
                // Phase 5.1: 元组解构绑定 let (a, b) = expr
                let v = self.eval_expr(value, env.clone())?;
                let elems = match v {
                    Value::Tuple { elems } => elems,
                    other => {
                        return Err(RuntimeError::Msg(format!(
                            "元组解构要求右侧是元组，得到 {:?}",
                            other.type_name()
                        )))
                    }
                };
                if elems.len() != names.len() {
                    return Err(RuntimeError::Msg(format!(
                        "元组解构数量不匹配：模式含 {} 个名字，值含 {} 个元素",
                        names.len(),
                        elems.len()
                    )));
                }
                for (name, elem) in names.iter().zip(elems.into_iter()) {
                    env.borrow_mut().define(name.clone(), elem);
                }
                Ok(ControlFlow::Normal(Value::Unit))
            }
            Stmt::Assign { target, value } => {
                let v = self.eval_expr(value, env.clone())?;
                if !env.borrow_mut().set_existing(target, v) {
                    return Err(RuntimeError::Msg(format!(
                        "赋值给未定义变量: '{}'",
                        target
                    )));
                }
                Ok(ControlFlow::Normal(Value::Unit))
            }
            Stmt::If(if_stmt) => {
                for (cond, body) in &if_stmt.branches {
                    let c = self.eval_expr(cond, env.clone())?;
                    if c.is_truthy()? {
                        let block_env = Scope::new(Some(env.clone()));
                        return self.exec_block(body, block_env);
                    }
                }
                if let Some(else_body) = &if_stmt.else_branch {
                    let block_env = Scope::new(Some(env.clone()));
                    return self.exec_block(else_body, block_env);
                }
                Ok(ControlFlow::Normal(Value::Unit))
            }
            Stmt::While { cond, body } => {
                loop {
                    let c = self.eval_expr(cond, env.clone())?;
                    if !c.is_truthy()? {
                        break;
                    }
                    let block_env = Scope::new(Some(env.clone()));
                    match self.exec_block(body, block_env)? {
                        ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                        ControlFlow::Normal(_) => {}
                    }
                }
                Ok(ControlFlow::Normal(Value::Unit))
            }
            Stmt::For { var, iter, body } => {
                let iter_val = self.eval_expr(iter, env.clone())?;
                match iter_val {
                    Value::Str(s) => {
                        for ch in s.chars() {
                            let block_env = Scope::new(Some(env.clone()));
                            block_env.borrow_mut().define(var.clone(), Value::Str(ch.to_string()));
                            match self.exec_block(body, block_env)? {
                                ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                                ControlFlow::Normal(_) => {}
                            }
                        }
                    }
                    Value::Int(n) => {
                        // 整数迭代：0..n
                        for i in 0..n {
                            let block_env = Scope::new(Some(env.clone()));
                            block_env.borrow_mut().define(var.clone(), Value::Int(i));
                            match self.exec_block(body, block_env)? {
                                ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                                ControlFlow::Normal(_) => {}
                            }
                        }
                    }
                    Value::List { elems } => {
                        // 列表迭代：逐个元素绑定（Phase 5.3 / v0.4.1 P0 缺口）
                        // Value::List 不可变，迭代期间元素不会被修改，直接按序取出即可
                        for elem in elems {
                            let block_env = Scope::new(Some(env.clone()));
                            block_env.borrow_mut().define(var.clone(), elem);
                            match self.exec_block(body, block_env)? {
                                ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                                ControlFlow::Normal(_) => {}
                            }
                        }
                    }
                    _ => {
                        return Err(RuntimeError::Msg(format!(
                            "for 循环不支持迭代 {}",
                            iter_val.type_name()
                        )));
                    }
                }
                Ok(ControlFlow::Normal(Value::Unit))
            }
            Stmt::Return(expr) => {
                let v = match expr {
                    Some(e) => self.eval_expr(e, env)?,
                    None => Value::Unit,
                };
                Ok(ControlFlow::Return(v))
            }
            Stmt::Expr(e) => {
                let v = self.eval_expr(e, env)?;
                Ok(ControlFlow::Normal(v))
            }
            Stmt::Hole { line, col } => Err(RuntimeError::Msg(format!(
                "代码洞（hole）@ {}:{} — 该处解析失败，无法执行（Phase 2.2 容错解析插入的占位符）",
                line, col
            ))),
        }
    }

    /// 求值表达式
    fn eval_expr(&mut self, expr: &Expr, env: ScopeRef) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Int(n) => Ok(Value::Int(*n)),
            Expr::Float(f) => Ok(Value::Float(*f)),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Str(s) => Ok(Value::Str(s.clone())),
            Expr::Unit => Ok(Value::Unit),
            Expr::Ident(name) => {
                // 先查变量
                if let Some(v) = env.borrow().get(name) {
                    Ok(v)
                } else if self.nullary_variants.contains(name) {
                    // 无参数枚举变体（如 None）
                    Ok(Value::Enum {
                        variant: name.clone(),
                        args: Vec::new(),
                    })
                } else if self.functions.contains_key(name) {
                    // 函数引用（转为闭包）
                    let f = self.functions.get(name).unwrap();
                    // Phase 1 限制：不支持函数作为一等值传递（仅闭包字面量可以）
                    Err(RuntimeError::Msg(format!(
                        "不能将函数 '{}' 作为值使用（限制：仅支持闭包字面量作为一等值）",
                        name
                    )))
                } else {
                    Err(RuntimeError::Msg(format!("未定义变量: '{}'", name)))
                }
            }
            Expr::Binary { op, left, right } => {
                let l = self.eval_expr(left, env.clone())?;
                let r = self.eval_expr(right, env)?;
                self.eval_binary(op, l, r)
            }
            Expr::Unary { op, expr } => {
                let v = self.eval_expr(expr, env)?;
                match op {
                    UnaryOp::Neg => match v {
                        Value::Int(n) => Ok(Value::Int(-n)),
                        Value::Float(f) => Ok(Value::Float(-f)),
                        _ => Err(RuntimeError::Msg(format!("负号不支持 {}", v.type_name()))),
                    },
                    UnaryOp::Not => match v {
                        Value::Bool(b) => Ok(Value::Bool(!b)),
                        _ => Err(RuntimeError::Msg(format!("! 不支持 {}", v.type_name()))),
                    },
                }
            }
            Expr::Logical { op, left, right } => {
                let l = self.eval_expr(left, env.clone())?;
                let l_truthy = l.is_truthy()?;
                match op {
                    LogicalOp::And => {
                        if !l_truthy {
                            Ok(Value::Bool(false))
                        } else {
                            let r = self.eval_expr(right, env)?;
                            Ok(Value::Bool(r.is_truthy()?))
                        }
                    }
                    LogicalOp::Or => {
                        if l_truthy {
                            Ok(Value::Bool(true))
                        } else {
                            let r = self.eval_expr(right, env)?;
                            Ok(Value::Bool(r.is_truthy()?))
                        }
                    }
                }
            }
            Expr::Call { callee, args } => {
                // 评估参数
                let mut arg_vals = Vec::with_capacity(args.len());
                for a in args {
                    arg_vals.push(self.eval_expr(a, env.clone())?);
                }
                self.eval_call(callee, &arg_vals, env)
            }
            Expr::Index { .. } => Err(RuntimeError::Msg("索引操作 Phase 2.1.4 未实现（元组用 .0 .1 访问）".to_string())),
            Expr::Field { expr, name } => {
                let v = self.eval_expr(expr, env)?;
                match &v {
                    Value::Record { fields } => {
                        // 字段访问：按名查找
                        fields
                            .iter()
                            .find(|(k, _)| k == name)
                            .map(|(_, v)| v.clone())
                            .ok_or_else(|| {
                                RuntimeError::Msg(format!("记录没有字段 '{}'", name))
                            })
                    }
                    Value::Tuple { elems } => {
                        // 元组索引：.0 .1 ...
                        let idx: usize = name.parse().map_err(|_| {
                            RuntimeError::Msg(format!(
                                "元组索引必须是数字，得到 '{}'",
                                name
                            ))
                        })?;
                        elems
                            .get(idx)
                            .cloned()
                            .ok_or_else(|| {
                                RuntimeError::Msg(format!(
                                    "元组索引 {} 越界（长度 {}）",
                                    idx,
                                    elems.len()
                                ))
                            })
                    }
                    _ => Err(RuntimeError::Msg(format!(
                        "不能对 {} 使用字段访问 '.{}'",
                        v.type_name(),
                        name
                    ))),
                }
            }
            Expr::Group(e) => self.eval_expr(e, env),
            Expr::If(if_stmt) => {
                for (cond, body) in &if_stmt.branches {
                    let c = self.eval_expr(cond, env.clone())?;
                    if c.is_truthy()? {
                        let block_env = Scope::new(Some(env));
                        return match self.exec_block(body, block_env)? {
                            ControlFlow::Return(v) | ControlFlow::Normal(v) => Ok(v),
                        };
                    }
                }
                if let Some(else_body) = &if_stmt.else_branch {
                    let block_env = Scope::new(Some(env));
                    match self.exec_block(else_body, block_env)? {
                        ControlFlow::Return(v) | ControlFlow::Normal(v) => Ok(v),
                    }
                } else {
                    Ok(Value::Unit)
                }
            }
            Expr::Closure {
                params,
                body,
                ..
            } => Ok(Value::Closure {
                params: params.clone(),
                body: (**body).clone(),
                env,
            }),
            Expr::Match(m) => {
                let scrutinee_val = self.eval_expr(&m.scrutinee, env.clone())?;
                for arm in &m.arms {
                    // 每个分支独立作用域，绑定模式变量
                    let arm_env = Scope::new(Some(env.clone()));
                    if self.match_pattern(&arm.pattern, &scrutinee_val, &arm_env)? {
                        // v0.4.2 P1-2: guard 求值，为 False 则继续尝试下一臂
                        if let Some(g) = &arm.guard {
                            let gv = self.eval_expr(g, arm_env.clone())?;
                            if !gv.is_truthy()? {
                                continue;
                            }
                        }
                        return match &arm.body {
                            MatchArmBody::Expr(e) => self.eval_expr(e, arm_env),
                            MatchArmBody::Block(b) => {
                                match self.exec_block(b, arm_env)? {
                                    ControlFlow::Return(v) | ControlFlow::Normal(v) => Ok(v),
                                }
                            }
                        };
                    }
                }
                Err(RuntimeError::Msg(format!(
                    "match 无匹配分支（值: {}）",
                    scrutinee_val.to_display()
                )))
            }
            Expr::Try(e) => {
                let v = self.eval_expr(e, env)?;
                match v {
                    Value::Enum { variant, args } if variant == "Ok" && args.len() == 1 => {
                        Ok(args.into_iter().next().unwrap())
                    }
                    Value::Enum { variant, args } if variant == "Some" && args.len() == 1 => {
                        Ok(args.into_iter().next().unwrap())
                    }
                    // Err(e) / None：触发提前返回，携带原 Err/None 值
                    Value::Enum { variant: e_var, args: e_args }
                        if (e_var == "Err" && e_args.len() == 1)
                            || (e_var == "None" && e_args.is_empty()) =>
                    {
                        Err(RuntimeError::EarlyReturn(Value::Enum {
                            variant: e_var,
                            args: e_args,
                        }))
                    }
                    _ => Err(RuntimeError::Msg(format!(
                        "`?` 只能用于 Result/Option，得到 {}",
                        v.to_display()
                    ))),
                }
            }
            Expr::Pipe { left, right } => {
                // 求值左侧，作为右侧函数的第一个参数
                let lv = self.eval_expr(left, env.clone())?;
                match right.as_ref() {
                    Expr::Call { callee, args } => {
                        // x |> f(y, z) => f(x, y, z)
                        let mut arg_vals = Vec::with_capacity(args.len() + 1);
                        arg_vals.push(lv);
                        for a in args {
                            arg_vals.push(self.eval_expr(a, env.clone())?);
                        }
                        self.eval_call(callee, &arg_vals, env)
                    }
                    _ => {
                        // x |> f => f(x)
                        // 右侧作为 callee 直接调用（Ident 解析为函数名/变体/闭包）
                        self.eval_call(right, &[lv], env)
                    }
                }
            }
            Expr::Record { fields } => {
                let mut vals = Vec::with_capacity(fields.len());
                for (name, e) in fields {
                    let v = self.eval_expr(e, env.clone())?;
                    vals.push((name.clone(), v));
                }
                Ok(Value::Record { fields: vals })
            }
            Expr::Tuple { elems } => {
                let mut vals = Vec::with_capacity(elems.len());
                for e in elems {
                    vals.push(self.eval_expr(e, env.clone())?);
                }
                Ok(Value::Tuple { elems: vals })
            }
            Expr::Range { start, end } => {
                // v0.4.2 P1-1: a..b → List<Int>（左闭右开，与 for i in n 的 0..n 语义一致）
                // 求值为 List 使得 range 可以直接复用 for-in-List（Phase 5.3）与 list 模块
                let sv = self.eval_expr(start, env.clone())?;
                let ev = self.eval_expr(end, env)?;
                match (sv, ev) {
                    (Value::Int(a), Value::Int(b)) => Ok(Value::List {
                        elems: (a..b).map(Value::Int).collect(),
                    }),
                    (s, e) => Err(RuntimeError::Msg(format!(
                        "range 表达式 a..b 的两端必须是 Int，得到 {} 和 {}",
                        s.type_name(),
                        e.type_name()
                    ))),
                }
            }
        }
    }

    /// 调用求值：对已求值好的参数调用 callee
    /// callee 可能是：
    /// - Ident(变体名)：构造枚举值
    /// - Ident(内置函数名)：调用内置函数
    /// - Ident(用户函数名)：调用用户函数
    /// - Ident(闭包变量名)：调用闭包
    /// - 其他表达式：求值为 Closure 后调用
    fn eval_call(
        &mut self,
        callee: &Expr,
        arg_vals: &[Value],
        env: ScopeRef,
    ) -> Result<Value, RuntimeError> {
        match callee {
            Expr::Ident(name) => {
                // 枚举变体构造：Ok(v), Err(e), Some(v), 用户带参变体
                // 注意：变体名不能与变量/函数同名（变量优先已在前面 eval，这里 name 是 callee Ident）
                if self.variants.contains(name) && !env.borrow().get(name).is_some() {
                    return Ok(Value::Enum {
                        variant: name.clone(),
                        args: arg_vals.to_vec(),
                    });
                }
                // 内置函数
                if let Some(v) = self.call_builtin(name, arg_vals)? {
                    return Ok(v);
                }
                // 用户函数
                if let Some(f) = self.functions.get(name).cloned() {
                    return self.call_function(&f, arg_vals);
                }
                // 闭包变量
                if let Some(Value::Closure {
                    params,
                    body,
                    env: closure_env,
                }) = env.borrow().get(name)
                {
                    return self.call_closure(&params, &body, closure_env.clone(), arg_vals);
                }
                Err(RuntimeError::Msg(format!("未定义函数: '{}'", name)))
            }
            _ => {
                // 闭包调用
                let c = self.eval_expr(callee, env)?;
                match c {
                    Value::Closure {
                        params,
                        body,
                        env: closure_env,
                    } => self.call_closure(&params, &body, closure_env, arg_vals),
                    _ => Err(RuntimeError::Msg(format!(
                        "不能调用 {} 类型的值",
                        c.type_name()
                    ))),
                }
            }
        }
    }

    /// 模式匹配：尝试用 pat 匹配 val，匹配则绑定变量到 env 并返回 true
    fn match_pattern(
        &self,
        pat: &Pattern,
        val: &Value,
        env: &ScopeRef,
    ) -> Result<bool, RuntimeError> {
        match pat {
            Pattern::Wildcard => Ok(true),
            Pattern::Binder(name) => {
                // 若 name 是已知无参数变体（如 None、用户 Red），按变体匹配
                if self.nullary_variants.contains(name) {
                    return match val {
                        Value::Enum { variant, args } => {
                            Ok(variant == name && args.is_empty())
                        }
                        _ => Ok(false),
                    };
                }
                // 否则作为绑定变量
                env.borrow_mut().define(name.clone(), val.clone());
                Ok(true)
            }
            Pattern::Lit(e) => {
                // 字面量模式：仅支持 Int/Float/Bool/Str
                let lit_val = match e {
                    Expr::Int(n) => Value::Int(*n),
                    Expr::Float(f) => Value::Float(*f),
                    Expr::Bool(b) => Value::Bool(*b),
                    Expr::Str(s) => Value::Str(s.clone()),
                    _ => {
                        return Err(RuntimeError::Msg(format!(
                            "不支持的字面量模式: {:?}",
                            e
                        )))
                    }
                };
                Ok(self.values_eq(&lit_val, val))
            }
            Pattern::Variant { name, sub } => {
                match val {
                    Value::Enum {
                        variant,
                        args,
                    } => {
                        if variant != name {
                            return Ok(false);
                        }
                        if args.len() != sub.len() {
                            return Err(RuntimeError::Msg(format!(
                                "变体 {} 期望 {} 个子模式，得到 {} 个参数",
                                name,
                                sub.len(),
                                args.len()
                            )));
                        }
                        for (p, v) in sub.iter().zip(args.iter()) {
                            if !self.match_pattern(p, v, env)? {
                                return Ok(false);
                            }
                        }
                        Ok(true)
                    }
                    _ => Ok(false),
                }
            }
        }
    }

    /// 二元运算
    fn eval_binary(&self, op: &BinOp, l: Value, r: Value) -> Result<Value, RuntimeError> {
        // 字符串拼接：+ 重载
        // v0.4.1 P0-2：任一侧是 String 即可拼接，另一侧用 to_display() 提升
        // （消除 "n = " + 42 这类 LLM 自然写法被拒的缺口，对齐 Python/JS 习惯）
        if matches!(op, BinOp::Add) {
            if let Value::Str(a) = &l {
                let b = match &r {
                    Value::Str(b) => b.clone(),
                    other => other.to_display(),
                };
                return Ok(Value::Str(format!("{}{}", a, b)));
            }
            if let Value::Str(b) = &r {
                return Ok(Value::Str(format!("{}{}", l.to_display(), b)));
            }
        }
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                self.eval_arith(op, l, r)
            }
            BinOp::Eq => Ok(Value::Bool(self.values_eq(&l, &r))),
            BinOp::NotEq => Ok(Value::Bool(!self.values_eq(&l, &r))),
            BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => {
                self.eval_compare(op, l, r)
            }
        }
    }

    fn eval_arith(&self, op: &BinOp, l: Value, r: Value) -> Result<Value, RuntimeError> {
        match (&l, &r) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(match op {
                BinOp::Add => a + b,
                BinOp::Sub => a - b,
                BinOp::Mul => a * b,
                BinOp::Div => {
                    if *b == 0 {
                        return Err(RuntimeError::Msg("整数除以零".to_string()));
                    }
                    a / b
                }
                BinOp::Mod => {
                    if *b == 0 {
                        return Err(RuntimeError::Msg("整数取模零".to_string()));
                    }
                    a % b
                }
                _ => unreachable!(),
            })),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(match op {
                BinOp::Add => a + b,
                BinOp::Sub => a - b,
                BinOp::Mul => a * b,
                BinOp::Div => a / b,
                BinOp::Mod => a % b,
                _ => unreachable!(),
            })),
            // Int + Float 混合：提升为 Float
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(match op {
                BinOp::Add => *a as f64 + b,
                BinOp::Sub => *a as f64 - b,
                BinOp::Mul => *a as f64 * b,
                BinOp::Div => *a as f64 / b,
                BinOp::Mod => *a as f64 % b,
                _ => unreachable!(),
            })),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(match op {
                BinOp::Add => a + *b as f64,
                BinOp::Sub => a - *b as f64,
                BinOp::Mul => a * *b as f64,
                BinOp::Div => a / *b as f64,
                BinOp::Mod => a % *b as f64,
                _ => unreachable!(),
            })),
            _ => Err(RuntimeError::Msg(format!(
                "运算 {:?} 不支持 {} 和 {}",
                op,
                l.type_name(),
                r.type_name()
            ))),
        }
    }

    fn eval_compare(&self, op: &BinOp, l: Value, r: Value) -> Result<Value, RuntimeError> {
        let ord = match (&l, &r) {
            (Value::Int(a), Value::Int(b)) => a.cmp(b),
            (Value::Float(a), Value::Float(b)) => a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
            (Value::Str(a), Value::Str(b)) => a.cmp(b),
            (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
            _ => {
                return Err(RuntimeError::Msg(format!(
                    "比较 {:?} 不支持 {} 和 {}",
                    op,
                    l.type_name(),
                    r.type_name()
                )));
            }
        };
        Ok(Value::Bool(match op {
            BinOp::Lt => ord == std::cmp::Ordering::Less,
            BinOp::Gt => ord == std::cmp::Ordering::Greater,
            BinOp::LtEq => ord != std::cmp::Ordering::Greater,
            BinOp::GtEq => ord != std::cmp::Ordering::Less,
            _ => unreachable!(),
        }))
    }

    fn values_eq(&self, l: &Value, r: &Value) -> bool {
        match (l, r) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Unit, Value::Unit) => true,
            (
                Value::Enum {
                    variant: v1,
                    args: a1,
                },
                Value::Enum {
                    variant: v2,
                    args: a2,
                },
            ) => {
                if v1 != v2 || a1.len() != a2.len() {
                    return false;
                }
                a1.iter()
                    .zip(a2.iter())
                    .all(|(x, y)| self.values_eq(x, y))
            }
            (Value::Tuple { elems: a1 }, Value::Tuple { elems: a2 }) => {
                a1.len() == a2.len()
                    && a1.iter().zip(a2.iter()).all(|(x, y)| self.values_eq(x, y))
            }
            (Value::Record { fields: f1 }, Value::Record { fields: f2 }) => {
                // 结构等价：字段集相同（顺序不敏感），对应值相等
                if f1.len() != f2.len() {
                    return false;
                }
                f1.iter().all(|(k, v)| {
                    f2.iter()
                        .any(|(k2, v2)| k == k2 && self.values_eq(v, v2))
                })
            }
            (Value::List { elems: a1 }, Value::List { elems: a2 }) => {
                a1.len() == a2.len()
                    && a1.iter().zip(a2.iter()).all(|(x, y)| self.values_eq(x, y))
            }
            _ => false,
        }
    }

    /// 调用用户函数
    fn call_function(&mut self, f: &FnDecl, args: &[Value]) -> Result<Value, RuntimeError> {
        if args.len() != f.params.len() {
            return Err(RuntimeError::Msg(format!(
                "函数 '{}' 期望 {} 个参数，得到 {} 个",
                f.name,
                f.params.len(),
                args.len()
            )));
        }
        let env = Scope::new(Some(self.globals.clone()));
        for (param, arg) in f.params.iter().zip(args.iter()) {
            env.borrow_mut().define(param.name.clone(), arg.clone());
        }
        match self.exec_block(&f.body, env)? {
            ControlFlow::Return(v) | ControlFlow::Normal(v) => Ok(v),
        }
    }

    /// 调用闭包
    fn call_closure(
        &mut self,
        params: &[Param],
        body: &Block,
        closure_env: ScopeRef,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        if args.len() != params.len() {
            return Err(RuntimeError::Msg(format!(
                "闭包期望 {} 个参数，得到 {} 个",
                params.len(),
                args.len()
            )));
        }
        let env = Scope::new(Some(closure_env));
        for (param, arg) in params.iter().zip(args.iter()) {
            env.borrow_mut().define(param.name.clone(), arg.clone());
        }
        match self.exec_block(body, env)? {
            ControlFlow::Return(v) | ControlFlow::Normal(v) => Ok(v),
        }
    }

    /// 内置函数调用
    /// 返回 Ok(Some(v)) 表示是内置函数并成功调用
    /// 返回 Ok(None) 表示不是内置函数（让上层当用户函数/闭包处理）
    /// 返回 Err 表示是内置函数但调用错误（参数数量/类型错）或未导入
    fn call_builtin(&self, name: &str, args: &[Value]) -> Result<Option<Value>, RuntimeError> {
        // 1. 解析别名：name 可能是别名（如 log → println），也可能是本名
        let real_name = self
            .import_aliases
            .get(name)
            .map(|s| s.as_str())
            .unwrap_or(name);

        // 2. 可用性检查：name（别名/本名）必须在 available_builtins 中
        //    available_builtins 包含 prelude（println/print）+ 显式导入的符号/别名
        if !self.available_builtins.contains(name) {
            // 若 real_name 是已知内置函数但未导入，给出明确的"未导入"错误
            if is_known_builtin(real_name) {
                let module = module_of(real_name).unwrap_or("未知模块");
                return Err(RuntimeError::Msg(format!(
                    "符号 '{}' 未导入。需在文件顶部声明：from {} import {{{}}}",
                    name, module, real_name
                )));
            }
            // 不是已知内置函数，返回 None 让上层当用户函数/闭包处理
            return Ok(None);
        }

        // 3. 调用真实函数实现
        match real_name {
            "println" => {
                if args.len() != 1 {
                    return Err(RuntimeError::Msg(format!(
                        "println 期望 1 个参数，得到 {} 个",
                        args.len()
                    )));
                }
                println!("{}", args[0].to_display());
                Ok(Some(Value::Unit))
            }
            "print" => {
                if args.len() != 1 {
                    return Err(RuntimeError::Msg(format!(
                        "print 期望 1 个参数，得到 {} 个",
                        args.len()
                    )));
                }
                print!("{}", args[0].to_display());
                Ok(Some(Value::Unit))
            }
            // string 模块
            "len" => {
                expect_arity("len", 1, args)?;
                match &args[0] {
                    Value::Str(s) => Ok(Some(Value::Int(s.chars().count() as i64))),
                    _ => Err(RuntimeError::Msg("len 期望 String".to_string())),
                }
            }
            "int_to_string" => {
                expect_arity("int_to_string", 1, args)?;
                match &args[0] {
                    Value::Int(n) => Ok(Some(Value::Str(n.to_string()))),
                    _ => Err(RuntimeError::Msg("int_to_string 期望 Int".to_string())),
                }
            }
            "string_to_int" => {
                expect_arity("string_to_int", 1, args)?;
                match &args[0] {
                    Value::Str(s) => match s.parse::<i64>() {
                        Ok(n) => Ok(Some(Value::Int(n))),
                        Err(_) => Ok(Some(Value::Unit)), // Phase 1 简化：失败返回 Unit
                    },
                    _ => Err(RuntimeError::Msg("string_to_int 期望 String".to_string())),
                }
            }
            "trim" => {
                expect_arity("trim", 1, args)?;
                match &args[0] {
                    Value::Str(s) => Ok(Some(Value::Str(s.trim().to_string()))),
                    _ => Err(RuntimeError::Msg("trim 期望 String".to_string())),
                }
            }
            "upper" => {
                expect_arity("upper", 1, args)?;
                match &args[0] {
                    Value::Str(s) => Ok(Some(Value::Str(s.to_uppercase()))),
                    _ => Err(RuntimeError::Msg("upper 期望 String".to_string())),
                }
            }
            "lower" => {
                expect_arity("lower", 1, args)?;
                match &args[0] {
                    Value::Str(s) => Ok(Some(Value::Str(s.to_lowercase()))),
                    _ => Err(RuntimeError::Msg("lower 期望 String".to_string())),
                }
            }
            // math 模块
            "sqrt" => {
                expect_arity("sqrt", 1, args)?;
                match &args[0] {
                    Value::Float(x) => Ok(Some(Value::Float(x.sqrt()))),
                    Value::Int(x) => Ok(Some(Value::Float((*x as f64).sqrt()))),
                    _ => Err(RuntimeError::Msg("sqrt 期望 Float 或 Int".to_string())),
                }
            }
            "abs" => {
                expect_arity("abs", 1, args)?;
                match &args[0] {
                    Value::Int(n) => Ok(Some(Value::Int(n.abs()))),
                    Value::Float(x) => Ok(Some(Value::Float(x.abs()))),
                    _ => Err(RuntimeError::Msg("abs 期望 Int 或 Float".to_string())),
                }
            }
            "min" => {
                expect_arity("min", 2, args)?;
                match (&args[0], &args[1]) {
                    (Value::Int(a), Value::Int(b)) => Ok(Some(Value::Int((*a).min(*b)))),
                    (Value::Float(a), Value::Float(b)) => Ok(Some(Value::Float(a.min(*b)))),
                    _ => Err(RuntimeError::Msg(
                        "min 期望两个同类型参数 (Int, Int) 或 (Float, Float)".to_string(),
                    )),
                }
            }
            "max" => {
                expect_arity("max", 2, args)?;
                match (&args[0], &args[1]) {
                    (Value::Int(a), Value::Int(b)) => Ok(Some(Value::Int((*a).max(*b)))),
                    (Value::Float(a), Value::Float(b)) => Ok(Some(Value::Float(a.max(*b)))),
                    _ => Err(RuntimeError::Msg(
                        "max 期望两个同类型参数 (Int, Int) 或 (Float, Float)".to_string(),
                    )),
                }
            }
            // Phase 3.3: list 模块（不可变语义，函数返回新 List）
            "list_empty" => {
                expect_arity("list_empty", 0, args)?;
                Ok(Some(Value::List { elems: Vec::new() }))
            }
            "list_length" => {
                expect_arity("list_length", 1, args)?;
                match &args[0] {
                    Value::List { elems } => Ok(Some(Value::Int(elems.len() as i64))),
                    _ => Err(RuntimeError::Msg("list_length 期望 List".to_string())),
                }
            }
            "list_get" => {
                expect_arity("list_get", 2, args)?;
                let idx = match &args[1] {
                    Value::Int(n) => *n,
                    _ => {
                        return Err(RuntimeError::Msg(
                            "list_get 第二个参数期望 Int (索引)".to_string(),
                        ));
                    }
                };
                match &args[0] {
                    Value::List { elems } => {
                        if idx < 0 || (idx as usize) >= elems.len() {
                            Err(RuntimeError::Msg(format!(
                                "list_get 索引 {} 越界（列表长度 {}）",
                                idx,
                                elems.len()
                            )))
                        } else {
                            Ok(Some(elems[idx as usize].clone()))
                        }
                    }
                    _ => Err(RuntimeError::Msg("list_get 期望 List".to_string())),
                }
            }
            "list_is_empty" => {
                expect_arity("list_is_empty", 1, args)?;
                match &args[0] {
                    Value::List { elems } => Ok(Some(Value::Bool(elems.is_empty()))),
                    _ => Err(RuntimeError::Msg("list_is_empty 期望 List".to_string())),
                }
            }
            "list_head" => {
                expect_arity("list_head", 1, args)?;
                match &args[0] {
                    Value::List { elems } => {
                        if elems.is_empty() {
                            Err(RuntimeError::Msg("list_head 空列表无首元素".to_string()))
                        } else {
                            Ok(Some(elems[0].clone()))
                        }
                    }
                    _ => Err(RuntimeError::Msg("list_head 期望 List".to_string())),
                }
            }
            "list_tail" => {
                expect_arity("list_tail", 1, args)?;
                match &args[0] {
                    Value::List { elems } => {
                        if elems.is_empty() {
                            Err(RuntimeError::Msg("list_tail 空列表无尾".to_string()))
                        } else {
                            Ok(Some(Value::List {
                                elems: elems[1..].to_vec(),
                            }))
                        }
                    }
                    _ => Err(RuntimeError::Msg("list_tail 期望 List".to_string())),
                }
            }
            "list_cons" => {
                // list_cons(head, list) → 新 List：[head, ...list]
                expect_arity("list_cons", 2, args)?;
                match &args[1] {
                    Value::List { elems } => {
                        let mut new_elems = Vec::with_capacity(elems.len() + 1);
                        new_elems.push(args[0].clone());
                        new_elems.extend(elems.iter().cloned());
                        Ok(Some(Value::List { elems: new_elems }))
                    }
                    _ => Err(RuntimeError::Msg("list_cons 第二个参数期望 List".to_string())),
                }
            }
            // Phase 3.3: json 模块
            "json_parse" => {
                expect_arity("json_parse", 1, args)?;
                match &args[0] {
                    Value::Str(s) => match crate::json::parse(s) {
                        Ok(v) => Ok(Some(v)),
                        Err(e) => Err(RuntimeError::Msg(format!("json_parse 失败: {}", e))),
                    },
                    _ => Err(RuntimeError::Msg("json_parse 期望 String".to_string())),
                }
            }
            "json_stringify" => {
                expect_arity("json_stringify", 1, args)?;
                Ok(Some(Value::Str(crate::json::stringify(&args[0]))))
            }
            // Phase 3.4: string 扩展（纯函数）
            "split" => {
                // split(s, sep) -> List<String>；sep 为空字符串时按字符分割
                expect_arity("split", 2, args)?;
                match (&args[0], &args[1]) {
                    (Value::Str(s), Value::Str(sep)) => {
                        let elems: Vec<Value> = if sep.is_empty() {
                            // 按字符分割
                            s.chars().map(|c| Value::Str(c.to_string())).collect()
                        } else {
                            s.split(sep.as_str())
                                .map(|piece| Value::Str(piece.to_string()))
                                .collect()
                        };
                        Ok(Some(Value::List { elems }))
                    }
                    _ => Err(RuntimeError::Msg(
                        "split 期望两个 String 参数 (s, sep)".to_string(),
                    )),
                }
            }
            "contains" => {
                expect_arity("contains", 2, args)?;
                match (&args[0], &args[1]) {
                    (Value::Str(s), Value::Str(sub)) => {
                        Ok(Some(Value::Bool(s.contains(sub.as_str()))))
                    }
                    _ => Err(RuntimeError::Msg(
                        "contains 期望两个 String 参数 (s, sub)".to_string(),
                    )),
                }
            }
            "replace" => {
                // replace(s, from, to) -> String；替换所有匹配
                expect_arity("replace", 3, args)?;
                match (&args[0], &args[1], &args[2]) {
                    (Value::Str(s), Value::Str(from), Value::Str(to)) => {
                        Ok(Some(Value::Str(s.replace(from.as_str(), to.as_str()))))
                    }
                    _ => Err(RuntimeError::Msg(
                        "replace 期望三个 String 参数 (s, from, to)".to_string(),
                    )),
                }
            }
            "starts_with" => {
                expect_arity("starts_with", 2, args)?;
                match (&args[0], &args[1]) {
                    (Value::Str(s), Value::Str(prefix)) => {
                        Ok(Some(Value::Bool(s.starts_with(prefix.as_str()))))
                    }
                    _ => Err(RuntimeError::Msg(
                        "starts_with 期望两个 String 参数 (s, prefix)".to_string(),
                    )),
                }
            }
            "ends_with" => {
                expect_arity("ends_with", 2, args)?;
                match (&args[0], &args[1]) {
                    (Value::Str(s), Value::Str(suffix)) => {
                        Ok(Some(Value::Bool(s.ends_with(suffix.as_str()))))
                    }
                    _ => Err(RuntimeError::Msg(
                        "ends_with 期望两个 String 参数 (s, suffix)".to_string(),
                    )),
                }
            }
            // Phase 3.4: file 模块（均有 [IO] 效应）
            "file_read" => {
                // file_read(path) -> String；失败返回运行时错误
                expect_arity("file_read", 1, args)?;
                match &args[0] {
                    Value::Str(path) => match fs::read_to_string(path) {
                        Ok(content) => Ok(Some(Value::Str(content))),
                        Err(e) => Err(RuntimeError::Msg(format!(
                            "file_read 无法读取 '{}': {}",
                            path, e
                        ))),
                    },
                    _ => Err(RuntimeError::Msg("file_read 期望 String".to_string())),
                }
            }
            "file_write" => {
                // file_write(path, content) -> Unit；覆盖写入
                expect_arity("file_write", 2, args)?;
                match (&args[0], &args[1]) {
                    (Value::Str(path), Value::Str(content)) => {
                        match fs::write(path, content.as_bytes()) {
                            Ok(()) => Ok(Some(Value::Unit)),
                            Err(e) => Err(RuntimeError::Msg(format!(
                                "file_write 无法写入 '{}': {}",
                                path, e
                            ))),
                        }
                    }
                    _ => Err(RuntimeError::Msg(
                        "file_write 期望 (String, String)".to_string(),
                    )),
                }
            }
            "file_append" => {
                // file_append(path, content) -> Unit；追加写入
                expect_arity("file_append", 2, args)?;
                match (&args[0], &args[1]) {
                    (Value::Str(path), Value::Str(content)) => {
                        let mut file = match fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(path)
                        {
                            Ok(f) => f,
                            Err(e) => {
                                return Err(RuntimeError::Msg(format!(
                                    "file_append 无法打开 '{}': {}",
                                    path, e
                                )))
                            }
                        };
                        match file.write_all(content.as_bytes()) {
                            Ok(()) => Ok(Some(Value::Unit)),
                            Err(e) => Err(RuntimeError::Msg(format!(
                                "file_append 写入失败 '{}': {}",
                                path, e
                            ))),
                        }
                    }
                    _ => Err(RuntimeError::Msg(
                        "file_append 期望 (String, String)".to_string(),
                    )),
                }
            }
            "file_exists" => {
                expect_arity("file_exists", 1, args)?;
                match &args[0] {
                    Value::Str(path) => Ok(Some(Value::Bool(Path::new(path).exists()))),
                    _ => Err(RuntimeError::Msg("file_exists 期望 String".to_string())),
                }
            }
            // Phase 3.5: env 模块
            "args" => {
                // args() -> List<String>；返回程序参数（argv[0] = .lom 文件路径）
                expect_arity("args", 0, args)?;
                let elems: Vec<Value> = self
                    .program_args
                    .iter()
                    .map(|s| Value::Str(s.clone()))
                    .collect();
                Ok(Some(Value::List { elems }))
            }
            _ => Ok(None), // 不是内置函数
        }
    }
}

/// 检查 name 是否是已知内置函数（不论是否已导入）
fn is_known_builtin(name: &str) -> bool {
    matches!(
        name,
        "println"
            | "print"
            | "len"
            | "int_to_string"
            | "string_to_int"
            | "trim"
            | "upper"
            | "lower"
            | "split"
            | "contains"
            | "replace"
            | "starts_with"
            | "ends_with"
            | "sqrt"
            | "abs"
            | "min"
            | "max"
            | "list_empty"
            | "list_length"
            | "list_get"
            | "list_is_empty"
            | "list_head"
            | "list_tail"
            | "list_cons"
            | "json_parse"
            | "json_stringify"
            | "file_read"
            | "file_write"
            | "file_append"
            | "file_exists"
            | "args"
    )
}

/// 返回内置函数所属的标准库模块名（用于错误提示）
fn module_of(name: &str) -> Option<&'static str> {
    match name {
        "println" | "print" => Some("io"),
        "len" | "int_to_string" | "string_to_int" | "trim" | "upper" | "lower" | "split"
        | "contains" | "replace" | "starts_with" | "ends_with" => Some("string"),
        "sqrt" | "abs" | "min" | "max" => Some("math"),
        "list_empty" | "list_length" | "list_get" | "list_is_empty" | "list_head" | "list_tail" | "list_cons" => {
            Some("list")
        }
        "json_parse" | "json_stringify" => Some("json"),
        "file_read" | "file_write" | "file_append" | "file_exists" => Some("file"),
        "args" => Some("env"),
        _ => None,
    }
}

/// 校验参数数量，不符则返回错误
fn expect_arity(name: &str, expected: usize, args: &[Value]) -> Result<(), RuntimeError> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(RuntimeError::Msg(format!(
            "{} 期望 {} 个参数，得到 {} 个",
            name,
            expected,
            args.len()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    fn run_src(src: &str) -> Result<(), RuntimeError> {
        let program = Parser::parse(src).unwrap_or_else(|e| panic!("解析失败: {}", e));
        let mut interp = Interpreter::new();
        interp.run(&program)
    }

    #[test]
    fn test_arithmetic() {
        let src = "fn main() -> Unit\n    println(1 + 2 * 3)\nend";
        run_src(src).unwrap();
    }

    #[test]
    fn test_fib() {
        let src = r#"
fn fib(n: Int) -> Int
    if n < 2
        n
    else
        fib(n - 1) + fib(n - 2)
    end
end

fn main() -> Unit
    println(fib(10))
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_string_concat() {
        let src = r#"fn main() -> Unit
    let s = "Hello" + ", " + "World"
    println(s)
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_string_concat_promotion() {
        // v0.4.1 P0-2: String + 非 String → 另一侧 to_display() 提升
        let src = r#"fn main() -> Unit
    println("n = " + 42)
    println("pi ≈ " + 3.14)
    println("flag = " + True)
    println(42 + " is the answer")
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_string_concat_promotion_list_record() {
        // 复合类型也能提升:List/Record 走 to_display 的调试格式
        let src = r#"from list import {list_cons, list_empty}

fn main() -> Unit
    let xs = list_cons(1, list_cons(2, list_empty()))
    println("xs = " + xs)
    println("p = " + {x: 3, y: 4})
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_closure() {
        let src = r#"
fn main() -> Unit
    let add = fn(x: Int, y: Int) -> Int
        x + y
    end
    println(add(3, 4))
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_while() {
        let src = r#"
fn main() -> Unit
    let mut i = 0
    let mut sum = 0
    while i < 5
        sum = sum + i
        i = i + 1
    end
    println(sum)
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_for_string() {
        let src = r#"
fn main() -> Unit
    for c in "abc"
        println(c)
    end
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_for_list() {
        // Phase 5.3 (v0.4.1): for 遍历 List —— 元素按序绑定求和 1+2+3=6
        let src = r#"
from list import {list_cons, list_empty}

fn main() -> Unit
    let xs = list_cons(1, list_cons(2, list_cons(3, list_empty())))
    let mut sum = 0
    for x in xs
        sum = sum + x
    end
    println(sum)
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_for_list_return_propagates() {
        // for 遍历 List 时 return 正确穿透循环提前返回
        let src = r#"
from list import {list_cons, list_empty}

fn find_first_gt2() -> Int
    let xs = list_cons(1, list_cons(2, list_cons(3, list_empty())))
    for x in xs
        if x > 2
            return x
        end
    end
    0
end

fn main() -> Unit
    println(find_first_gt2())
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_compound_assign() {
        // v0.4.1 P0-3: += -= *= /= 四种复合赋值
        let src = r#"
fn main() -> Unit
    let mut sum = 0
    sum += 5
    println(sum)
    sum *= 2
    println(sum)
    sum -= 3
    println(sum)
    sum /= 2
    println(sum)
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_compound_assign_string() {
        // += 与字符串拼接提升组合:s += 非 String 自动 to_display()
        let src = r#"
fn main() -> Unit
    let mut s = "a"
    s += "b"
    s += 1
    println(s)
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_compound_assign_undefined_errors() {
        // 复合赋值目标是未定义变量 → 与 = 一样报"赋值给未定义变量"
        let src = r#"
fn main() -> Unit
    x += 1
end
"#;
        assert!(run_src(src).is_err());
    }

    #[test]
    fn test_range_for_loop() {
        // v0.4.2 P1-1: for i in 1..5 → 1+2+3+4=10(左闭右开)
        let src = r#"
fn main() -> Unit
    let mut total = 0
    for i in 1..5
        total += i
    end
    println(total)
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_range_produces_list() {
        // range 求值为 List<Int>,可直接复用 list 模块与打印
        let src = r#"
from list import {list_length}

fn main() -> Unit
    let xs = 1..4
    println(xs)
    println(list_length(xs))
    let empty = 5..1
    println(empty)
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_range_non_int_errors() {
        // 两端必须是 Int:1.5..3 运行时报错
        let src = r#"
fn main() -> Unit
    let xs = 1.5..3
    println(xs)
end
"#;
        assert!(run_src(src).is_err());
    }

    #[test]
    fn test_match_guard() {
        // v0.4.2 P1-2: match guard —— 模式匹配后再看 guard,为假继续下一臂
        let src = r#"
fn classify(n: Int) -> String
    match n
        m if m < 0 => "negative"
        0 => "zero"
        m if m > 100 => "big"
        _ => "normal"
    end
end

fn main() -> Unit
    println(classify(-5))
    println(classify(0))
    println(classify(42))
    println(classify(200))
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_match_guard_uses_binding() {
        // guard 可以使用模式绑定的变量;guard 失败穿透到后面的普通变体臂
        let src = r#"
fn main() -> Unit
    match Some(5)
        Some(x) if x > 10 => "big"
        Some(x) => "some: " + x
        None => "none"
    end
    |> println
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_match_guard_all_false_no_match() {
        // 全部 guard 为假且无兜底 → 运行时"无匹配分支"错误
        let src = r#"
fn main() -> Unit
    match 5
        m if m > 10 => "big"
        m if m < 0 => "neg"
    end
    |> println
end
"#;
        assert!(run_src(src).is_err());
    }

    #[test]
    fn test_if_expression() {
        let src = r#"
fn sign(n: Int) -> Int
    if n > 0
        1
    elif n < 0
        -1
    else
        0
    end
end

fn main() -> Unit
    println(sign(5))
    println(sign(-3))
    println(sign(0))
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_match_literal() {
        let src = r#"
fn classify(n: Int) -> String
    match n
        0 => "zero"
        1 => "one"
        _ => "many"
    end
end

fn main() -> Unit
    println(classify(0))
    println(classify(1))
    println(classify(42))
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_match_result() {
        let src = r#"
fn safe_divide(x: Int, y: Int) -> Result<Int, String>
    if y == 0
        Err("div by zero")
    else
        Ok(x / y)
    end
end

fn main() -> Unit
    match safe_divide(10, 2)
        Ok(n) => println(n)
        Err(e) => println("Error: " + e)
    end
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_match_option_none() {
        let src = r#"
fn main() -> Unit
    let x = None
    match x
        Some(v) => println(v)
        None => println("nothing")
    end
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_match_user_enum() {
        let src = r#"
enum Color = Red | Green | Blue

fn code(c: Color) -> Int
    match c
        Red => 1
        Green => 2
        Blue => 3
    end
end

fn main() -> Unit
    println(code(Red))
    println(code(Blue))
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_match_form_b_block() {
        let src = r#"
fn main() -> Unit
    let x = Some(5)
    match x
        Some(v) =>
            println("got:")
            println(v)
        end
        None =>
            println("none")
        end
    end
end
"#;
        run_src(src).unwrap();
    }

    /// 辅助：解析并执行 main，返回其 body 尾表达式的求值结果
    /// 用于在测试中严格验证表达式求值
    fn eval_main_tail(src: &str) -> Value {
        let program = Parser::parse(src).unwrap_or_else(|e| panic!("解析失败: {}", e));
        let mut interp = Interpreter::new();
        // 先处理导入（与 run 一致）
        for item in &program.items {
            if let Item::Import(imp) = item {
                interp.process_import(imp).unwrap_or_else(|e| panic!("导入失败: {}", e));
            }
        }
        // 注册所有顶层函数 + 枚举变体（复用 run 的逻辑）
        for item in &program.items {
            match item {
                Item::Fn(f) => {
                    interp.functions.insert(f.name.clone(), f.clone());
                }
                Item::Enum(e) => {
                    for v in &e.variants {
                        interp.variants.insert(v.name.clone());
                        if v.fields.is_empty() {
                            interp.nullary_variants.insert(v.name.clone());
                        }
                    }
                }
                Item::Import(_) => {} // 已在前面的循环处理
            }
        }
        let main = interp
            .functions
            .get("main")
            .cloned()
            .expect("测试需要 fn main");
        let env = Scope::new(Some(interp.globals.clone()));
        match interp.exec_block(&main.body, env).unwrap() {
            ControlFlow::Normal(v) | ControlFlow::Return(v) => v,
        }
    }

    #[test]
    fn test_pipeline_basic_run() {
        // 5 |> double |> inc => inc(double(5)) = 11
        let src = "fn double(x: Int) -> Int\n    x * 2\nend\nfn inc(x: Int) -> Int\n    x + 1\nend\nfn main() -> Int\n    5 |> double |> inc\nend";
        let v = eval_main_tail(src);
        match v {
            Value::Int(n) => assert_eq!(n, 11),
            other => panic!("期望 Int(11)，得到 {:?}", other),
        }
    }

    #[test]
    fn test_pipeline_with_args_run() {
        // 10 |> add(3) => add(10, 3) = 13
        let src = "fn add(x: Int, y: Int) -> Int\n    x + y\nend\nfn main() -> Int\n    10 |> add(3)\nend";
        let v = eval_main_tail(src);
        match v {
            Value::Int(n) => assert_eq!(n, 13),
            other => panic!("期望 Int(13)，得到 {:?}", other),
        }
    }

    #[test]
    fn test_pipeline_with_arithmetic_precedence() {
        // 1 + 2 |> double => double(1 + 2) = 6
        let src = "fn double(x: Int) -> Int\n    x * 2\nend\nfn main() -> Int\n    1 + 2 |> double\nend";
        let v = eval_main_tail(src);
        match v {
            Value::Int(n) => assert_eq!(n, 6),
            other => panic!("期望 Int(6)，得到 {:?}", other),
        }
    }

    #[test]
    fn test_pipeline_with_comparison_precedence() {
        // 5 |> double == 10 => (5 |> double) == 10 => True
        let src = "fn double(x: Int) -> Int\n    x * 2\nend\nfn main() -> Bool\n    5 |> double == 10\nend";
        let v = eval_main_tail(src);
        match v {
            Value::Bool(b) => assert!(b),
            other => panic!("期望 Bool(true)，得到 {:?}", other),
        }
    }

    #[test]
    fn test_pipeline_with_closure_value() {
        // 4 |> closure => 4 * 4 + 1 = 17
        let src = "fn main() -> Int\n    let f = fn(x: Int) -> Int\n        x * x + 1\n    end\n    4 |> f\nend";
        let v = eval_main_tail(src);
        match v {
            Value::Int(n) => assert_eq!(n, 17),
            other => panic!("期望 Int(17)，得到 {:?}", other),
        }
    }

    #[test]
    fn test_pipeline_chained_with_args() {
        // 1 |> add(2) |> add(3) => add(add(1, 2), 3) = 6
        let src = "fn add(x: Int, y: Int) -> Int\n    x + y\nend\nfn main() -> Int\n    1 |> add(2) |> add(3)\nend";
        let v = eval_main_tail(src);
        match v {
            Value::Int(n) => assert_eq!(n, 6),
            other => panic!("期望 Int(6)，得到 {:?}", other),
        }
    }

    // ===== Phase 2.1.5 显式导入执行测试 =====

    #[test]
    fn test_prelude_no_import_needed() {
        // println/print 在 prelude 中，无需显式导入
        let src = "fn main() -> Unit\n    println(1)\n    print(\"a\")\n    println(\"\")\nend";
        run_src(src).unwrap();
    }

    #[test]
    fn test_import_string_len() {
        let src = "from string import { len }\nfn main() -> Int\n    len(\"hello\")\nend";
        let v = eval_main_tail(src);
        match v {
            Value::Int(n) => assert_eq!(n, 5),
            other => panic!("期望 Int(5)，得到 {:?}", other),
        }
    }

    #[test]
    fn test_import_string_upper_lower() {
        let src = "from string import { upper, lower }\nfn main() -> String\n    upper(\"hi\") + lower(\"BYE\")\nend";
        let v = eval_main_tail(src);
        match v {
            Value::Str(s) => assert_eq!(s, "HIbye"),
            other => panic!("期望 Str(\"HIbye\")，得到 {:?}", other),
        }
    }

    #[test]
    fn test_import_string_trim() {
        let src = "from string import { trim }\nfn main() -> String\n    trim(\"  hi  \")\nend";
        let v = eval_main_tail(src);
        match v {
            Value::Str(s) => assert_eq!(s, "hi"),
            other => panic!("期望 Str(\"hi\")，得到 {:?}", other),
        }
    }

    #[test]
    fn test_import_string_int_to_string() {
        let src = "from string import { int_to_string }\nfn main() -> String\n    int_to_string(42)\nend";
        let v = eval_main_tail(src);
        match v {
            Value::Str(s) => assert_eq!(s, "42"),
            other => panic!("期望 Str(\"42\")，得到 {:?}", other),
        }
    }

    #[test]
    fn test_import_math_sqrt() {
        let src = "from math import { sqrt }\nfn main() -> Float\n    sqrt(16.0)\nend";
        let v = eval_main_tail(src);
        match v {
            Value::Float(x) => assert!((x - 4.0).abs() < 1e-9),
            other => panic!("期望 Float(4.0)，得到 {:?}", other),
        }
    }

    #[test]
    fn test_import_math_abs() {
        let src = "from math import { abs }\nfn main() -> Int\n    abs(-7)\nend";
        let v = eval_main_tail(src);
        match v {
            Value::Int(n) => assert_eq!(n, 7),
            other => panic!("期望 Int(7)，得到 {:?}", other),
        }
    }

    #[test]
    fn test_import_math_min_max() {
        let src = "from math import { min, max }\nfn main() -> Int\n    min(3, 8) + max(3, 8)\nend";
        let v = eval_main_tail(src);
        match v {
            Value::Int(n) => assert_eq!(n, 11), // 3 + 8
            other => panic!("期望 Int(11)，得到 {:?}", other),
        }
    }

    #[test]
    fn test_import_with_alias_call() {
        // 导入 println as log，用 log 调用
        let src = "from io import { println as log }\nfn main() -> Unit\n    log(42)\nend";
        run_src(src).unwrap();
    }

    #[test]
    fn test_import_alias_returns_value() {
        // 别名导入 len as length，验证返回值正确
        let src = "from string import { len as length }\nfn main() -> Int\n    length(\"hello\")\nend";
        let v = eval_main_tail(src);
        match v {
            Value::Int(n) => assert_eq!(n, 5),
            other => panic!("期望 Int(5)，得到 {:?}", other),
        }
    }

    #[test]
    fn test_import_multiple_modules() {
        // 同时导入多个模块
        let src = "from string import { len }\nfrom math import { max }\nfn main() -> Int\n    max(len(\"hi\"), len(\"hello\"))\nend";
        let v = eval_main_tail(src);
        match v {
            Value::Int(n) => assert_eq!(n, 5), // max(2, 5) = 5
            other => panic!("期望 Int(5)，得到 {:?}", other),
        }
    }

    #[test]
    fn test_import_unimported_builtin_errors() {
        // len 未导入，应报"未导入"错误（而非"未定义函数"）
        let src = "fn main() -> Int\n    len(\"hi\")\nend";
        let err = run_src(src).unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("未导入"),
            "期望错误信息含'未导入'，得到: {}",
            msg
        );
        assert!(
            msg.contains("from string import"),
            "期望错误信息含导入提示，得到: {}",
            msg
        );
    }

    #[test]
    fn test_import_unknown_module_errors() {
        let src = "from nonexistent import { foo }\nfn main() -> Unit\n    println(1)\nend";
        let err = run_src(src).unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("未知模块"),
            "期望错误信息含'未知模块'，得到: {}",
            msg
        );
    }

    #[test]
    fn test_import_unknown_symbol_errors() {
        // math 模块不导出 println
        let src = "from math import { println }\nfn main() -> Unit\n    println(1)\nend";
        let err = run_src(src).unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("不导出符号"),
            "期望错误信息含'不导出符号'，得到: {}",
            msg
        );
    }

    #[test]
    fn test_import_dotted_module_errors() {
        // Phase 2.1.5 不支持用户点分模块路径
        let src = "from utils.helpers import { format_date }\nfn main() -> Unit\n    println(1)\nend";
        let err = run_src(src).unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("未知模块"),
            "期望错误信息含'未知模块'，得到: {}",
            msg
        );
    }

    // ===== Phase 2.2 容错解析：Hole 运行时行为 =====

    #[test]
    fn test_hole_statement_runtime_error() {
        // 带洞 AST（含 Stmt::Hole）不应能正常执行——解释器遇到 Hole 报运行时错误。
        // 通过 parse_recover 得到带洞程序，直接执行 main 应失败并提及 hole。
        let src = "fn main() -> Unit\n    let x = +\n    println(1)\nend";
        let result = Parser::parse_recover(src);
        assert!(!result.is_ok(), "此源码应产生解析错误（带洞）");
        // 带洞程序仍可被解释器加载（函数已注册），但执行到 Hole 时报错
        let mut interp = Interpreter::new();
        for item in &result.program.items {
            if let Item::Fn(f) = item {
                interp.functions.insert(f.name.clone(), f.clone());
            }
        }
        let main = interp
            .functions
            .get("main")
            .cloned()
            .expect("应有 fn main");
        let env = Scope::new(Some(interp.globals.clone()));
        let err = match interp.exec_block(&main.body, env) {
            Ok(_) => panic!("带洞程序执行应失败，但成功了"),
            Err(e) => e,
        };
        let msg = format!("{}", err);
        assert!(
            msg.contains("hole") || msg.contains("洞"),
            "期望错误信息提及 hole/洞，得到: {}",
            msg
        );
    }

    // ===== Phase 3.3: list / json 模块集成测试 =====

    #[test]
    fn test_list_empty_and_cons() {
        let src = r#"
from list import { list_empty, list_cons, list_length }
fn main() -> Unit
    let l = list_cons(1, list_cons(2, list_cons(3, list_empty())))
    println(list_length(l))
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_list_get_head_tail() {
        let src = r#"
from list import { list_empty, list_cons, list_get, list_head, list_tail }
fn main() -> Unit
    let l = list_cons(10, list_cons(20, list_cons(30, list_empty())))
    println(list_get(l, 0))
    println(list_get(l, 2))
    println(list_head(l))
    println(list_head(list_tail(l)))
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_list_is_empty() {
        let src = r#"
from list import { list_empty, list_is_empty, list_cons }
fn main() -> Unit
    println(list_is_empty(list_empty()))
    let l = list_cons(1, list_empty())
    println(list_is_empty(l))
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_list_get_out_of_bounds_errors() {
        let src = r#"
from list import { list_empty, list_cons, list_get }
fn main() -> Unit
    let l = list_cons(1, list_empty())
    list_get(l, 5)
end
"#;
        let result = run_src(src);
        assert!(result.is_err(), "越界访问应报错");
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("越界"), "期望提及越界，得到: {}", msg);
    }

    #[test]
    fn test_list_immutable() {
        // 不可变性：list_cons 返回新列表，原列表不变
        let src = r#"
from list import { list_empty, list_cons, list_length }
fn main() -> Unit
    let l1 = list_cons(2, list_cons(3, list_empty()))
    let l2 = list_cons(1, l1)
    println(list_length(l1))
    println(list_length(l2))
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_json_parse_object() {
        let src = r#"
from json import { json_parse }
fn main() -> Unit
    let data = json_parse("{\"name\": \"Alice\", \"age\": 30}")
    println(data.name)
    println(data.age)
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_json_parse_array_to_list() {
        let src = r#"
from json import { json_parse }
from list import { list_length, list_get }
fn main() -> Unit
    let arr = json_parse("[10, 20, 30]")
    println(list_length(arr))
    println(list_get(arr, 1))
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_json_stringify_roundtrip() {
        let src = r#"
from json import { json_parse, json_stringify }
fn main() -> Unit
    let data = json_parse("{\"x\": 1, \"y\": 2}")
    let s = json_stringify(data)
    println(s)
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_json_parse_primitives() {
        let src = r#"
from json import { json_parse }
fn main() -> Unit
    println(json_parse("42"))
    println(json_parse("true"))
    println(json_parse("\"hello\""))
    println(json_parse("null"))
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_json_stringify_record() {
        let src = r#"
from json import { json_stringify }
fn main() -> Unit
    let person = { name: "Bob", age: 25 }
    println(json_stringify(person))
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_json_parse_invalid_errors() {
        let src = r#"
from json import { json_parse }
fn main() -> Unit
    json_parse("{invalid}")
end
"#;
        let result = run_src(src);
        assert!(result.is_err(), "无效 JSON 应报错");
    }

    #[test]
    fn test_list_unimported_errors() {
        let src = r#"
fn main() -> Unit
    let l = list_empty()
    println(l)
end
"#;
        let result = run_src(src);
        assert!(result.is_err(), "未导入的 list_empty 应报错");
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("未导入"), "期望提及未导入，得到: {}", msg);
    }

    // ===== Phase 3.4: string 扩展测试 =====

    #[test]
    fn test_string_split_by_comma() {
        let src = r#"
from string import { split }
from list import { list_length, list_get }
from io import { println }
fn main() -> Unit
    let parts = split("a,b,c", ",")
    println(list_length(parts))
    println(list_get(parts, 0))
    println(list_get(parts, 2))
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_string_split_empty_sep() {
        // 空分隔符按字符分割
        let src = r#"
from string import { split }
from list import { list_length }
from io import { println }
fn main() -> Unit
    let chars = split("hi", "")
    println(list_length(chars))
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_string_contains() {
        let src = r#"
from string import { contains }
from io import { println }
fn main() -> Unit
    println(contains("hello world", "world"))
    println(contains("hello", "xyz"))
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_string_replace_all() {
        let src = r#"
from string import { replace }
from io import { println }
fn main() -> Unit
    println(replace("a-b-c", "-", "+"))
    println(replace("hello", "xyz", "abc"))
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_string_starts_ends_with() {
        let src = r#"
from string import { starts_with, ends_with }
from io import { println }
fn main() -> Unit
    println(starts_with("hello", "he"))
    println(starts_with("hello", "lo"))
    println(ends_with("hello", "lo"))
    println(ends_with("hello", "he"))
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_string_split_wrong_type_errors() {
        let src = r#"
from string import { split }
fn main() -> Unit
    split(123, ",")
end
"#;
        let result = run_src(src);
        assert!(result.is_err(), "split 非 String 参数应报错");
    }

    // ===== Phase 5.1: let 元组解构测试 =====

    #[test]
    fn test_let_destructure_two() {
        let src = r#"
fn pair() -> (Int, Int)
    (1, 2)
end
fn main() -> Unit
    let (a, b) = (10, 20)
    println(a + b)
    let (x, y) = pair()
    println(x * y)
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_let_destructure_three() {
        let src = r#"
fn main() -> Unit
    let (a, b, c) = (1, 2, 3)
    println(a + b + c)
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_let_destructure_nested_call() {
        // 自举场景：解析函数返回 (结果, 剩余输入)
        let src = r#"
fn divmod(a: Int, b: Int) -> (Int, Int)
    (a / b, a - a / b * b)
end
fn main() -> Unit
    let (q, r) = divmod(17, 5)
    println(q)
    println(r)
end
"#;
        run_src(src).unwrap();
    }

    #[test]
    fn test_let_destructure_count_mismatch_errors() {
        let src = r#"
fn main() -> Unit
    let (a, b) = (1, 2, 3)
    println(a)
end
"#;
        let result = run_src(src);
        assert!(result.is_err(), "解构数量不匹配应报错");
    }

    #[test]
    fn test_let_destructure_non_tuple_errors() {
        let src = r#"
fn main() -> Unit
    let (a, b) = 42
    println(a)
end
"#;
        let result = run_src(src);
        assert!(result.is_err(), "解构非元组值应报错");
    }

    // ===== Phase 3.4: file 模块测试 =====

    #[test]
    fn test_file_write_read_roundtrip() {
        let path = "examples/_test_file_roundtrip.txt";
        // 先确保文件不存在
        let _ = std::fs::remove_file(path);
        let src = format!(
            r#"
from file import {{ file_write, file_read, file_exists }}
from io import {{ println }}
fn main() -> Unit
    println(file_exists("{path}"))
    file_write("{path}", "hello\nworld\n")
    println(file_exists("{path}"))
    let content = file_read("{path}")
    println(content)
end
"#,
            path = path
        );
        run_src(&src).unwrap();
        // 清理
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_file_append() {
        let path = "examples/_test_file_append.txt";
        let _ = std::fs::remove_file(path);
        let src = format!(
            r#"
from file import {{ file_write, file_append, file_read }}
from io import {{ println }}
fn main() -> Unit
    file_write("{path}", "line1\n")
    file_append("{path}", "line2\n")
    let content = file_read("{path}")
    println(content)
end
"#,
            path = path
        );
        run_src(&src).unwrap();
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_file_read_nonexistent_errors() {
        let src = r#"
from file import { file_read }
fn main() -> Unit
    file_read("examples/__nonexistent_file__.txt")
end
"#;
        let result = run_src(src);
        assert!(result.is_err(), "读取不存在的文件应报错");
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("file_read"), "期望提及 file_read，得到: {}", msg);
    }

    #[test]
    fn test_file_wrong_type_errors() {
        let src = r#"
from file import { file_exists }
fn main() -> Unit
    file_exists(123)
end
"#;
        let result = run_src(src);
        assert!(result.is_err(), "file_exists 非 String 参数应报错");
    }

    #[test]
    fn test_file_unimported_errors() {
        let src = r#"
fn main() -> Unit
    file_exists("test.txt")
end
"#;
        let result = run_src(src);
        assert!(result.is_err(), "未导入的 file_exists 应报错");
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("未导入"), "期望提及未导入，得到: {}", msg);
    }
}
