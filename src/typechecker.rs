// Lom Type Checker — Phase 2.4 渐进式类型检查器
//
// 设计目标：
//   1. 渐进式：默认动态可跑（不强制类型检查）；--check 时执行检查
//   2. 两遍分析：第一遍收集签名（fn/enum），第二遍检查函数体
//   3. 类型错误用 warning 级别（可跑但提示问题），签名冲突等用 error
//   4. 结构等价：记录字段顺序不敏感
//   5. 不做：trait 解析（推迟至 Phase 3+）、泛型实例化（仅占位符兼容性）、元组索引精确推断
//      注：Phase 2.5 已实现效应检查（EFF001）；Phase 2.6 已实现类型信息导出（lom info，独立模块）
//
// 类型检查范围：
//   - 字面量类型推断
//   - 变量绑定与查找（未定义变量 → NAM003）
//   - 二元运算类型检查（TYPE001 类型不匹配）
//   - 函数调用参数匹配（TYPE003 参数数量/类型不符）
//   - 返回类型与声明匹配（TYPE010 返回类型不符）
//   - `?` 运算符：操作数必须是 Result/Option，且所在函数返回兼容类型（TYPE020）
//   - match 穷尽性：用户枚举必须穷尽或带 `_`（MAT001）
//   - 记录/元组字段访问
//
// 错误码体系：
//   NAM001-099  名称解析（NAM003 未定义变量）
//   TYPE001-099 类型错误（TYPE001 不匹配、TYPE003 参数、TYPE010 返回、TYPE020 ? 误用）
//   MAT001-099  match 穷尽性（MAT001 非穷尽）

use crate::ast::*;
use crate::diagnostics::{Diagnostic, Diagnostics, Severity, Stage};
use std::collections::HashMap;

/// 类型检查器
pub struct TypeChecker {
    /// 函数签名表：name -> (param types, ret type)
    /// ret_type 为 None 表示函数无返回类型注解（动态）
    functions: HashMap<String, FnSig>,
    /// 枚举定义：enum name -> variants
    /// 内置 Result/Option 也注册在此
    enums: HashMap<String, EnumInfo>,
    /// 当前所在函数的返回类型（用于 return 语句和 `?` 检查）
    current_ret: Option<Type>,
    /// Phase 2.5: 当前所在函数声明的效应集合
    /// 用于检查函数体内的调用是否越界使用效应（EFF001）
    current_effects: Vec<Effect>,
    /// Phase 2.5: 当前函数是否是 main（入口函数隐式拥有所有效应，跳过 EFF001 检查）
    /// 设计理由：main 是程序入口，调用 println 等副作用是常态；
    /// 强制 main 声明 `! [IO]` 会给 LLM 增加无意义负担，违反 LLM-coding-native 容错原则。
    current_fn_is_main: bool,
    /// Phase 3.2: 当前函数签名 span（来自 FnDecl.span，用于 EFF001/TYPE010 诊断定位）
    current_fn_span: Span,
    /// 已收集的诊断
    diags: Vec<Diagnostic>,
    /// 文件名（用于诊断）
    file: String,
    /// 源码行（用于诊断的 source_line 字段）
    source_lines: Vec<String>,
}

/// 函数签名
#[derive(Clone)]
struct FnSig {
    params: Vec<(String, Type)>,
    ret: Option<Type>,
    /// Phase 2.5: 函数声明的效应列表
    /// 空 Vec 表示纯函数（无 `! [...]` 注解）
    effects: Vec<Effect>,
    /// Phase 3.2: 函数签名 span（来自 FnDecl.span，用于 EFF001/TYPE010/NAM002 诊断定位）
    /// 替代 Phase 3.1 的 sig_line hack（find_fn_line 扫描源码）
    span: Span,
}

/// 枚举信息
#[derive(Clone)]
struct EnumInfo {
    /// 是否是内置的 Result/Option（用于穷尽性检查时知道有哪些变体）
    is_builtin: bool,
    /// 变体列表：(变体名, 参数类型列表)
    /// 内置：Result=[Ok(T), Err(E)], Option=[Some(T), None]
    variants: Vec<(String, Vec<Type>)>,
    /// 类型参数（如 Result 的 T, E）
    type_params: Vec<String>,
}

/// 类型推断结果
///
/// 渐进式语义：Unknown 表示无法推断（不报错，跳过后续检查）
#[derive(Clone, Debug, PartialEq)]
enum TypeOrUnknown {
    Known(Type),
    Unknown,
}

impl TypeOrUnknown {
    fn known(t: Type) -> Self {
        TypeOrUnknown::Known(t)
    }
    fn unknown() -> Self {
        TypeOrUnknown::Unknown
    }
    fn as_type(&self) -> Option<&Type> {
        match self {
            TypeOrUnknown::Known(t) => Some(t),
            TypeOrUnknown::Unknown => None,
        }
    }
}

impl TypeChecker {
    pub fn new(file: &str, source_lines: Vec<String>) -> Self {
        let mut tc = TypeChecker {
            functions: HashMap::new(),
            enums: HashMap::new(),
            current_ret: None,
            current_effects: Vec::new(),
            current_fn_is_main: false,
            current_fn_span: Span::default(),
            diags: Vec::new(),
            file: file.to_string(),
            source_lines,
        };
        tc.register_builtins();
        tc
    }

    /// 注册内置类型（Result/Option 及其变体）+ prelude/标准库函数签名
    fn register_builtins(&mut self) {
        self.enums.insert(
            "Result".to_string(),
            EnumInfo {
                is_builtin: true,
                variants: vec![
                    ("Ok".to_string(), vec![Type::Generic("T".to_string(), vec![])]),
                    ("Err".to_string(), vec![Type::Generic("E".to_string(), vec![])]),
                ],
                type_params: vec!["T".to_string(), "E".to_string()],
            },
        );
        self.enums.insert(
            "Option".to_string(),
            EnumInfo {
                is_builtin: true,
                variants: vec![
                    ("Some".to_string(), vec![Type::Generic("T".to_string(), vec![])]),
                    ("None".to_string(), vec![]),
                ],
                type_params: vec!["T".to_string()],
            },
        );
        // Prelude 函数（自动可用，无需 import）
        // Phase 2.5: println/print 声明 IO 效应
        self.functions.insert(
            "println".to_string(),
            FnSig { params: vec![("_".to_string(), Type::Named("_Any".to_string()))], ret: Some(Type::Unit), effects: vec!["IO".to_string()], span: Span::default() },
        );
        self.functions.insert(
            "print".to_string(),
            FnSig { params: vec![("_".to_string(), Type::Named("_Any".to_string()))], ret: Some(Type::Unit), effects: vec!["IO".to_string()], span: Span::default() },
        );
        // string 模块（纯函数）
        self.functions.insert(
            "len".to_string(),
            FnSig { params: vec![("s".to_string(), Type::String)], ret: Some(Type::Int), effects: vec![], span: Span::default() },
        );
        self.functions.insert(
            "int_to_string".to_string(),
            FnSig { params: vec![("n".to_string(), Type::Int)], ret: Some(Type::String), effects: vec![], span: Span::default() },
        );
        self.functions.insert(
            "string_to_int".to_string(),
            FnSig { params: vec![("s".to_string(), Type::String)], ret: Some(Type::Named("_Any".to_string())), effects: vec![], span: Span::default() },
        );
        self.functions.insert(
            "trim".to_string(),
            FnSig { params: vec![("s".to_string(), Type::String)], ret: Some(Type::String), effects: vec![], span: Span::default() },
        );
        self.functions.insert(
            "upper".to_string(),
            FnSig { params: vec![("s".to_string(), Type::String)], ret: Some(Type::String), effects: vec![], span: Span::default() },
        );
        self.functions.insert(
            "lower".to_string(),
            FnSig { params: vec![("s".to_string(), Type::String)], ret: Some(Type::String), effects: vec![], span: Span::default() },
        );
        // math 模块（纯函数）
        self.functions.insert(
            "sqrt".to_string(),
            FnSig { params: vec![("x".to_string(), Type::Named("_Any".to_string()))], ret: Some(Type::Float), effects: vec![], span: Span::default() },
        );
        self.functions.insert(
            "abs".to_string(),
            FnSig { params: vec![("x".to_string(), Type::Named("_Any".to_string()))], ret: Some(Type::Named("_Any".to_string())), effects: vec![], span: Span::default() },
        );
        self.functions.insert(
            "min".to_string(),
            FnSig { params: vec![("a".to_string(), Type::Named("_Any".to_string())), ("b".to_string(), Type::Named("_Any".to_string()))], ret: Some(Type::Named("_Any".to_string())), effects: vec![], span: Span::default() },
        );
        self.functions.insert(
            "max".to_string(),
            FnSig { params: vec![("a".to_string(), Type::Named("_Any".to_string())), ("b".to_string(), Type::Named("_Any".to_string()))], ret: Some(Type::Named("_Any".to_string())), effects: vec![], span: Span::default() },
        );
        // Phase 3.3: list 模块（纯函数，不可变语义）
        // List<T> 用 Type::Generic("List", [T]) 表示；签名用 List<_Any> 接受任何元素类型
        let list_any = || Type::Generic("List".to_string(), vec![Type::Named("_Any".to_string())]);
        self.functions.insert(
            "list_empty".to_string(),
            FnSig { params: vec![], ret: Some(list_any()), effects: vec![], span: Span::default() },
        );
        self.functions.insert(
            "list_length".to_string(),
            FnSig { params: vec![("list".to_string(), list_any())], ret: Some(Type::Int), effects: vec![], span: Span::default() },
        );
        self.functions.insert(
            "list_get".to_string(),
            FnSig { params: vec![("list".to_string(), list_any()), ("idx".to_string(), Type::Int)], ret: Some(Type::Named("_Any".to_string())), effects: vec![], span: Span::default() },
        );
        self.functions.insert(
            "list_is_empty".to_string(),
            FnSig { params: vec![("list".to_string(), list_any())], ret: Some(Type::Bool), effects: vec![], span: Span::default() },
        );
        self.functions.insert(
            "list_head".to_string(),
            FnSig { params: vec![("list".to_string(), list_any())], ret: Some(Type::Named("_Any".to_string())), effects: vec![], span: Span::default() },
        );
        self.functions.insert(
            "list_tail".to_string(),
            FnSig { params: vec![("list".to_string(), list_any())], ret: Some(list_any()), effects: vec![], span: Span::default() },
        );
        self.functions.insert(
            "list_cons".to_string(),
            FnSig { params: vec![("head".to_string(), Type::Named("_Any".to_string())), ("list".to_string(), list_any())], ret: Some(list_any()), effects: vec![], span: Span::default() },
        );
        // Phase 3.3: json 模块（纯函数）
        // json_parse 返回 _Any（可能是 Record/List/Int/Float/Bool/Str/Unit）
        // json_stringify 接受任何值，返回 String
        self.functions.insert(
            "json_parse".to_string(),
            FnSig { params: vec![("s".to_string(), Type::String)], ret: Some(Type::Named("_Any".to_string())), effects: vec![], span: Span::default() },
        );
        self.functions.insert(
            "json_stringify".to_string(),
            FnSig { params: vec![("v".to_string(), Type::Named("_Any".to_string()))], ret: Some(Type::String), effects: vec![], span: Span::default() },
        );
        // Phase 3.4: string 扩展（纯函数）
        // split(s, sep) -> List<String>；返回 List<_Any>（元素类型追踪推迟）
        let list_string = || Type::Generic("List".to_string(), vec![Type::Named("_Any".to_string())]);
        self.functions.insert(
            "split".to_string(),
            FnSig { params: vec![("s".to_string(), Type::String), ("sep".to_string(), Type::String)], ret: Some(list_string()), effects: vec![], span: Span::default() },
        );
        self.functions.insert(
            "contains".to_string(),
            FnSig { params: vec![("s".to_string(), Type::String), ("sub".to_string(), Type::String)], ret: Some(Type::Bool), effects: vec![], span: Span::default() },
        );
        self.functions.insert(
            "replace".to_string(),
            FnSig { params: vec![("s".to_string(), Type::String), ("from".to_string(), Type::String), ("to".to_string(), Type::String)], ret: Some(Type::String), effects: vec![], span: Span::default() },
        );
        self.functions.insert(
            "starts_with".to_string(),
            FnSig { params: vec![("s".to_string(), Type::String), ("prefix".to_string(), Type::String)], ret: Some(Type::Bool), effects: vec![], span: Span::default() },
        );
        self.functions.insert(
            "ends_with".to_string(),
            FnSig { params: vec![("s".to_string(), Type::String), ("suffix".to_string(), Type::String)], ret: Some(Type::Bool), effects: vec![], span: Span::default() },
        );
        // Phase 3.4: file 模块（均声明 [IO] 效应）
        // file_read/file_write/file_append/file_exists 都涉及文件系统副作用
        let io_effect = vec!["IO".to_string()];
        self.functions.insert(
            "file_read".to_string(),
            FnSig { params: vec![("path".to_string(), Type::String)], ret: Some(Type::String), effects: io_effect.clone(), span: Span::default() },
        );
        self.functions.insert(
            "file_write".to_string(),
            FnSig { params: vec![("path".to_string(), Type::String), ("content".to_string(), Type::String)], ret: Some(Type::Unit), effects: io_effect.clone(), span: Span::default() },
        );
        self.functions.insert(
            "file_append".to_string(),
            FnSig { params: vec![("path".to_string(), Type::String), ("content".to_string(), Type::String)], ret: Some(Type::Unit), effects: io_effect.clone(), span: Span::default() },
        );
        self.functions.insert(
            "file_exists".to_string(),
            FnSig { params: vec![("path".to_string(), Type::String)], ret: Some(Type::Bool), effects: io_effect, span: Span::default() },
        );
        // Phase 3.5: env 模块
        // args() -> List<String>；返回命令行参数（argv[0] = .lom 文件路径）
        // 纯函数（读取解释器内部状态，无副作用）
        let list_string = || Type::Generic("List".to_string(), vec![Type::Named("_Any".to_string())]);
        self.functions.insert(
            "args".to_string(),
            FnSig { params: vec![], ret: Some(list_string()), effects: vec![], span: Span::default() },
        );
    }

    /// 入口：检查整个程序，返回填充了类型诊断的 Diagnostics
    pub fn check(mut self, program: &Program, diags: &mut Diagnostics) {
        // 第一遍：收集函数签名、枚举定义、导入别名
        for item in &program.items {
            match item {
                Item::Fn(f) => self.collect_fn_sig(f),
                Item::Enum(e) => self.collect_enum(e),
                Item::Import(imp) => self.collect_import(imp),
            }
        }
        // 第二遍：检查每个函数体
        for item in &program.items {
            if let Item::Fn(f) = item {
                self.check_fn_body(f);
            }
        }
        // 把收集到的诊断合并到 Diagnostics
        for d in self.diags.drain(..) {
            let is_error = d.severity == Severity::Error;
            diags.diagnostics.push(d);
            if is_error {
                diags.ok = false;
            }
        }
    }

    // ===== 第一遍：签名收集 =====

    fn collect_fn_sig(&mut self, f: &FnDecl) {
        if self.functions.contains_key(&f.name) {
            self.push_diag(
                Severity::Error,
                "NAM002".into(),
                format!("函数 '{}' 重复定义", f.name),
                f.span.line,
                f.span.col,
            );
            return;
        }
        let params: Vec<(String, Type)> = f
            .params
            .iter()
            .map(|p| (p.name.clone(), p.ty.clone()))
            .collect();
        // Phase 3.2: 直接用 FnDecl.span（parser 填充），替代 Phase 3.1 的 find_fn_line 扫描 hack
        self.functions.insert(
            f.name.clone(),
            FnSig {
                params,
                ret: f.ret_type.clone(),
                effects: f.effects.clone(),
                span: f.span,
            },
        );
    }

    fn collect_enum(&mut self, e: &EnumDecl) {
        if self.enums.contains_key(&e.name) {
            self.push_diag(
                Severity::Error,
                "NAM002".into(),
                format!("枚举 '{}' 重复定义", e.name),
                0,
                0,
            );
            return;
        }
        let variants: Vec<(String, Vec<Type>)> = e
            .variants
            .iter()
            .map(|v| (v.name.clone(), v.fields.clone()))
            .collect();
        self.enums.insert(
            e.name.clone(),
            EnumInfo {
                is_builtin: false,
                variants,
                type_params: e.type_params.clone(),
            },
        );
        // 同时把变体名注册为可调用的构造器（无独立符号表，靠 enums 表查询）
    }

    /// 收集 import 声明：把别名注册到函数签名表
    /// （别名继承真实函数的签名；符号是否在模块导出由解释器运行时检查，typechecker 不重复报错）
    fn collect_import(&mut self, imp: &ImportDecl) {
        for item in &imp.items {
            // 仅当真实名已注册（prelude/stdlib）时，才注册别名
            if let Some(sig) = self.functions.get(&item.name).cloned() {
                self.functions.insert(item.alias.clone(), sig);
            }
        }
    }

    // ===== 第二遍：函数体检查 =====

    fn check_fn_body(&mut self, f: &FnDecl) {
        self.current_ret = f.ret_type.clone();
        // Phase 2.5: 进入函数体时记录声明的效应集合
        self.current_effects = f.effects.clone();
        // main 函数隐式拥有所有效应（跳过 EFF001 检查）
        self.current_fn_is_main = f.name == "main";
        // Phase 3.2: 记录函数签名 span（来自 FnDecl.span，用于 EFF001/TYPE010 诊断定位）
        // 替代 Phase 3.1 的 sig_line hack（find_fn_line 扫描源码）
        self.current_fn_span = f.span;
        // 构建初始环境：参数 + 预导入符号
        let mut env = TypeEnv::new();
        for p in &f.params {
            env.define(p.name.clone(), TypeOrUnknown::Known(p.ty.clone()));
        }
        // 检查函数体
        let body_ty = self.check_block(&f.body, &mut env);
        // 检查返回类型匹配
        if let Some(ret_ty) = &f.ret_type {
            if let TypeOrUnknown::Known(bt) = &body_ty {
                if !self.types_compatible(bt, ret_ty) {
                    self.push_diag(
                        Severity::Warning,
                        "TYPE010".into(),
                        format!(
                            "函数 '{}' 声明返回 {:?}，但实际返回 {:?}",
                            f.name, ret_ty, bt
                        ),
                        self.current_fn_span.line,
                        self.current_fn_span.col,
                    );
                }
            }
        }
        self.current_ret = None;
        self.current_effects.clear();
        self.current_fn_is_main = false;
        self.current_fn_span = Span::default();
    }

    /// 检查块，返回块的类型（尾表达式类型或 Unit）
    fn check_block(&mut self, block: &Block, env: &mut TypeEnv) -> TypeOrUnknown {
        for stmt in &block.stmts {
            self.check_stmt(stmt, env);
        }
        match &block.tail {
            Some(e) => self.check_expr(e, env),
            None => TypeOrUnknown::known(Type::Unit),
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt, env: &mut TypeEnv) {
        match stmt {
            Stmt::Let { name, ty, value, .. } => {
                let val_ty = self.check_expr(value, env);
                if let (Some(annot), TypeOrUnknown::Known(vt)) = (ty, &val_ty) {
                    if !self.types_compatible(vt, annot) {
                        self.push_diag(
                            Severity::Warning,
                            "TYPE001".into(),
                            format!(
                                "let {} 声明类型 {:?}，但值类型 {:?}",
                                name, annot, vt
                            ),
                            0,
                            0,
                        );
                    }
                }
                let final_ty = if let Some(annot) = ty {
                    TypeOrUnknown::Known(annot.clone())
                } else {
                    val_ty
                };
                env.define(name.clone(), final_ty);
            }
            Stmt::LetDestruct { names, value } => {
                // Phase 5.1: 元组解构绑定 let (a, b) = expr
                let val_ty = self.check_expr(value, env);
                match val_ty {
                    TypeOrUnknown::Known(Type::Tuple(elem_tys)) => {
                        if elem_tys.len() != names.len() {
                            self.push_diag(
                                Severity::Warning,
                                "TYPE001".into(),
                                format!(
                                    "元组解构数量不匹配：模式含 {} 个名字，值含 {} 个元素",
                                    names.len(),
                                    elem_tys.len()
                                ),
                                0,
                                0,
                            );
                        }
                        for (name, ety) in names.iter().zip(elem_tys.iter()) {
                            env.define(name.clone(), TypeOrUnknown::Known(ety.clone()));
                        }
                    }
                    // 未知类型（动态值/函数调用返回）：绑 Unknown，运行时检查兜底
                    _ => {
                        for name in names {
                            env.define(name.clone(), TypeOrUnknown::Unknown);
                        }
                    }
                }
            }
            Stmt::Assign { target, value } => {
                let val_ty = self.check_expr(value, env);
                if let Some(expected) = env.get(target) {
                    if let (TypeOrUnknown::Known(e), TypeOrUnknown::Known(v)) = (&expected, &val_ty)
                    {
                        if !self.types_compatible(v, e) {
                            self.push_diag(
                                Severity::Warning,
                                "TYPE001".into(),
                                format!(
                                    "赋值给 '{}': 期望 {:?}，得到 {:?}",
                                    target, e, v
                                ),
                                0,
                                0,
                            );
                        }
                    }
                } else {
                    self.push_diag(
                        Severity::Error,
                        "NAM003".into(),
                        format!("赋值给未定义变量 '{}'", target),
                        0,
                        0,
                    );
                }
            }
            Stmt::If(if_stmt) => {
                for (cond, body) in &if_stmt.branches {
                    let cond_ty = self.check_expr(cond, env);
                    if let TypeOrUnknown::Known(t) = &cond_ty {
                        if !matches!(t, Type::Bool) {
                            self.push_diag(
                                Severity::Warning,
                                "TYPE002".into(),
                                format!("if 条件应为 Bool，得到 {:?}", t),
                                0,
                                0,
                            );
                        }
                    }
                    self.check_block(body, env);
                }
                if let Some(else_b) = &if_stmt.else_branch {
                    self.check_block(else_b, env);
                }
            }
            Stmt::While { cond, body } => {
                let cond_ty = self.check_expr(cond, env);
                if let TypeOrUnknown::Known(t) = &cond_ty {
                    if !matches!(t, Type::Bool) {
                        self.push_diag(
                            Severity::Warning,
                            "TYPE002".into(),
                            format!("while 条件应为 Bool，得到 {:?}", t),
                            0,
                            0,
                        );
                    }
                }
                self.check_block(body, env);
            }
            Stmt::For { var, iter, body } => {
                let iter_ty = self.check_expr(iter, env);
                // for x in String -> x: String; for x in Int -> x: Int (0..n)
                let elem_ty = match &iter_ty {
                    TypeOrUnknown::Known(Type::String) => TypeOrUnknown::known(Type::String),
                    TypeOrUnknown::Known(Type::Int) => TypeOrUnknown::known(Type::Int),
                    _ => TypeOrUnknown::unknown(),
                };
                env.define(var.clone(), elem_ty);
                self.check_block(body, env);
            }
            Stmt::Return(expr) => {
                let ret_ty = match expr {
                    Some(e) => self.check_expr(e, env),
                    None => TypeOrUnknown::known(Type::Unit),
                };
                if let (Some(expected), TypeOrUnknown::Known(actual)) = (&self.current_ret, &ret_ty)
                {
                    if !self.types_compatible(actual, expected) {
                        self.push_diag(
                            Severity::Warning,
                            "TYPE010".into(),
                            format!(
                                "return 返回 {:?}，但函数声明返回 {:?}",
                                actual, expected
                            ),
                            0,
                            0,
                        );
                    }
                }
            }
            Stmt::Expr(e) => {
                self.check_expr(e, env);
            }
            Stmt::Hole { line, col } => {
                // Hole 是 Phase 2.2 容错解析的产物，不重复报错
                let _ = (line, col);
            }
        }
    }

    /// 检查表达式，返回其类型
    fn check_expr(&mut self, expr: &Expr, env: &mut TypeEnv) -> TypeOrUnknown {
        match expr {
            Expr::Int(_) => TypeOrUnknown::known(Type::Int),
            Expr::Float(_) => TypeOrUnknown::known(Type::Float),
            Expr::Bool(_) => TypeOrUnknown::known(Type::Bool),
            Expr::Str(_) => TypeOrUnknown::known(Type::String),
            Expr::Unit => TypeOrUnknown::known(Type::Unit),
            Expr::Ident(name) => {
                if let Some(ty) = env.get(name) {
                    ty
                } else if self.functions.contains_key(name) {
                    // 顶层函数引用（作为值，Phase 2.4 暂不深入检查函数类型）
                    TypeOrUnknown::unknown()
                } else if self.is_variant_constructor(name) {
                    // 枚举变体构造器
                    self.variant_return_type(name)
                } else {
                    self.push_diag(
                        Severity::Error,
                        "NAM003".into(),
                        format!("未定义变量 '{}'", name),
                        0,
                        0,
                    );
                    self.patch_nam003_hint(name, &*env);
                    TypeOrUnknown::unknown()
                }
            }
            Expr::Binary { op, left, right } => {
                let lt = self.check_expr(left, env);
                let rt = self.check_expr(right, env);
                self.binary_result_type(op, &lt, &rt)
            }
            Expr::Unary { op, expr } => {
                let t = self.check_expr(expr, env);
                match op {
                    UnaryOp::Neg => match &t {
                        TypeOrUnknown::Known(Type::Int) => TypeOrUnknown::known(Type::Int),
                        TypeOrUnknown::Known(Type::Float) => TypeOrUnknown::known(Type::Float),
                        TypeOrUnknown::Known(other) => {
                            self.push_diag(
                                Severity::Warning,
                                "TYPE001".into(),
                                format!("一元负号要求 Int/Float，得到 {:?}", other),
                                0,
                                0,
                            );
                            TypeOrUnknown::unknown()
                        }
                        TypeOrUnknown::Unknown => TypeOrUnknown::unknown(),
                    },
                    UnaryOp::Not => match &t {
                        TypeOrUnknown::Known(Type::Bool) => TypeOrUnknown::known(Type::Bool),
                        TypeOrUnknown::Known(other) => {
                            self.push_diag(
                                Severity::Warning,
                                "TYPE001".into(),
                                format!("一元非要求 Bool，得到 {:?}", other),
                                0,
                                0,
                            );
                            TypeOrUnknown::unknown()
                        }
                        TypeOrUnknown::Unknown => TypeOrUnknown::unknown(),
                    },
                }
            }
            Expr::Logical { .. } => {
                // and/or 短路求值，结果为 Bool；不强制检查操作数（渐进式）
                TypeOrUnknown::known(Type::Bool)
            }
            Expr::Call { callee, args } => {
                self.check_call(callee, args, env)
            }
            Expr::Index { expr, index } => {
                let _ = self.check_expr(expr, env);
                let _ = self.check_expr(index, env);
                // 元组索引在 Phase 2.4 暂不精确推断（Index 节点未在解释器实现）
                TypeOrUnknown::unknown()
            }
            Expr::Field { expr, name: field } => {
                let t = self.check_expr(expr, env);
                match &t {
                    TypeOrUnknown::Known(Type::Record(fields)) => {
                        for (fname, fty) in fields {
                            if fname == field {
                                return TypeOrUnknown::Known(fty.clone());
                            }
                        }
                        self.push_diag(
                            Severity::Error,
                            "NAM004".into(),
                            format!("记录无字段 '{}'", field),
                            0,
                            0,
                        );
                        TypeOrUnknown::unknown()
                    }
                    TypeOrUnknown::Known(Type::Tuple(_)) => {
                        // 元组通过 .0/.1 访问，但当前 parser 把 .0 解析为 Field？
                        // 实际：Field 节点的 name 是字符串，元组索引用 Field.name="0" 表示
                        // 简化：不精确检查
                        TypeOrUnknown::unknown()
                    }
                    _ => TypeOrUnknown::unknown(),
                }
            }
            Expr::Group(e) => self.check_expr(e, env),
            Expr::If(if_stmt) => {
                let mut branch_tys = Vec::new();
                for (cond, body) in &if_stmt.branches {
                    let cond_ty = self.check_expr(cond, env);
                    if let TypeOrUnknown::Known(t) = &cond_ty {
                        if !matches!(t, Type::Bool) {
                            self.push_diag(
                                Severity::Warning,
                                "TYPE002".into(),
                                format!("if 条件应为 Bool，得到 {:?}", t),
                                0,
                                0,
                            );
                        }
                    }
                    branch_tys.push(self.check_block(body, env));
                }
                if let Some(else_b) = &if_stmt.else_branch {
                    branch_tys.push(self.check_block(else_b, env));
                }
                // 取所有分支的公共类型（若一致）
                self.unify_types(&branch_tys)
            }
            Expr::Closure { params, ret_type, body } => {
                // 闭包捕获外部环境：closure_env 继承当前 env，使闭包内可引用外部变量
                let mut closure_env = env.child();
                for p in params {
                    closure_env.define(p.name.clone(), TypeOrUnknown::Known(p.ty.clone()));
                }
                let saved_ret = self.current_ret.clone();
                self.current_ret = ret_type.clone();
                let body_ty = self.check_block(body, &mut closure_env);
                self.current_ret = saved_ret;
                // 检查返回类型匹配
                if let (Some(ret), TypeOrUnknown::Known(bt)) = (ret_type, &body_ty) {
                    if !self.types_compatible(bt, ret) {
                        self.push_diag(
                            Severity::Warning,
                            "TYPE010".into(),
                            format!("闭包返回 {:?}，但声明返回 {:?}", bt, ret),
                            0,
                            0,
                        );
                    }
                }
                TypeOrUnknown::unknown()
            }
            Expr::Match(m) => self.check_match(m, env),
            Expr::Try(e) => {
                let inner_ty = self.check_expr(e, env);
                match &inner_ty {
                    TypeOrUnknown::Known(Type::Result(ok_t, err_t)) => {
                        // ? on Result<T, E> yields T; requires enclosing fn to return Result<_, E>
                        if let Some(ret) = &self.current_ret {
                            if !self.result_compatible(ret, ok_t, err_t) {
                                self.push_diag(
                                    Severity::Warning,
                                    "TYPE020".into(),
                                    format!(
                                        "`?` 用于 Result<{:?}, {:?}>，但所在函数返回 {:?}",
                                        ok_t, err_t, ret
                                    ),
                                    0,
                                    0,
                                );
                            }
                        }
                        TypeOrUnknown::Known((**ok_t).clone())
                    }
                    TypeOrUnknown::Known(Type::Option(t)) => {
                        if let Some(ret) = &self.current_ret {
                            if !self.option_compatible(ret, t) {
                                self.push_diag(
                                    Severity::Warning,
                                    "TYPE020".into(),
                                    format!(
                                        "`?` 用于 Option<{:?}>，但所在函数返回 {:?}",
                                        t, ret
                                    ),
                                    0,
                                    0,
                                );
                            }
                        }
                        TypeOrUnknown::Known((**t).clone())
                    }
                    TypeOrUnknown::Known(other) => {
                        self.push_diag(
                            Severity::Warning,
                            "TYPE020".into(),
                            format!("`?` 要求 Result/Option，得到 {:?}", other),
                            0,
                            0,
                        );
                        TypeOrUnknown::unknown()
                    }
                    TypeOrUnknown::Unknown => TypeOrUnknown::unknown(),
                }
            }
            Expr::Pipe { left, right } => {
                let left_ty = self.check_expr(left, env);
                // right 应该是 callee（Ident 或 Call）；把 left_ty 作为第一个参数
                if let Expr::Call { callee, args } = right.as_ref() {
                    let mut all_args = vec![left_ty];
                    for a in args {
                        all_args.push(self.check_expr(a, env));
                    }
                    self.check_call(callee, &all_args_args_to_exprs(args), env);
                    let _ = all_args;
                    // 返回 callee 的返回类型
                    self.callee_return_type(callee, env)
                } else {
                    // x |> f => f(x)
                    self.check_call(right, std::slice::from_ref(left), env)
                }
            }
            Expr::Record { fields } => {
                let mut field_tys = Vec::new();
                for (name, e) in fields {
                    let t = self.check_expr(e, env);
                    field_tys.push((name.clone(), t));
                }
                // 转为 Known Type::Record
                let known_fields: Vec<(String, Type)> = field_tys
                    .into_iter()
                    .filter_map(|(n, t)| match t {
                        TypeOrUnknown::Known(ty) => Some((n, ty)),
                        TypeOrUnknown::Unknown => None,
                    })
                    .collect();
                if known_fields.len() == fields.len() {
                    TypeOrUnknown::known(Type::Record(known_fields))
                } else {
                    TypeOrUnknown::unknown()
                }
            }
            Expr::Tuple { elems } => {
                let mut elem_tys = Vec::new();
                for e in elems {
                    elem_tys.push(self.check_expr(e, env));
                }
                if elem_tys.iter().all(|t| matches!(t, TypeOrUnknown::Known(_))) {
                    let tys: Vec<Type> = elem_tys
                        .into_iter()
                        .map(|t| match t {
                            TypeOrUnknown::Known(ty) => ty,
                            TypeOrUnknown::Unknown => unreachable!(),
                        })
                        .collect();
                    TypeOrUnknown::known(Type::Tuple(tys))
                } else {
                    TypeOrUnknown::unknown()
                }
            }
        }
    }

    /// 检查函数调用
    fn check_call(&mut self, callee: &Expr, args: &[Expr], env: &mut TypeEnv) -> TypeOrUnknown {
        // 先检查参数表达式（副作用：更新环境中的变量类型）
        let mut arg_tys: Vec<TypeOrUnknown> = Vec::new();
        for a in args {
            arg_tys.push(self.check_expr(a, env));
        }
        // 解析 callee
        match callee {
            Expr::Ident(name) => {
                // 顶层函数？
                if let Some(sig) = self.functions.get(name).cloned() {
                    // Phase 2.5: 效应检查
                    // 当前函数未声明的效应，不能调用带该效应的函数（EFF001，Warning，渐进式）
                    self.check_call_effects(name, &sig.effects);
                    // 参数数量检查
                    if sig.params.len() != args.len() {
                        self.push_diag(
                            Severity::Warning,
                            "TYPE003".into(),
                            format!(
                                "函数 '{}' 期望 {} 个参数，得到 {} 个",
                                name,
                                sig.params.len(),
                                args.len()
                            ),
                            0,
                            0,
                        );
                    } else {
                        // 参数类型检查
                        for (i, ((pname, pty), arg_ty)) in
                            sig.params.iter().zip(arg_tys.iter()).enumerate()
                        {
                            if let TypeOrUnknown::Known(at) = arg_ty {
                                if !self.types_compatible(at, pty) {
                                    self.push_diag(
                                        Severity::Warning,
                                        "TYPE003".into(),
                                        format!(
                                            "函数 '{}' 参数 {}: 期望 {:?}，得到 {:?}",
                                            name, i, pty, at
                                        ),
                                        0,
                                        0,
                                    );
                                }
                            }
                            let _ = pname;
                        }
                    }
                    sig.ret.map(TypeOrUnknown::Known).unwrap_or(TypeOrUnknown::unknown())
                } else if self.is_variant_constructor(name) {
                    // 枚举变体构造器
                    let (expected_arity, ret_ty) = self.variant_info(name);
                    if expected_arity != args.len() {
                        self.push_diag(
                            Severity::Warning,
                            "TYPE003".into(),
                            format!(
                                "变体 '{}' 期望 {} 个参数，得到 {} 个",
                                name, expected_arity, args.len()
                            ),
                            0,
                            0,
                        );
                    }
                    ret_ty
                } else if env.get(name).is_some() {
                    // 局部变量（可能是闭包）— 不深入检查
                    TypeOrUnknown::unknown()
                } else {
                    self.push_diag(
                        Severity::Error,
                        "NAM003".into(),
                        format!("调用未定义函数 '{}'", name),
                        0,
                        0,
                    );
                    self.patch_nam003_hint(name, &*env);
                    TypeOrUnknown::unknown()
                }
            }
            _ => {
                // callee 是复杂表达式（如闭包字面量、管道结果）
                self.check_expr(callee, env)
            }
        }
    }

    /// Phase 2.5: 检查函数调用的效应合规性
    ///
    /// 规则：被调函数声明的每个效应，必须出现在当前函数的效应集合中。
    /// 否则报 EFF001（Warning，渐进式：不阻止运行）。
    ///
    /// 例外：
    /// - main 函数隐式拥有所有效应（跳过检查），因为入口函数调用 IO 是常态
    /// - 闭包不进行效应检查（current_effects 在闭包内不重置，继承外层）
    fn check_call_effects(&mut self, callee_name: &str, callee_effects: &[Effect]) {
        if self.current_fn_is_main {
            return;
        }
        for eff in callee_effects {
            if !self.current_effects.iter().any(|e| e == eff) {
                self.push_diag(
                    Severity::Warning,
                    "EFF001".into(),
                    format!(
                        "纯函数或未声明效应 [{}] 的函数调用了带效应 [{}] 的函数 '{}'",
                        self.current_effects.join(", "),
                        eff,
                        callee_name
                    ),
                    self.current_fn_span.line,
                    self.current_fn_span.col,
                );
            }
        }
    }

    /// 检查 match 表达式
    fn check_match(&mut self, m: &MatchExpr, env: &mut TypeEnv) -> TypeOrUnknown {
        let scrut_ty = self.check_expr(&m.scrutinee, env);
        // 检查每个 arm 的模式是否匹配 scrut 类型
        let mut arm_tys = Vec::new();
        let mut has_wildcard = false;
        let mut matched_variants = std::collections::HashSet::new();
        for arm in &m.arms {
            let mut arm_env = env.child();
            let variant_name = self.check_pattern(&arm.pattern, &scrut_ty, &mut arm_env);
            if let Some(vn) = variant_name {
                matched_variants.insert(vn);
            }
            if matches!(arm.pattern, Pattern::Wildcard) {
                has_wildcard = true;
            }
            let body_ty = match &arm.body {
                MatchArmBody::Expr(e) => self.check_expr(e, &mut arm_env),
                MatchArmBody::Block(b) => self.check_block(b, &mut arm_env),
            };
            arm_tys.push(body_ty);
        }
        // 穷尽性检查
        if !has_wildcard {
            if let TypeOrUnknown::Known(Type::Named(name)) = &scrut_ty {
                if let Some(info) = self.enums.get(name).cloned() {
                    if !info.is_builtin {
                        // 用户枚举：必须覆盖所有变体
                        // Phase 4.1.2: line 填 match 的 end 行，供 fix 精确定位插入点
                        for (vn, _) in &info.variants {
                            if !matched_variants.contains(vn) {
                                self.push_diag(
                                    Severity::Warning,
                                    "MAT001".into(),
                                    format!(
                                        "match 非穷尽：未覆盖变体 '{}'（枚举 {}）",
                                        vn, name
                                    ),
                                    m.end_line,
                                    1,
                                );
                            }
                        }
                    }
                }
            } else if let TypeOrUnknown::Known(Type::Result(_, _)) = &scrut_ty {
                // Result 必须覆盖 Ok 和 Err
                if !matched_variants.contains("Ok") {
                    self.push_diag(Severity::Warning, "MAT001".into(),
                        "match 非穷尽：未覆盖 Ok".into(), m.end_line, 1);
                }
                if !matched_variants.contains("Err") {
                    self.push_diag(Severity::Warning, "MAT001".into(),
                        "match 非穷尽：未覆盖 Err".into(), m.end_line, 1);
                }
            } else if let TypeOrUnknown::Known(Type::Option(_)) = &scrut_ty {
                if !matched_variants.contains("Some") {
                    self.push_diag(Severity::Warning, "MAT001".into(),
                        "match 非穷尽：未覆盖 Some".into(), m.end_line, 1);
                }
                if !matched_variants.contains("None") {
                    self.push_diag(Severity::Warning, "MAT001".into(),
                        "match 非穷尽：未覆盖 None".into(), m.end_line, 1);
                }
            }
        }
        // 所有 arm 的类型应一致
        self.unify_types(&arm_tys)
    }

    /// 检查模式，返回变体名（若是变体模式）。
    /// 同时把模式中引入的绑定变量注入 `env`，供 arm body 检查使用。
    fn check_pattern(
        &mut self,
        pattern: &Pattern,
        scrut_ty: &TypeOrUnknown,
        env: &mut TypeEnv,
    ) -> Option<String> {
        match pattern {
            Pattern::Lit(lit) => {
                // 字面量模式的类型应与 scrutinee 一致（渐进式：不强制）
                let _ = lit;
                None
            }
            Pattern::Binder(name) => {
                // 绑定模式可能是无参数变体（如 None、Red）—— parser 把无参数变体
                // 解析为 Binder 而非 Variant。这里识别并当作变体处理，
                // 使 match 穷尽性检查能正确收集已覆盖的变体名。
                if self.is_variant_constructor(name) {
                    return Some(name.clone());
                }
                // 普通绑定变量：类型 = scrutinee 类型（若已知），否则 Unknown
                let ty = match scrut_ty {
                    TypeOrUnknown::Known(t) => TypeOrUnknown::Known(t.clone()),
                    TypeOrUnknown::Unknown => TypeOrUnknown::Unknown,
                };
                env.define(name.clone(), ty);
                None
            }
            Pattern::Wildcard => None,
            Pattern::Variant { name, sub } => {
                // 阶段1：只读借用 self.enums，收集变体信息到局部变量
                // （随后要可变借用 self 进行 push_diag 和递归 check_pattern，故先 clone 出来）
                let (variant_exists, expected_arity, field_tys, enum_name): (
                    bool,
                    usize,
                    Option<Vec<Type>>,
                    String,
                ) = if let TypeOrUnknown::Known(t) = scrut_ty {
                    let en = match t {
                        Type::Result(_, _) => "Result".to_string(),
                        Type::Option(_) => "Option".to_string(),
                        Type::Named(n) => n.clone(),
                        _ => String::new(),
                    };
                    if en.is_empty() {
                        (true, sub.len(), None, en)
                    } else if let Some(info) = self.enums.get(&en) {
                        match info.variants.iter().find(|(vn, _)| vn == name) {
                            None => (false, 0, None, en),
                            Some((_, ftypes)) => (true, ftypes.len(), Some(ftypes.clone()), en),
                        }
                    } else {
                        // 枚举未注册（如 scrut_ty 是泛型占位符），不报错
                        (true, sub.len(), None, en)
                    }
                } else {
                    (true, sub.len(), None, String::new())
                };

                // 阶段2：可变借用 self，根据收集的信息报错
                if !variant_exists {
                    self.push_diag(
                        Severity::Error,
                        "NAM004".into(),
                        format!("枚举 {} 无变体 '{}'", enum_name, name),
                        0,
                        0,
                    );
                } else if expected_arity != sub.len() {
                    self.push_diag(
                        Severity::Warning,
                        "TYPE003".into(),
                        format!(
                            "变体 '{}' 期望 {} 个子模式，得到 {} 个",
                            name,
                            expected_arity,
                            sub.len()
                        ),
                        0,
                        0,
                    );
                }

                // 递归检查子模式，把子绑定变量注入环境
                // 字段类型若为泛型占位符（T/E）则用 Unknown，避免误报
                for (i, sub_pat) in sub.iter().enumerate() {
                    let sub_ty = field_tys
                        .as_ref()
                        .and_then(|fs| fs.get(i))
                        .map(|t| {
                            if is_generic_placeholder(t) {
                                TypeOrUnknown::Unknown
                            } else {
                                TypeOrUnknown::Known(t.clone())
                            }
                        })
                        .unwrap_or(TypeOrUnknown::Unknown);
                    self.check_pattern(sub_pat, &sub_ty, env);
                }
                Some(name.clone())
            }
        }
    }

    // ===== 类型兼容性 =====

    /// 类型兼容性检查（结构等价，记录字段顺序不敏感）
    ///
    /// 特殊处理：
    ///   - `Named("_Any")` 是内置通配符类型（prelude/stdlib 函数签名用），兼容任何类型
    ///   - `Named("T")` / `Named("E")` 是内置变体的泛型占位符，兼容任何类型（简化泛型推断）
    fn types_compatible(&self, a: &Type, b: &Type) -> bool {
        // 通配符类型（prelude/stdlib 签名用）
        if matches!(a, Type::Named(n) if n == "_Any") || matches!(b, Type::Named(n) if n == "_Any") {
            return true;
        }
        // 内置变体泛型占位符（Ok/Err/Some 返回 Result<T,E>/Option<T>，T/E 是占位符）
        if matches!(a, Type::Named(n) if n == "T" || n == "E")
            || matches!(b, Type::Named(n) if n == "T" || n == "E")
        {
            return true;
        }
        match (a, b) {
            (Type::Int, Type::Int) => true,
            (Type::Float, Type::Float) => true,
            (Type::Bool, Type::Bool) => true,
            (Type::String, Type::String) => true,
            (Type::Unit, Type::Unit) => true,
            (Type::Named(a), Type::Named(b)) => a == b,
            (Type::Option(a), Type::Option(b)) => self.types_compatible(a, b),
            (Type::Result(a1, a2), Type::Result(b1, b2)) => {
                self.types_compatible(a1, b1) && self.types_compatible(a2, b2)
            }
            (Type::Tuple(a), Type::Tuple(b)) => {
                a.len() == b.len()
                    && a.iter().zip(b.iter()).all(|(x, y)| self.types_compatible(x, y))
            }
            (Type::Record(a), Type::Record(b)) => {
                if a.len() != b.len() {
                    return false;
                }
                for (name, ty) in a {
                    match b.iter().find(|(n, _)| n == name) {
                        Some((_, bty)) => {
                            if !self.types_compatible(ty, bty) {
                                return false;
                            }
                        }
                        None => return false,
                    }
                }
                true
            }
            (Type::Generic(a_name, _), Type::Generic(b_name, _)) => a_name == b_name,
            _ => false,
        }
    }

    /// 检查所在函数返回类型是否与 `?` 的 Result 兼容
    ///
    /// 放宽：只要所在函数返回 Result 即可（不严格检查 Err 类型，因泛型推断不完整）
    fn result_compatible(&self, ret: &Type, _ok_t: &Type, _err_t: &Type) -> bool {
        matches!(ret, Type::Result(_, _))
    }

    /// 检查所在函数返回类型是否与 `?` 的 Option 兼容
    fn option_compatible(&self, ret: &Type, _t: &Type) -> bool {
        matches!(ret, Type::Option(_))
    }

    /// 判断类型是否为内置通配符 `_Any`（prelude/stdlib 函数签名用）
    /// `_Any` 兼容任何类型，参与运算时不报类型错误。
    fn is_any_type(&self, t: &Type) -> bool {
        matches!(t, Type::Named(n) if n == "_Any")
    }

    /// 二元运算结果类型
    fn binary_result_type(
        &mut self,
        op: &BinOp,
        lt: &TypeOrUnknown,
        rt: &TypeOrUnknown,
    ) -> TypeOrUnknown {
        match op {
            BinOp::Add => {
                // + 支持 Int+Int, Float+Float, String+String
                match (lt, rt) {
                    (TypeOrUnknown::Known(Type::Int), TypeOrUnknown::Known(Type::Int)) => {
                        TypeOrUnknown::known(Type::Int)
                    }
                    (TypeOrUnknown::Known(Type::Float), TypeOrUnknown::Known(Type::Float)) => {
                        TypeOrUnknown::known(Type::Float)
                    }
                    (TypeOrUnknown::Known(Type::String), TypeOrUnknown::Known(Type::String)) => {
                        TypeOrUnknown::known(Type::String)
                    }
                    (TypeOrUnknown::Known(a), TypeOrUnknown::Known(b)) => {
                        // _Any 通配符（stdlib 返回 _Any 的函数参与运算）：不报错
                        if self.is_any_type(a) || self.is_any_type(b) {
                            return TypeOrUnknown::unknown();
                        }
                        self.push_diag(
                            Severity::Warning,
                            "TYPE001".into(),
                            format!("'+' 不支持 {:?} 和 {:?}", a, b),
                            0,
                            0,
                        );
                        TypeOrUnknown::unknown()
                    }
                    _ => TypeOrUnknown::unknown(),
                }
            }
            BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                match (lt, rt) {
                    (TypeOrUnknown::Known(Type::Int), TypeOrUnknown::Known(Type::Int)) => {
                        TypeOrUnknown::known(Type::Int)
                    }
                    (TypeOrUnknown::Known(Type::Float), TypeOrUnknown::Known(Type::Float)) => {
                        TypeOrUnknown::known(Type::Float)
                    }
                    (TypeOrUnknown::Known(a), TypeOrUnknown::Known(b)) => {
                        if self.is_any_type(a) || self.is_any_type(b) {
                            return TypeOrUnknown::unknown();
                        }
                        self.push_diag(
                            Severity::Warning,
                            "TYPE001".into(),
                            format!("'{:?}' 不支持 {:?} 和 {:?}", op, a, b),
                            0,
                            0,
                        );
                        TypeOrUnknown::unknown()
                    }
                    _ => TypeOrUnknown::unknown(),
                }
            }
            BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => {
                // 比较运算返回 Bool；检查操作数类型一致（渐进式：不强制）
                if let (TypeOrUnknown::Known(a), TypeOrUnknown::Known(b)) = (lt, rt) {
                    if !self.types_compatible(a, b) {
                        self.push_diag(
                            Severity::Warning,
                            "TYPE001".into(),
                            format!("比较运算两边类型不一致: {:?} vs {:?}", a, b),
                            0,
                            0,
                        );
                    }
                }
                TypeOrUnknown::known(Type::Bool)
            }
        }
    }

    /// 统一多个类型：若一致返回该类型，否则 Unknown
    fn unify_types(&self, tys: &[TypeOrUnknown]) -> TypeOrUnknown {
        if tys.is_empty() {
            return TypeOrUnknown::unknown();
        }
        let first = &tys[0];
        if let TypeOrUnknown::Known(first_t) = first {
            for t in &tys[1..] {
                if let TypeOrUnknown::Known(t_t) = t {
                    if !self.types_compatible(first_t, t_t) {
                        return TypeOrUnknown::unknown();
                    }
                } else {
                    return TypeOrUnknown::unknown();
                }
            }
            first.clone()
        } else {
            TypeOrUnknown::unknown()
        }
    }

    // ===== 辅助 =====

    fn is_variant_constructor(&self, name: &str) -> bool {
        self.enums
            .values()
            .any(|e| e.variants.iter().any(|(vn, _)| vn == name))
    }

    fn variant_return_type(&self, name: &str) -> TypeOrUnknown {
        for (enum_name, info) in &self.enums {
            for (vn, fields) in &info.variants {
                if vn == name {
                    if info.is_builtin {
                        // Ok/Some 返回 Result/Option，Err 返回 Result，None 返回 Option
                        match enum_name.as_str() {
                            "Result" => {
                                if name == "Ok" {
                                    return TypeOrUnknown::known(Type::Result(
                                        Box::new(Type::Named("T".to_string())),
                                        Box::new(Type::Named("E".to_string())),
                                    ));
                                } else {
                                    return TypeOrUnknown::known(Type::Result(
                                        Box::new(Type::Named("T".to_string())),
                                        Box::new(Type::Named("E".to_string())),
                                    ));
                                }
                            }
                            "Option" => {
                                return TypeOrUnknown::known(Type::Option(Box::new(Type::Named(
                                    "T".to_string(),
                                ))))
                            }
                            _ => {}
                        }
                    } else {
                        return TypeOrUnknown::known(Type::Named(enum_name.clone()));
                    }
                }
                let _ = fields;
            }
        }
        TypeOrUnknown::unknown()
    }

    fn variant_info(&self, name: &str) -> (usize, TypeOrUnknown) {
        for (enum_name, info) in &self.enums {
            for (vn, fields) in &info.variants {
                if vn == name {
                    let arity = fields.len();
                    let ret_ty = if info.is_builtin {
                        match enum_name.as_str() {
                            "Result" => TypeOrUnknown::known(Type::Result(
                                Box::new(Type::Named("T".to_string())),
                                Box::new(Type::Named("E".to_string())),
                            )),
                            "Option" => TypeOrUnknown::known(Type::Option(Box::new(Type::Named(
                                "T".to_string(),
                            )))),
                            _ => TypeOrUnknown::unknown(),
                        }
                    } else {
                        TypeOrUnknown::known(Type::Named(enum_name.clone()))
                    };
                    return (arity, ret_ty);
                }
            }
        }
        (0, TypeOrUnknown::unknown())
    }

    fn callee_return_type(&mut self, callee: &Expr, env: &mut TypeEnv) -> TypeOrUnknown {
        match callee {
            Expr::Ident(name) => {
                if let Some(sig) = self.functions.get(name) {
                    sig.ret.clone().map(TypeOrUnknown::Known).unwrap_or(TypeOrUnknown::unknown())
                } else {
                    self.check_expr(callee, env)
                }
            }
            _ => self.check_expr(callee, env),
        }
    }

    /// Phase 4.1.1: 为未定义名找拼写建议
    ///
    /// 候选集 = 作用域变量名 + 顶层函数名（含标准库）+ 枚举变体名。
    /// 返回编辑距离最小且 ≤ 2 的候选；无则 None。
    fn suggest_spelling(&self, name: &str, env: &TypeEnv) -> Option<String> {
        let mut candidates: Vec<String> = Vec::new();
        env.collect_names(&mut candidates);
        for k in self.functions.keys() {
            candidates.push(k.clone());
        }
        for info in self.enums.values() {
            for (vn, _) in &info.variants {
                candidates.push(vn.clone());
            }
        }
        let mut best: Option<(usize, String)> = None;
        for c in candidates {
            if c == name {
                continue;
            }
            let d = levenshtein(name, &c);
            if d <= 2 {
                // 距离更小必更新；距离相同则偏好更长的候选名
                // （用户更可能漏字符，如 printl→println 而非 print）
                let should_update = match &best {
                    None => true,
                    Some((bd, bs)) => d < *bd || (d == *bd && c.len() > bs.len()),
                };
                if should_update {
                    best = Some((d, c));
                }
            }
        }
        best.map(|(_, c)| c)
    }

    /// Phase 4.1.1: 为最后一条 NAM003 诊断附加拼写建议 hint
    ///
    /// 在 push_diag(NAM003) 之后调用：若有建议，覆盖默认通用 hint 为
    /// "是否想用 'X'？"，让 LLM/fix 拿到具体修复方向。
    fn patch_nam003_hint(&mut self, name: &str, env: &TypeEnv) {
        if let Some(suggestion) = self.suggest_spelling(name, env) {
            if let Some(last) = self.diags.last_mut() {
                last.hint = Some(format!("是否想用 '{}'？", suggestion));
            }
        }
    }

    fn push_diag(&mut self, severity: Severity, code: String, message: String, line: usize, col: usize) {
        let hint = type_hint(&code);
        let source_line = if line > 0 {
            self.source_lines.get(line.saturating_sub(1)).cloned()
        } else {
            None
        };
        self.diags.push(Diagnostic {
            severity,
            stage: Stage::Type,
            code,
            message,
            file: self.file.clone(),
            line,
            col,
            source_line,
            is_hole: false,
            hint,
        });
    }
}

/// 类型环境：变量名 → 类型
#[derive(Clone)]
struct TypeEnv {
    vars: HashMap<String, TypeOrUnknown>,
    parent: Option<Box<TypeEnv>>,
}

impl TypeEnv {
    fn new() -> Self {
        TypeEnv {
            vars: HashMap::new(),
            parent: None,
        }
    }

    fn child(&self) -> Self {
        TypeEnv {
            vars: HashMap::new(),
            parent: Some(Box::new(self.clone())),
        }
    }

    fn define(&mut self, name: String, ty: TypeOrUnknown) {
        self.vars.insert(name, ty);
    }

    fn get(&self, name: &str) -> Option<TypeOrUnknown> {
        if let Some(t) = self.vars.get(name) {
            Some(t.clone())
        } else if let Some(p) = &self.parent {
            p.get(name)
        } else {
            None
        }
    }

    /// Phase 4.1.1: 收集作用域内所有变量名（含父作用域），供 NAM003 拼写建议
    fn collect_names(&self, out: &mut Vec<String>) {
        for k in self.vars.keys() {
            out.push(k.clone());
        }
        if let Some(p) = &self.parent {
            p.collect_names(out);
        }
    }
}

/// 修复提示
fn type_hint(code: &str) -> Option<String> {
    match code {
        "NAM002" => Some("重命名重复的函数/枚举".into()),
        "NAM003" => Some("确认变量/函数已声明，拼写无误".into()),
        "NAM004" => Some("检查字段/变体名是否存在".into()),
        "TYPE001" => Some("检查运算符两侧类型是否一致".into()),
        "TYPE002" => Some("条件表达式应为 Bool".into()),
        "TYPE003" => Some("检查函数参数数量和类型".into()),
        "TYPE010" => Some("调整返回值或修改函数签名".into()),
        "TYPE020" => Some("`?` 要求 Result/Option，且所在函数返回兼容类型".into()),
        "MAT001" => Some("添加缺失的 match 分支，或用 `_` 通配符".into()),
        _ => None,
    }
}

/// Phase 4.1.1: Levenshtein 编辑距离
///
/// 用于 NAM003 拼写建议：未定义名与候选名的距离 ≤ 2 视为可能的拼写错误。
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }
    // 长度差 > 2 时编辑距离必然 > 2，直接返回（快速剪枝）
    if m.abs_diff(n) > 2 {
        return m.max(n);
    }
    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr: Vec<usize> = vec![0; n + 1];
    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[n]
}

// 辅助函数：把 args 转回表达式切片（用于管道场景的递归调用）
fn all_args_args_to_exprs(args: &[Expr]) -> Vec<Expr> {
    args.to_vec()
}

/// 判断类型是否为内置变体的泛型占位符（如 Result 的 T/E、Option 的 T）
/// 这些占位符在类型推断中应视为 Unknown，避免误报类型不匹配。
fn is_generic_placeholder(t: &Type) -> bool {
    matches!(t, Type::Generic(name, _) if name == "T" || name == "E")
}

// ===== 公开入口 =====

/// 对程序执行类型检查，将诊断合并到 diags
pub fn check_program(program: &Program, src: &str, file: &str, diags: &mut Diagnostics) {
    let source_lines: Vec<String> = src.lines().map(|s| s.to_string()).collect();
    let tc = TypeChecker::new(file, source_lines);
    tc.check(program, diags);
}

// ===== 单元测试 =====

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Diagnostics;

    fn check_src(src: &str) -> Diagnostics {
        let result = crate::parser::Parser::parse_recover(src);
        let mut diags = Diagnostics::new("test.lom");
        // 合并解析错误
        let source_lines: Vec<String> = src.lines().map(|s| s.to_string()).collect();
        for e in &result.errors {
            diags.diagnostics
                .push(Diagnostic::from_parse(e, "test.lom", &source_lines.iter().map(|s| s.as_str()).collect::<Vec<_>>()));
            diags.ok = false;
        }
        if result.is_ok() {
            check_program(&result.program, src, "test.lom", &mut diags);
        }
        diags
    }

    fn count_type_diags(diags: &Diagnostics) -> usize {
        diags.diagnostics.iter().filter(|d| d.stage == Stage::Type).count()
    }

    #[test]
    fn clean_program_has_no_type_errors() {
        let src = "fn double(x: Int) -> Int\n    x * 2\nend\nfn main() -> Unit\n    println(double(5))\nend\n";
        let diags = check_src(src);
        assert_eq!(count_type_diags(&diags), 0, "should have no type errors");
    }

    #[test]
    fn undefined_variable_reported() {
        let src = "fn main() -> Unit\n    println(undefined_var)\nend\n";
        let diags = check_src(src);
        let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.stage == Stage::Type).collect();
        assert!(!type_diags.is_empty());
        assert!(type_diags.iter().any(|d| d.code == "NAM003"));
    }

    /// Phase 4.1.1: NAM003 拼写建议 — 函数名拼错时应建议正确名
    #[test]
    fn nam003_suggests_similar_function_name() {
        // printl 拼错，应为 println（prelude 自动可用）
        let src = "fn main() -> Unit\n    printl(\"hi\")\nend\n";
        let diags = check_src(src);
        let nam003: Vec<_> = diags
            .diagnostics
            .iter()
            .filter(|d| d.code == "NAM003")
            .collect();
        assert!(!nam003.is_empty(), "应报 NAM003");
        let hint = nam003[0].hint.as_ref().expect("应有 hint");
        assert!(
            hint.contains("println"),
            "hint 应建议 println，实际: {}",
            hint
        );
    }

    /// Phase 4.1.1: NAM003 拼写建议 — 变量名拼错时应建议正确名
    #[test]
    fn nam003_suggests_similar_variable_name() {
        // cont 拼错，应为 count
        let src = "fn main() -> Unit\n    let count = 5\n    println(cont)\nend\n";
        let diags = check_src(src);
        let nam003: Vec<_> = diags
            .diagnostics
            .iter()
            .filter(|d| d.code == "NAM003")
            .collect();
        assert!(!nam003.is_empty(), "应报 NAM003");
        let hint = nam003[0].hint.as_ref().expect("应有 hint");
        assert!(
            hint.contains("count"),
            "hint 应建议 count，实际: {}",
            hint
        );
    }

    /// Phase 4.1.1: NAM003 拼写建议 — 无相似名时保持通用 hint（不误报）
    #[test]
    fn nam003_no_suggestion_when_no_similar() {
        // xyzqwerty 与任何已知名都不相近
        let src = "fn main() -> Unit\n    xyzqwerty\nend\n";
        let diags = check_src(src);
        let nam003: Vec<_> = diags
            .diagnostics
            .iter()
            .filter(|d| d.code == "NAM003")
            .collect();
        assert!(!nam003.is_empty(), "应报 NAM003");
        // 无建议时 hint 应是通用文本，不含"是否想用"
        let hint = nam003[0].hint.as_ref().expect("应有 hint");
        assert!(
            !hint.contains("是否想用"),
            "无相似名不应给建议，实际: {}",
            hint
        );
    }

    #[test]
    fn type_mismatch_in_let_annotation() {
        let src = "fn main() -> Unit\n    let x: Int = \"hello\"\nend\n";
        let diags = check_src(src);
        let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.stage == Stage::Type).collect();
        assert!(type_diags.iter().any(|d| d.code == "TYPE001"));
    }

    #[test]
    fn function_param_count_mismatch() {
        let src = "fn add(a: Int, b: Int) -> Int\n    a + b\nend\nfn main() -> Unit\n    add(1)\nend\n";
        let diags = check_src(src);
        let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.stage == Stage::Type).collect();
        assert!(type_diags.iter().any(|d| d.code == "TYPE003" && d.message.contains("参数")));
    }

    #[test]
    fn return_type_mismatch() {
        let src = "fn f() -> Int\n    \"hello\"\nend\n";
        let diags = check_src(src);
        let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.stage == Stage::Type).collect();
        // Phase 3.2: TYPE010 诊断应定位到函数签名行（fn 关键字位置 1:1），而非 (0:0)
        let type010 = type_diags.iter().find(|d| d.code == "TYPE010");
        assert!(type010.is_some(), "应报告 TYPE010");
        let d = type010.unwrap();
        assert_eq!(d.line, 1, "TYPE010 应定位到函数签名行 (line 1)");
        assert_eq!(d.col, 1, "TYPE010 应定位到 fn 关键字 (col 1)");
    }

    #[test]
    fn if_condition_must_be_bool() {
        let src = "fn main() -> Unit\n    if 5\n        println(\"hi\")\n    end\nend\n";
        let diags = check_src(src);
        let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.stage == Stage::Type).collect();
        assert!(type_diags.iter().any(|d| d.code == "TYPE002"));
    }

    #[test]
    fn result_exhaustive_match_ok() {
        let src = "fn f() -> Unit\n    let x = Ok(5)\n    match x\n        Ok(n) => println(n)\n        Err(e) => println(e)\n    end\nend\n";
        let diags = check_src(src);
        let mat_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "MAT001").collect();
        assert_eq!(mat_diags.len(), 0, "exhaustive Result match should not warn");
    }

    #[test]
    fn result_non_exhaustive_match_warns() {
        let src = "fn f() -> Unit\n    let x = Ok(5)\n    match x\n        Ok(n) => println(n)\n    end\nend\n";
        let diags = check_src(src);
        let mat_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "MAT001").collect();
        assert!(mat_diags.iter().any(|d| d.message.contains("Err")), "should warn about missing Err");
    }

    #[test]
    fn option_exhaustive_with_none_and_some() {
        let src = "fn f() -> Unit\n    let x = Some(5)\n    match x\n        Some(n) => println(n)\n        None => println(\"none\")\n    end\nend\n";
        let diags = check_src(src);
        let mat_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "MAT001").collect();
        assert_eq!(mat_diags.len(), 0);
    }

    #[test]
    fn user_enum_non_exhaustive_warns() {
        let src = "enum Color = Red | Green | Blue\nfn f() -> Unit\n    let c = Red\n    match c\n        Red => println(\"r\")\n    end\nend\n";
        let diags = check_src(src);
        let mat_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "MAT001").collect();
        assert!(mat_diags.iter().any(|d| d.message.contains("Green")));
        assert!(mat_diags.iter().any(|d| d.message.contains("Blue")));
    }

    #[test]
    fn user_enum_exhaustive_no_warn() {
        let src = "enum Color = Red | Green | Blue\nfn f() -> Unit\n    let c = Red\n    match c\n        Red => println(\"r\")\n        Green => println(\"g\")\n        Blue => println(\"b\")\n    end\nend\n";
        let diags = check_src(src);
        let mat_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "MAT001").collect();
        assert_eq!(mat_diags.len(), 0);
    }

    #[test]
    fn wildcard_makes_match_exhaustive() {
        let src = "enum Color = Red | Green | Blue\nfn f() -> Unit\n    let c = Red\n    match c\n        Red => println(\"r\")\n        _ => println(\"other\")\n    end\nend\n";
        let diags = check_src(src);
        let mat_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "MAT001").collect();
        assert_eq!(mat_diags.len(), 0);
    }

    /// Phase 4.1.2: MAT001 Result 非穷尽 — fix --apply 应自动补全缺失的 Err 分支
    #[test]
    fn mat001_fix_apply_inserts_missing_err_branch() {
        // 缺少 Err 分支的 Result match（LLM 常见遗漏）
        let src = "fn f() -> Unit\n    let x = Ok(5)\n    match x\n        Ok(n) => println(n)\n    end\nend\n";
        let diags = check_src(src);
        // 确实报 MAT001 未覆盖 Err
        assert!(
            diags
                .diagnostics
                .iter()
                .any(|d| d.code == "MAT001" && d.message.contains("Err")),
            "应报 MAT001 未覆盖 Err"
        );
        // 生成修复计划并应用（端到端：typechecker → fix → apply）
        let plan = crate::fix::generate_plan(&diags, src);
        let result = crate::apply::apply_plan(&plan, src);
        assert!(
            result.applied >= 1,
            "应自动应用至少 1 个修复，实际 {}，patched: {:?}",
            result.applied,
            result.patched_source
        );
        // 修复后源码应含 Err(_) => () 分支
        assert!(
            result.patched_source.contains("Err(_) => ()"),
            "patched 应含 Err(_) => ()，实际: {:?}",
            result.patched_source
        );
    }

    /// Phase 4.1.2: MAT001 Option 非穷尽 — fix --apply 应自动补全缺失的 None 分支
    #[test]
    fn mat001_fix_apply_inserts_missing_none_branch() {
        let src = "fn f() -> Unit\n    let x = Some(5)\n    match x\n        Some(n) => println(n)\n    end\nend\n";
        let diags = check_src(src);
        assert!(
            diags
                .diagnostics
                .iter()
                .any(|d| d.code == "MAT001" && d.message.contains("None")),
            "应报 MAT001 未覆盖 None"
        );
        let plan = crate::fix::generate_plan(&diags, src);
        let result = crate::apply::apply_plan(&plan, src);
        assert!(
            result.applied >= 1,
            "应自动应用至少 1 个修复，实际 {}，patched: {:?}",
            result.applied,
            result.patched_source
        );
        assert!(
            result.patched_source.contains("None => ()"),
            "patched 应含 None => ()，实际: {:?}",
            result.patched_source
        );
    }

    /// Phase 4.1.2: MAT001 用户枚举非穷尽 — 不应自动 apply（参数未知，安全边界）
    ///
    /// 用户枚举变体可能带参数（如 Point(x, y)），`Name => ()` 会引入语法错误，
    /// 故保持 Hint + Medium，不自动应用。锁定此安全边界防止未来回归。
    #[test]
    fn mat001_user_enum_not_auto_applied() {
        // 用无副作用 body（Red => 0，fn 返回 Int）避免触发 EFF001 干扰 apply 计数
        let src = "enum Color = Red | Green | Blue\nfn f() -> Int\n    let c = Red\n    match c\n        Red => 0\n    end\nend\n";
        let diags = check_src(src);
        assert!(
            diags
                .diagnostics
                .iter()
                .any(|d| d.code == "MAT001" && d.message.contains("Green")),
            "应报 MAT001 未覆盖 Green"
        );
        let plan = crate::fix::generate_plan(&diags, src);
        let result = crate::apply::apply_plan(&plan, src);
        // 用户枚举变体是 Hint + Medium，不应被 --apply 自动应用
        assert_eq!(
            result.applied, 0,
            "用户枚举变体不应自动 apply，实际 {}，patched: {:?}",
            result.applied, result.patched_source
        );
    }

    #[test]
    fn try_on_non_result_warns() {
        let src = "fn f() -> Int\n    let x = 5\n    x?\nend\n";
        let diags = check_src(src);
        let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.stage == Stage::Type).collect();
        assert!(type_diags.iter().any(|d| d.code == "TYPE020"));
    }

    #[test]
    fn try_on_result_in_result_function_ok() {
        let src = "fn f() -> Result<Int, String>\n    let x = Ok(5)\n    Ok(x?)\nend\n";
        let diags = check_src(src);
        let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "TYPE020").collect();
        // ? on Result<Int, String> in fn returning Result<Int, String> — should not warn
        assert_eq!(type_diags.len(), 0);
    }

    #[test]
    fn string_concat_ok() {
        let src = "fn f() -> String\n    \"hello\" + \" world\"\nend\n";
        let diags = check_src(src);
        let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.stage == Stage::Type && d.code == "TYPE001").collect();
        assert_eq!(type_diags.len(), 0);
    }

    #[test]
    fn int_plus_string_warns() {
        let src = "fn f() -> Unit\n    let x = 1 + \"hello\"\nend\n";
        let diags = check_src(src);
        let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "TYPE001").collect();
        assert!(!type_diags.is_empty());
    }

    #[test]
    fn record_field_access_ok() {
        let src = "fn f() -> Int\n    let p = {x: 3, y: 4}\n    p.x\nend\n";
        let diags = check_src(src);
        let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.stage == Stage::Type && d.code == "NAM004").collect();
        assert_eq!(type_diags.len(), 0);
    }

    #[test]
    fn record_missing_field_warns() {
        let src = "fn f() -> Int\n    let p = {x: 3, y: 4}\n    p.z\nend\n";
        let diags = check_src(src);
        let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "NAM004").collect();
        assert!(!type_diags.is_empty());
    }

    #[test]
    fn duplicate_function_definition() {
        let src = "fn f() -> Unit\n    println(1)\nend\nfn f() -> Unit\n    println(2)\nend\n";
        let diags = check_src(src);
        let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "NAM002").collect();
        assert!(!type_diags.is_empty());
        // Phase 3.2: NAM002 应定位到第二次重复定义的 fn 关键字 (line 4, col 1)，而非 (0:0)
        let d = &type_diags[0];
        assert_eq!(d.line, 4, "NAM002 应定位到第二次定义的函数签名行 (line 4)");
        assert_eq!(d.col, 1, "NAM002 应定位到 fn 关键字 (col 1)");
    }

    #[test]
    fn closure_return_type_checked() {
        let src = "fn main() -> Unit\n    let f = fn(x: Int) -> String\n        x\n    end\nend\n";
        let diags = check_src(src);
        let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "TYPE010").collect();
        assert!(!type_diags.is_empty());
    }

    // ===== Phase 2.5 效应系统测试 =====

    #[test]
    fn pure_function_calling_io_function_reports_eff001() {
        // 纯函数 helper 调用 println（带 IO 效应）→ 应报 EFF001
        let src = "fn helper(x: Int) -> Int\n    println(x)\n    x\nend\nfn main() -> Unit\n    helper(5)\nend\n";
        let diags = check_src(src);
        let eff_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "EFF001").collect();
        assert_eq!(eff_diags.len(), 1, "纯函数调用 IO 函数应报 EFF001");
        // Phase 3.2: EFF001 应定位到 helper 的函数签名行 (line 1, col 1)，而非 (0:0)
        let d = &eff_diags[0];
        assert_eq!(d.line, 1, "EFF001 应定位到 helper 函数签名行 (line 1)");
        assert_eq!(d.col, 1, "EFF001 应定位到 fn 关键字 (col 1)");
    }

    #[test]
    fn io_function_calling_io_function_no_error() {
        // helper 声明 ! [IO]，调用 println 不报 EFF001
        let src = "fn helper(x: Int) -> Int ! [IO]\n    println(x)\n    x\nend\nfn main() -> Unit\n    helper(5)\nend\n";
        let diags = check_src(src);
        let eff_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "EFF001").collect();
        assert!(eff_diags.is_empty(), "声明了 IO 效应的函数调用 println 不应报 EFF001");
    }

    #[test]
    fn main_function_calling_io_no_error() {
        // main 函数隐式拥有所有效应，调用 println 不报 EFF001
        let src = "fn main() -> Unit\n    println(\"hello\")\nend\n";
        let diags = check_src(src);
        let eff_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "EFF001").collect();
        assert!(eff_diags.is_empty(), "main 函数调用 println 不应报 EFF001");
    }

    #[test]
    fn main_calling_pure_then_io_no_error() {
        // main 调用纯函数 double，再调用 println — 都不报 EFF001
        let src = "fn double(x: Int) -> Int\n    x * 2\nend\nfn main() -> Unit\n    println(double(5))\nend\n";
        let diags = check_src(src);
        let eff_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "EFF001").collect();
        assert!(eff_diags.is_empty(), "main 中调用 println 不应报 EFF001");
    }

    #[test]
    fn partial_effect_coverage_reports_eff001() {
        // helper 声明 ! [IO]，调用 declare_clock（带 Clock 效应）→ 应报 EFF001（Clock 未声明）
        let src = "fn declare_clock() -> Int ! [Clock]\n    0\nend\nfn helper(x: Int) -> Int ! [IO]\n    declare_clock()\n    x\nend\nfn main() -> Unit\n    helper(5)\nend\n";
        let diags = check_src(src);
        let eff_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "EFF001").collect();
        assert_eq!(eff_diags.len(), 1, "声明 [IO] 但调用带 [Clock] 的函数应报 EFF001");
    }

    #[test]
    fn multi_effect_function_no_error() {
        // helper 声明 ! [IO, Clock]，调用 println 和 declare_clock 都不报
        let src = "fn declare_clock() -> Int ! [Clock]\n    0\nend\nfn helper(x: Int) -> Int ! [IO, Clock]\n    println(x)\n    declare_clock()\n    x\nend\nfn main() -> Unit\n    helper(5)\nend\n";
        let diags = check_src(src);
        let eff_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "EFF001").collect();
        assert!(eff_diags.is_empty(), "声明 [IO, Clock] 后调用对应效应函数不应报 EFF001");
    }

    #[test]
    fn empty_effect_list_treated_as_pure() {
        // ! [] 等价于纯函数，调用 println 仍报 EFF001
        let src = "fn helper(x: Int) -> Int ! []\n    println(x)\n    x\nend\nfn main() -> Unit\n    helper(5)\nend\n";
        let diags = check_src(src);
        let eff_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "EFF001").collect();
        assert_eq!(eff_diags.len(), 1, "! [] 等价于纯函数，应报 EFF001");
    }

    #[test]
    fn pure_function_calling_pure_no_error() {
        // 纯函数调用纯函数（如 math 模块的 len/upper 等）不报 EFF001
        let src = "from string import { len }\nfn helper(s: String) -> Int\n    len(s)\nend\nfn main() -> Unit\n    println(helper(\"hi\"))\nend\n";
        let diags = check_src(src);
        let eff_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "EFF001").collect();
        assert!(eff_diags.is_empty(), "纯函数调用纯函数不应报 EFF001");
    }

    #[test]
    fn effect_annotation_parsed_correctly() {
        // 验证效应注解被正确解析（不报语法错误，且 typechecker 能识别）
        let src = "fn fetch(url: String) -> String ! [IO, Network]\n    \"data\"\nend\nfn main() -> Unit\n    fetch(\"x\")\nend\n";
        let diags = check_src(src);
        // main 调用 fetch（带 IO/Network 效应）— main 隐式所有效应，不报
        let eff_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "EFF001").collect();
        assert!(eff_diags.is_empty(), "main 调用带效应函数不应报 EFF001");
        // 也不应有语法错误
        let parse_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.stage == crate::diagnostics::Stage::Parse).collect();
        assert!(parse_diags.is_empty(), "不应有语法错误");
    }

    #[test]
    fn nested_pure_function_calling_io_through_chain() {
        // a (纯) 调用 b (纯) 调用 c (IO) — 在 c 调用 println 处报 EFF001，但 b 调用 c 处不报
        // （因为 b 也是纯函数，b 调用 c 时 c 的 IO 效应未声明 → 报 EFF001）
        // 实际：c 内部调用 println 报 EFF001；b 调用 c 时 c 没声明 IO 效应 → 不报（c 自身效应列表空）
        // 这个测试验证：效应检查只看声明的效应，不传递
        let src = "fn c(x: Int) -> Int\n    println(x)\n    x\nend\nfn b(x: Int) -> Int\n    c(x)\nend\nfn main() -> Unit\n    b(5)\nend\n";
        let diags = check_src(src);
        let eff_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "EFF001").collect();
        // c 内调用 println 报 1 次 EFF001；b 调用 c 不报（c 未声明效应）
        assert_eq!(eff_diags.len(), 1, "只有 c 内调用 println 报 EFF001");
    }
}
