// Lom WASM 后端 —— Phase 7.2：AST → WASM 编译器
//
// 设计（RFC-0002）：
// - 编译的是**动态语义**（与树遍历解释器逐字对齐），不是静态类型特化
// - 值表示：tagged i64，低 3 位 tag：
//     0 = Int（v<<3）        1 = Bool（true=9, false=1）   2 = Unit（常量 2）
//     3 = F64 盒（堆指针）    4 = Str（堆指针，布局 [len:u32][utf8 字节]）
//   （List/Map/Record/Tuple/Enum/Closure 是 7.3-7.6 的事，本阶段编译期报错）
// - Float/Str 字面量编译期进数据段（去重）；运行时 Float 运算结果走 bump allocator
//   动态装盒（全局 $hp，arena 不释放——目标负载是短生命周期 CLI，见 RFC-0002）
// - 运行时 tag 分派逻辑集中在一组手写 helper 函数（rt_add/rt_eq/rt_print 等），
//   codegen 只负责结构翻译 + call helper，避免每个二元运算处内联展开
// - 与解释器的已知差异（7.2 如实记录）：
//   除零/取模零在 WASM 是 trap（harness 退出码 1），消息文本与解释器不同；
//   不支持的构造是**编译期错误**而非运行时错误（更严格，宁缺毋滥）；
//   字符串大小比较（</> 等）未实现（==/!= 已实现）；Int 溢出回绕（与 release 解释器一致）；
//   闭包捕获是**创建时值拷贝**（解释器是 Rc 共享作用域——创建后修改被捕获变量，
//   解释器的闭包会看到新值，WASM 后端不会；该模式在实践中罕见，如需共享语义请提 issue）。

use crate::ast::*;
use crate::wasm::{
    leb_s, leb_u, op, DataSegment, ExportKind, FuncType, Function, Global, Import, Module, ValType,
};

// ===== tag 与值常量 =====
const TAG_INT: i64 = 0;
const TAG_BOOL: i64 = 1;
const TAG_UNIT: i64 = 2;
const TAG_F64: i64 = 3;
const TAG_STR: i64 = 4;

const V_FALSE: i64 = 1; // (0<<3)|1
const V_TRUE: i64 = 9; //  (1<<3)|1
const V_UNIT: i64 = 2; //  (0<<3)|2

// ===== 导入函数索引（函数索引空间：导入在前）=====
const IMP_PRINT_INT: u32 = 0; // (i64 v, i64 newline) -> ()
const IMP_PRINT_FLOAT: u32 = 1; // (f64 v, i64 newline) -> ()
const IMP_PRINT_BOOL: u32 = 2; // (i64 v, i64 newline) -> ()
const IMP_PRINT_UNIT: u32 = 3; // (i64 newline) -> ()
const IMP_PRINT_STR: u32 = 4; // (i32 ptr, i32 len) -> () —— 原始字节

// ===== 运行时 helper 函数索引 =====
const RT_BOX_F64: u32 = 5; // (f64) -> i64      动态装盒（bump alloc 8 字节）
const RT_UNBOX_F64: u32 = 6; // (i64) -> f64    拆盒
const RT_PROMOTE_F64: u32 = 7; // (i64) -> f64  Int→convert / F64→unbox / 其他 trap
const RT_STR_EQ: u32 = 8; // (i64, i64) -> i32 字符串按字节相等
const RT_ADD: u32 = 9; // (i64, i64) -> i64
const RT_SUB: u32 = 10;
const RT_MUL: u32 = 11;
const RT_DIV: u32 = 12;
const RT_MOD: u32 = 13;
const RT_LT: u32 = 14; // (i64, i64) -> i64（Bool tagged）
const RT_GT: u32 = 15;
const RT_LE: u32 = 16;
const RT_GE: u32 = 17;
const RT_EQ: u32 = 18;
const RT_NE: u32 = 19;
const RT_NEG: u32 = 20; // (i64) -> i64
const RT_NOT: u32 = 21; // (i64) -> i64
const RT_TRUTHY: u32 = 22; // (i64) -> i32      非 Bool 时 trap
const RT_PRINT: u32 = 23; // (i64 v, i64 newline) -> ()
const RT_ALLOC: u32 = 24; // (i32 size) -> i32  bump allocator（7.3 起：闭包 env/对象分配）
/// 第一个用户函数的 funcidx
const FIRST_USER_FN: u32 = 25;

/// 闭包值 tag：堆对象布局 [table_idx: i32][env: i32]（env 指向 [n: i32][v0..vn: i64]）
const TAG_CLOSURE: i64 = 5;

// ===== 最小汇编器（WASM 指令按顺序发射；条件必须先算好再写 if/br_if）=====
#[derive(Default)]
struct Asm {
    b: Vec<u8>,
}

#[allow(dead_code)]
impl Asm {
    fn new() -> Self {
        Asm { b: Vec::new() }
    }
    fn op(&mut self, o: u8) -> &mut Self {
        self.b.push(o);
        self
    }
    fn i64c(&mut self, v: i64) -> &mut Self {
        self.b.push(op::I64_CONST);
        self.b.extend(leb_s(v));
        self
    }
    fn i32c(&mut self, v: i32) -> &mut Self {
        self.b.push(op::I32_CONST);
        self.b.extend(leb_s(v as i64));
        self
    }
    fn lget(&mut self, i: u32) -> &mut Self {
        self.b.push(op::LOCAL_GET);
        self.b.extend(leb_u(i as u64));
        self
    }
    fn lset(&mut self, i: u32) -> &mut Self {
        self.b.push(op::LOCAL_SET);
        self.b.extend(leb_u(i as u64));
        self
    }
    fn call(&mut self, f: u32) -> &mut Self {
        self.b.push(op::CALL);
        self.b.extend(leb_u(f as u64));
        self
    }
    fn gget(&mut self, i: u32) -> &mut Self {
        self.b.push(op::GLOBAL_GET);
        self.b.extend(leb_u(i as u64));
        self
    }
    fn gset(&mut self, i: u32) -> &mut Self {
        self.b.push(op::GLOBAL_SET);
        self.b.extend(leb_u(i as u64));
        self
    }
    /// block 的开始（块类型：空）
    fn block(&mut self) -> &mut Self {
        self.op(op::BLOCK).op(0x40)
    }
    /// block 的开始（块类型：结果 i64）——函数级 $ret 用
    fn block_i64(&mut self) -> &mut Self {
        self.op(op::BLOCK).op(0x7E)
    }
    /// loop 的开始（块类型：空）
    fn loop_(&mut self) -> &mut Self {
        self.op(op::LOOP).op(0x40)
    }
    /// if 的开始（块类型：空）——条件须已压栈
    fn if_(&mut self) -> &mut Self {
        self.op(op::IF).op(0x40)
    }
    /// if 的开始（块类型：结果 i64）
    fn if_i64(&mut self) -> &mut Self {
        self.op(op::IF).op(0x7E)
    }
    /// if 的开始（块类型：结果 f64）
    fn if_f64(&mut self) -> &mut Self {
        self.op(op::IF).op(0x7C)
    }
    /// if 的开始（块类型：结果 i32）
    fn if_i32(&mut self) -> &mut Self {
        self.op(op::IF).op(0x7F)
    }
    fn else_(&mut self) -> &mut Self {
        self.op(op::ELSE)
    }
    fn end(&mut self) -> &mut Self {
        self.op(op::END)
    }
    fn br(&mut self, depth: u32) -> &mut Self {
        self.op(op::BR);
        self.b.extend(leb_u(depth as u64));
        self
    }
    fn br_if(&mut self, depth: u32) -> &mut Self {
        self.op(op::BR_IF);
        self.b.extend(leb_u(depth as u64));
        self
    }
    /// memarg: align=0（1 字节对齐，永远安全）+ offset
    fn i32_load(&mut self, offset: u32) -> &mut Self {
        self.op(op::I32_LOAD);
        self.b.extend([0x00]);
        self.b.extend(leb_u(offset as u64));
        self
    }
    fn i32_load8_u(&mut self, offset: u32) -> &mut Self {
        self.op(op::I32_LOAD8_U);
        self.b.extend([0x00]);
        self.b.extend(leb_u(offset as u64));
        self
    }
    fn i32_store(&mut self, offset: u32) -> &mut Self {
        self.op(op::I32_STORE);
        self.b.extend([0x00]);
        self.b.extend(leb_u(offset as u64));
        self
    }
    fn i64_load(&mut self, offset: u32) -> &mut Self {
        self.op(op::I64_LOAD);
        self.b.extend([0x00]);
        self.b.extend(leb_u(offset as u64));
        self
    }
    fn i64_store(&mut self, offset: u32) -> &mut Self {
        self.op(op::I64_STORE);
        self.b.extend([0x00]);
        self.b.extend(leb_u(offset as u64));
        self
    }
    /// call_indirect：栈上先压参数、最后压表索引；table 0 固定
    fn call_indirect(&mut self, type_idx: u32) -> &mut Self {
        self.op(op::CALL_INDIRECT);
        self.b.extend(leb_u(type_idx as u64));
        self.b.push(0x00);
        self
    }
    fn f64_load(&mut self, offset: u32) -> &mut Self {
        self.op(op::F64_LOAD);
        self.b.extend([0x00]);
        self.b.extend(leb_u(offset as u64));
        self
    }
    fn f64_store(&mut self, offset: u32) -> &mut Self {
        self.op(op::F64_STORE);
        self.b.extend([0x00]);
        self.b.extend(leb_u(offset as u64));
        self
    }
    /// 栈顶 i64 → tag（v & 7）
    fn tag(&mut self) -> &mut Self {
        self.i64c(7).op(op::I64_AND)
    }
    /// local `l` 的 tag == `tag`？（→ i32）
    fn tag_is(&mut self, l: u32, tag: i64) -> &mut Self {
        self.lget(l).tag().i64c(tag).op(op::I64_EQ)
    }
    /// Int tagged → 原值（>>3 算术）
    fn untag(&mut self) -> &mut Self {
        self.i64c(3).op(op::I64_SHR_S)
    }
    /// 原值 → Int tagged（<<3）
    fn tag_int(&mut self) -> &mut Self {
        self.i64c(3).op(op::I64_SHL)
    }
    /// i32(0/1) → Bool tagged i64
    fn bool_tag(&mut self) -> &mut Self {
        self.op(op::I64_EXTEND_I32_S)
            .i64c(3)
            .op(op::I64_SHL)
            .i64c(TAG_BOOL)
            .op(op::I64_OR)
    }
}

/// 编译错误（不支持的构造，信息里指明哪个 Phase 7.x 支持）
fn unsupported<T>(what: &str, phase: &str) -> Result<T, String> {
    Err(format!("WASM 后端暂不支持 {}（将在 {} 支持）", what, phase))
}

/// 标签种类（br 深度计算用）
#[derive(Clone, Copy, PartialEq)]
enum Label {
    Block, // break 目标
    Loop,  // continue 目标
    If,    // if 也是 WASM label（br 深度要计入），但不是 break/continue 目标
}

/// 单函数编译上下文
struct FnCtx {
    n_params: u32,
    /// 用户局部变量（不含参数；全是 I64）
    locals: Vec<ValType>,
    /// 作用域栈：名字 → local 索引（内层遮蔽外层，与解释器 Scope 链一致）
    scopes: Vec<Vec<(String, u32)>>,
    /// 闭包捕获表：名字 → 捕获槽位（值在 env 对象 4+8*i 处；env 是 local 0）
    /// 仅闭包函数非空
    captures: Vec<(String, u32)>,
    /// 标签栈（labels[0] 永远是函数级 $ret 块）
    labels: Vec<Label>,
}

impl FnCtx {
    fn new(n_params: u32) -> Self {
        FnCtx {
            n_params,
            locals: Vec::new(),
            scopes: vec![Vec::new()],
            captures: Vec::new(),
            labels: vec![Label::Block], // $ret
        }
    }
    fn alloc(&mut self) -> u32 {
        self.locals.push(ValType::I64);
        self.n_params + self.locals.len() as u32 - 1
    }
    fn bind(&mut self, name: &str) -> u32 {
        let idx = self.alloc();
        self.scopes.last_mut().unwrap().push((name.to_string(), idx));
        idx
    }
    fn lookup(&self, name: &str) -> Option<u32> {
        for scope in self.scopes.iter().rev() {
            for (n, i) in scope.iter().rev() {
                if n == name {
                    return Some(*i);
                }
            }
        }
        None
    }
    /// 捕获槽位查找（闭包函数内）
    fn capture_slot(&self, name: &str) -> Option<u32> {
        self.captures.iter().find(|(n, _)| n == name).map(|(_, i)| *i)
    }
    /// labels 中第 pos 个（0=最外层 $ret）相对当前的 br 深度
    fn depth(&self, pos: usize) -> u32 {
        (self.labels.len() - 1 - pos) as u32
    }
    /// 最近一层 Block 标签（break 目标）的深度
    fn break_depth(&self) -> u32 {
        for (i, l) in self.labels.iter().enumerate().rev() {
            if *l == Label::Block {
                return self.depth(i);
            }
        }
        0 // $ret
    }
}

/// 编译器主结构
pub struct Codegen {
    m: Module,
    /// 用户函数名 → funcidx（先注册后编译，支持递归/前向引用/互递归）
    fn_idx: std::collections::HashMap<String, u32>,
    /// 用户函数名 → 参数个数（编译期 arity 检查）
    fn_arity: std::collections::HashMap<String, usize>,
    /// 具名函数当值时的 shim：函数名 → (shim funcidx, table 槽位)
    shim_idx: std::collections::HashMap<String, (u32, u32)>,
    /// funcref 表内容（table 槽位 i → funcidx）；闭包/函数值的 call_indirect 目标
    table_entries: Vec<u32>,
    /// 枚举变体名集合（用于给出"7.5 才支持"的明确错误）
    variant_names: std::collections::HashSet<String>,
    /// 静态数据镜像（offset 0 起）；字节 0 固定是 '\n'（rt_print 换行用）
    data: Vec<u8>,
    str_off: std::collections::HashMap<String, u32>,
    f64_off: std::collections::HashMap<u64, u32>,
}

/// 编译整个程序为 WASM 二进制。失败返回中文错误（不支持的构造/未定义符号等）。
pub fn compile_program(prog: &Program) -> Result<Vec<u8>, String> {
    let mut cg = Codegen::new();

    // 第一遍：注册函数名（支持递归/前向引用）、收集枚举变体名、校验导入
    let mut next_fn = FIRST_USER_FN;
    for item in &prog.items {
        match item {
            Item::Fn(f) => {
                if cg.fn_idx.contains_key(&f.name) {
                    return Err(format!("WASM 编译：函数 '{}' 重复定义", f.name));
                }
                cg.fn_idx.insert(f.name.clone(), next_fn);
                cg.fn_arity.insert(f.name.clone(), f.params.len());
                next_fn += 1;
            }
            Item::Enum(e) => {
                for v in &e.variants {
                    cg.variant_names.insert(v.name.clone());
                }
            }
            Item::Import(imp) => {
                // 7.2 只支持 io 模块（println/print 在 prelude 已可用，显式导入等价 no-op）
                if imp.module != "io" {
                    return Err(format!(
                        "WASM 后端暂不支持导入模块 '{}'（将在 7.4+ 支持；io 的 println/print 无需导入）",
                        imp.module
                    ));
                }
            }
        }
    }
    if !cg.fn_idx.contains_key("main") {
        return Err("WASM 编译：程序缺少 main 函数".to_string());
    }

    // 第一遍收尾：为每个用户函数预推占位 Function——
    // 固定 funcidx 布局（函数体编译期间产生的闭包函数只能往后排，不能挤占用户函数槽位）
    for item in &prog.items {
        if let Item::Fn(f) = item {
            let ty = cg.m.add_type(FuncType {
                params: vec![ValType::I64; f.params.len()],
                results: vec![ValType::I64],
            });
            cg.m.funcs.push(Function { type_idx: ty, locals: vec![], body: vec![] });
        }
    }

    // 第二遍：编译函数体（写入占位槽位）
    for item in &prog.items {
        if let Item::Fn(f) = item {
            let func = cg.compile_fn(f)?;
            let slot = (cg.fn_idx[&f.name] - 5) as usize; // funcidx - 导入数 = funcs 下标
            cg.m.funcs[slot] = func;
        }
    }

    // 收尾：funcref 表 + 数据段 + 堆指针全局（heap 从静态数据末尾开始，arena bump）
    if !cg.table_entries.is_empty() {
        let n = cg.table_entries.len() as u32;
        cg.m.table = Some((n, Some(n)));
        cg.m.elems = cg.table_entries.clone();
    }
    let heap_base = cg.data.len() as u32;
    let pages = (heap_base / 65536) + 1;
    cg.m.memory_min_pages = Some(pages.max(1));
    cg.m.globals.push(Global { ty: ValType::I32, mutable: true, init: heap_base as i64 });
    cg.m.exports.push(("memory".into(), ExportKind::Memory, 0));
    let main_idx = cg.fn_idx["main"];
    cg.m.exports.push(("main".into(), ExportKind::Func, main_idx));
    cg.m.data.push(DataSegment { offset: 0, bytes: std::mem::take(&mut cg.data) });

    Ok(cg.m.encode())
}

impl Codegen {
    fn new() -> Self {
        let mut m = Module::new();
        // 静态数据开头：offset 0 = '\n'（rt_print 换行）；随后是 "<闭包>" 字面量（rt_print 的 tag5 分支用）
        let mut data: Vec<u8> = vec![b'\n'];
        let closure_off = data.len() as u32;
        data.extend(8u32.to_le_bytes()); // "<闭包>" = 8 字节 UTF-8
        data.extend("<闭包>".as_bytes());
        // 类型注册（add_type 自动去重）
        let ty_ii_unit = m.add_type(FuncType { params: vec![ValType::I64, ValType::I64], results: vec![] });
        let ty_fi_unit = m.add_type(FuncType { params: vec![ValType::F64, ValType::I64], results: vec![] });
        let ty_i_unit = m.add_type(FuncType { params: vec![ValType::I64], results: vec![] });
        let ty_pp_unit = m.add_type(FuncType { params: vec![ValType::I32, ValType::I32], results: vec![] });
        let ty_f_i64 = m.add_type(FuncType { params: vec![ValType::F64], results: vec![ValType::I64] });
        let ty_i64_f = m.add_type(FuncType { params: vec![ValType::I64], results: vec![ValType::F64] });
        let ty_ii_i32 = m.add_type(FuncType { params: vec![ValType::I64, ValType::I64], results: vec![ValType::I32] });
        let ty_ii_i64 = m.add_type(FuncType { params: vec![ValType::I64, ValType::I64], results: vec![ValType::I64] });
        let ty_i64_i64 = m.add_type(FuncType { params: vec![ValType::I64], results: vec![ValType::I64] });
        let ty_i64_i32 = m.add_type(FuncType { params: vec![ValType::I64], results: vec![ValType::I32] });
        let ty_i32_i32 = m.add_type(FuncType { params: vec![ValType::I32], results: vec![ValType::I32] });

        // 导入（funcidx 0-4）
        for (name, ty) in [
            ("lom_print_int", ty_ii_unit),
            ("lom_print_float", ty_fi_unit),
            ("lom_print_bool", ty_ii_unit),
            ("lom_print_unit", ty_i_unit),
            ("lom_print", ty_pp_unit),
        ] {
            m.imports.push(Import { module: "env".into(), name: name.into(), type_idx: ty });
        }

        // 运行时 helper（funcidx 5-24；新增 helper 必须同步 FIRST_USER_FN 与上面的 RT_* 常量）
        let helpers: Vec<(u32, Vec<ValType>, Vec<u8>)> = vec![
            (ty_f_i64, vec![ValType::I64], build_box_f64()),
            (ty_i64_f, vec![], build_unbox_f64()),
            (ty_i64_f, vec![], build_promote_f64()),
            (ty_ii_i32, vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32], build_str_eq()),
            (ty_ii_i64, vec![], build_arith(ArithKind::Add)),
            (ty_ii_i64, vec![], build_arith(ArithKind::Sub)),
            (ty_ii_i64, vec![], build_arith(ArithKind::Mul)),
            (ty_ii_i64, vec![], build_arith(ArithKind::Div)),
            (ty_ii_i64, vec![], build_arith(ArithKind::Mod)),
            (ty_ii_i64, vec![], build_cmp(op::I64_LT_S, op::F64_LT)),
            (ty_ii_i64, vec![], build_cmp(op::I64_GT_S, op::F64_GT)),
            (ty_ii_i64, vec![], build_cmp(op::I64_LE_S, op::F64_LE)),
            (ty_ii_i64, vec![], build_cmp(op::I64_GE_S, op::F64_GE)),
            (ty_ii_i64, vec![], build_eq()),
            (ty_ii_i64, vec![], build_ne()),
            (ty_i64_i64, vec![], build_neg()),
            (ty_i64_i64, vec![], build_not()),
            (ty_i64_i32, vec![], build_truthy()),
            (ty_ii_unit, vec![ValType::I32], build_print(closure_off)),
            (ty_i32_i32, vec![ValType::I32], build_alloc()),
        ];
        for (ty, locals, body) in helpers {
            m.funcs.push(Function { type_idx: ty, locals, body });
        }

        let mut variant_names = std::collections::HashSet::new();
        for v in ["Ok", "Err", "Some", "None"] {
            variant_names.insert(v.to_string());
        }
        Codegen {
            m,
            fn_idx: std::collections::HashMap::new(),
            fn_arity: std::collections::HashMap::new(),
            shim_idx: std::collections::HashMap::new(),
            table_entries: Vec::new(),
            variant_names,
            data,
            str_off: std::collections::HashMap::new(),
            f64_off: std::collections::HashMap::new(),
        }
    }

    /// 字符串字面量 → 静态数据段（去重），返回 tagged 值
    fn intern_str(&mut self, s: &str) -> i64 {
        let off = if let Some(&o) = self.str_off.get(s) {
            o
        } else {
            let o = self.data.len() as u32;
            self.data.extend((s.len() as u32).to_le_bytes());
            self.data.extend(s.as_bytes());
            self.str_off.insert(s.to_string(), o);
            o
        };
        ((off as i64) << 3) | TAG_STR
    }

    /// f64 字面量 → 静态数据段装盒（去重），返回 tagged 值
    fn intern_f64(&mut self, f: f64) -> i64 {
        let bits = f.to_bits();
        let off = if let Some(&o) = self.f64_off.get(&bits) {
            o
        } else {
            let o = self.data.len() as u32;
            self.data.extend(bits.to_le_bytes());
            self.f64_off.insert(bits, o);
            o
        };
        ((off as i64) << 3) | TAG_F64
    }

    /// 编译一个用户函数（返回 Function，由调用方写入预分配的槽位）
    fn compile_fn(&mut self, f: &FnDecl) -> Result<Function, String> {
        let ty = self.m.add_type(FuncType {
            params: vec![ValType::I64; f.params.len()],
            results: vec![ValType::I64],
        });
        let mut ctx = FnCtx::new(f.params.len() as u32);
        // 参数占用 local 0..n（WASM 约定），直接登记进最外层作用域
        for (i, p) in f.params.iter().enumerate() {
            ctx.scopes[0].push((p.name.clone(), i as u32));
        }
        let mut a = Asm::new();
        // 函数体包在 $ret 块里（return = 带值 br 到 $ret）
        a.block_i64();
        self.compile_block_value(&mut ctx, &mut a, &f.body)?;
        a.end();
        Ok(Function { type_idx: ty, locals: ctx.locals, body: a.b })
    }

    /// 编译块（值语境）：语句 + 尾表达式（无尾表达式则补 Unit）
    fn compile_block_value(&mut self, ctx: &mut FnCtx, a: &mut Asm, block: &Block) -> Result<(), String> {
        ctx.scopes.push(Vec::new());
        for s in &block.stmts {
            self.compile_stmt(ctx, a, s)?;
        }
        match &block.tail {
            Some(e) => self.compile_expr(ctx, a, e)?,
            None => {
                a.i64c(V_UNIT);
            }
        }
        ctx.scopes.pop();
        Ok(())
    }

    /// 编译块（语句语境）：尾表达式的值丢弃
    fn compile_block_stmt(&mut self, ctx: &mut FnCtx, a: &mut Asm, block: &Block) -> Result<(), String> {
        ctx.scopes.push(Vec::new());
        for s in &block.stmts {
            self.compile_stmt(ctx, a, s)?;
        }
        if let Some(e) = &block.tail {
            self.compile_expr(ctx, a, e)?;
            a.op(op::DROP);
        }
        ctx.scopes.pop();
        Ok(())
    }

    fn compile_stmt(&mut self, ctx: &mut FnCtx, a: &mut Asm, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Let { name, value, .. } => {
                // 递归闭包特例（对齐解释器的共享作用域语义）：
                // `let f = fn(...) ... f ... end` 在创建时引用自己——值拷贝捕获此刻拿不到
                // 真值，故预绑定 local，创建后把闭包值补丁进 env 的自身槽位。
                if let Expr::Closure { params, body, .. } = value {
                    let frees = free_vars_block(body, &param_names(params));
                    if frees.iter().any(|n| n == name) {
                        let idx = ctx.bind(name); // 预绑定（值稍后填）
                        let (env_l, caps) = self.compile_closure(ctx, a, params, body)?;
                        a.lset(idx); // 闭包值入 local
                        if let Some(pos) = caps.iter().position(|n| n == name) {
                            // 补丁：env 里自身槽位写入真值
                            a.lget(env_l).op(op::I32_WRAP_I64).lget(idx).i64_store(4 + 8 * pos as u32);
                        }
                        return Ok(());
                    }
                }
                self.compile_expr(ctx, a, value)?;
                let idx = ctx.bind(name);
                a.lset(idx);
                Ok(())
            }
            Stmt::LetDestruct { .. } => unsupported("元组解构 let (a, b) = ...", "Phase 7.6"),
            Stmt::Assign { target, value } => {
                let idx = match ctx.lookup(target) {
                    Some(i) => i,
                    None => return Err(format!("WASM 编译：赋值给未定义变量 '{}'", target)),
                };
                self.compile_expr(ctx, a, value)?;
                a.lset(idx);
                Ok(())
            }
            Stmt::If(if_stmt) => self.compile_if(ctx, a, if_stmt, false),
            Stmt::While { cond, body } => {
                // block $break { loop $cont { if (!cond) break; body; continue } }
                a.block();
                ctx.labels.push(Label::Block);
                a.loop_();
                ctx.labels.push(Label::Loop);
                self.compile_expr(ctx, a, cond)?;
                a.call(RT_TRUTHY).op(op::I32_EQZ);
                let bd = ctx.break_depth();
                a.br_if(bd);
                self.compile_block_stmt(ctx, a, body)?;
                a.br(ctx.depth(ctx.labels.len() - 1)); // continue → loop 头
                ctx.labels.pop();
                a.end();
                ctx.labels.pop();
                a.end();
                Ok(())
            }
            Stmt::For { var, iter, body } => {
                // 7.2 仅支持 for i in <Int>（0..n 语义）；String/List 迭代在 7.4/7.6
                let it = ctx.alloc();
                let limit = ctx.alloc();
                let cnt = ctx.alloc();
                let var_idx = ctx.alloc();
                self.compile_expr(ctx, a, iter)?;
                a.lset(it);
                // 运行时必须为 Int，否则 trap（与解释器"for 循环不支持迭代 X"对应，7.2 无结构化消息）
                a.tag_is(it, TAG_INT).if_().else_().op(op::UNREACHABLE).end();
                a.lget(it).untag().lset(limit);
                a.i64c(0).lset(cnt);
                a.block();
                ctx.labels.push(Label::Block);
                a.loop_();
                ctx.labels.push(Label::Loop);
                a.lget(cnt).lget(limit).op(op::I64_GE_S);
                let bd = ctx.break_depth();
                a.br_if(bd);
                a.lget(cnt).tag_int().lset(var_idx);
                ctx.scopes.push(vec![(var.clone(), var_idx)]);
                for s in &body.stmts {
                    self.compile_stmt(ctx, a, s)?;
                }
                if let Some(e) = &body.tail {
                    self.compile_expr(ctx, a, e)?;
                    a.op(op::DROP);
                }
                ctx.scopes.pop();
                a.lget(cnt).i64c(1).op(op::I64_ADD).lset(cnt);
                a.br(ctx.depth(ctx.labels.len() - 1));
                ctx.labels.pop();
                a.end();
                ctx.labels.pop();
                a.end();
                Ok(())
            }
            Stmt::Return(expr) => {
                match expr {
                    Some(e) => self.compile_expr(ctx, a, e)?,
                    None => {
                        a.i64c(V_UNIT);
                    }
                }
                a.br(ctx.depth(0)); // br $ret
                Ok(())
            }
            Stmt::Expr(e) => {
                self.compile_expr(ctx, a, e)?;
                a.op(op::DROP);
                Ok(())
            }
            Stmt::Hole { line, col } => Err(format!(
                "WASM 编译：代码洞 @ {}:{}（源文件解析失败处），无法编译",
                line, col
            )),
        }
    }

    /// if/elif/else（want_value=true 时值语境，块类型 i64）
    fn compile_if(&mut self, ctx: &mut FnCtx, a: &mut Asm, if_stmt: &IfStmt, want_value: bool) -> Result<(), String> {
        self.compile_if_from(ctx, a, if_stmt, 0, want_value)
    }

    fn compile_if_from(&mut self, ctx: &mut FnCtx, a: &mut Asm, if_stmt: &IfStmt, branch_idx: usize, want_value: bool) -> Result<(), String> {
        let (cond, body) = &if_stmt.branches[branch_idx];
        self.compile_expr(ctx, a, cond)?;
        a.call(RT_TRUTHY);
        if want_value {
            a.if_i64();
        } else {
            a.if_();
        }
        ctx.labels.push(Label::If); // if 是 label，br 深度必须计入（7.2 实测 bug：漏计导致 return 穿透失败）
        if want_value {
            self.compile_block_value(ctx, a, body)?;
        } else {
            self.compile_block_stmt(ctx, a, body)?;
        }
        let last = branch_idx + 1 == if_stmt.branches.len();
        if !last || if_stmt.else_branch.is_some() || want_value {
            a.else_();
            if !last {
                self.compile_if_from(ctx, a, if_stmt, branch_idx + 1, want_value)?;
            } else if let Some(else_body) = &if_stmt.else_branch {
                if want_value {
                    self.compile_block_value(ctx, a, else_body)?;
                } else {
                    self.compile_block_stmt(ctx, a, else_body)?;
                }
            } else {
                // 值语境缺 else：与解释器一致补 Unit
                a.i64c(V_UNIT);
            }
        }
        a.end();
        ctx.labels.pop();
        Ok(())
    }

    fn compile_expr(&mut self, ctx: &mut FnCtx, a: &mut Asm, expr: &Expr) -> Result<(), String> {
        match expr {
            Expr::Int(n) => {
                a.i64c(n << 3); // tag 0
                Ok(())
            }
            Expr::Float(f) => {
                let v = self.intern_f64(*f);
                a.i64c(v);
                Ok(())
            }
            Expr::Bool(bv) => {
                a.i64c(if *bv { V_TRUE } else { V_FALSE });
                Ok(())
            }
            Expr::Str(s) => {
                let v = self.intern_str(s);
                a.i64c(v);
                Ok(())
            }
            Expr::Unit => {
                a.i64c(V_UNIT);
                Ok(())
            }
            Expr::Ident(name) => {
                if let Some(idx) = ctx.lookup(name) {
                    a.lget(idx);
                    return Ok(());
                }
                // 闭包函数内：从 env 对象读捕获值（env 是 local 0，槽位值在 4+8*slot）
                if let Some(slot) = ctx.capture_slot(name) {
                    a.lget(0).op(op::I32_WRAP_I64).i64_load(4 + 8 * slot);
                    return Ok(());
                }
                if self.variant_names.contains(name) {
                    return unsupported(&format!("枚举变体 '{}'", name), "Phase 7.5");
                }
                if self.fn_idx.contains_key(name) {
                    // 具名函数当值（v0.4.2 语义）：包装为零捕获闭包（shim 忽略 env 直调原函数）
                    return self.emit_fn_value(ctx, a, name);
                }
                Err(format!("WASM 编译：未定义变量 '{}'", name))
            }
            Expr::Binary { op: bop, left, right } => {
                let helper = match bop {
                    BinOp::Add => RT_ADD,
                    BinOp::Sub => RT_SUB,
                    BinOp::Mul => RT_MUL,
                    BinOp::Div => RT_DIV,
                    BinOp::Mod => RT_MOD,
                    BinOp::Eq => RT_EQ,
                    BinOp::NotEq => RT_NE,
                    BinOp::Lt => RT_LT,
                    BinOp::Gt => RT_GT,
                    BinOp::LtEq => RT_LE,
                    BinOp::GtEq => RT_GE,
                };
                self.compile_expr(ctx, a, left)?;
                self.compile_expr(ctx, a, right)?;
                a.call(helper);
                Ok(())
            }
            Expr::Unary { op: uop, expr: inner } => {
                self.compile_expr(ctx, a, inner)?;
                a.call(match uop {
                    UnaryOp::Neg => RT_NEG,
                    UnaryOp::Not => RT_NOT,
                });
                Ok(())
            }
            Expr::Logical { op: lop, left, right } => {
                // 解释器语义：and/or 的结果是 Bool（对右侧取真值），不是右侧原值
                let sc = ctx.alloc();
                self.compile_expr(ctx, a, left)?;
                a.lset(sc);
                a.lget(sc).call(RT_TRUTHY).if_i64();
                match lop {
                    LogicalOp::And => {
                        self.compile_expr(ctx, a, right)?;
                        a.call(RT_TRUTHY).bool_tag();
                        a.else_().i64c(V_FALSE);
                    }
                    LogicalOp::Or => {
                        a.i64c(V_TRUE);
                        a.else_();
                        self.compile_expr(ctx, a, right)?;
                        a.call(RT_TRUTHY).bool_tag();
                    }
                }
                a.end();
                Ok(())
            }
            Expr::Call { callee, args } => self.compile_call(ctx, a, callee, args),
            Expr::Group(inner) => self.compile_expr(ctx, a, inner),
            Expr::If(if_stmt) => self.compile_if(ctx, a, if_stmt, true),
            Expr::Pipe { left, right } => {
                // 去糖：x |> f → f(x)；x |> f(args) → f(x, args...)
                match right.as_ref() {
                    Expr::Ident(name) => {
                        let args = vec![left.as_ref().clone()];
                        self.compile_call(ctx, a, &Expr::Ident(name.clone()), &args)
                    }
                    Expr::Call { callee, args } => {
                        let mut new_args = vec![left.as_ref().clone()];
                        new_args.extend(args.iter().cloned());
                        self.compile_call(ctx, a, callee, &new_args)
                    }
                    _ => Err("WASM 编译：管道右侧必须是函数名或调用".to_string()),
                }
            }
            Expr::Closure { params, body, .. } => {
                self.compile_closure(ctx, a, params, body)?;
                Ok(())
            }
            Expr::Match(_) => unsupported("match", "Phase 7.5"),
            Expr::Try(_) => unsupported("? 操作符", "Phase 7.5"),
            Expr::Record { .. } => unsupported("记录", "Phase 7.6"),
            Expr::Tuple { .. } => unsupported("元组", "Phase 7.6"),
            Expr::Index { .. } => unsupported("索引", "Phase 7.6"),
            Expr::Field { .. } => unsupported("字段访问", "Phase 7.6"),
            Expr::Range { .. } => unsupported("range 表达式 a..b", "Phase 7.6"),
        }
    }

    fn compile_call(&mut self, ctx: &mut FnCtx, a: &mut Asm, callee: &Expr, args: &[Expr]) -> Result<(), String> {
        if let Expr::Ident(name) = callee {
            // 顺序对齐解释器 eval_call：变体（未被遮蔽）→ 内建 → 用户函数 → 闭包变量
            if self.variant_names.contains(name.as_str())
                && ctx.lookup(name).is_none()
                && ctx.capture_slot(name).is_none()
            {
                return unsupported(&format!("枚举变体构造 '{}(...)'", name), "Phase 7.5");
            }
            // println / print（prelude）
            if name == "println" || name == "print" {
                if args.len() != 1 {
                    return Err(format!("WASM 编译：{} 期望 1 个参数，得到 {} 个", name, args.len()));
                }
                self.compile_expr(ctx, a, &args[0])?;
                a.i64c(if name == "println" { 1 } else { 0 });
                a.call(RT_PRINT);
                a.i64c(V_UNIT); // println 返回 Unit
                return Ok(());
            }
            // 用户函数（具名直调；注意解释器里具名函数优先于同名闭包变量，保持一致）
            if let Some(&idx) = self.fn_idx.get(name.as_str()) {
                self.check_arity(name, args.len())?;
                for arg in args {
                    self.compile_expr(ctx, a, arg)?;
                }
                a.call(idx);
                return Ok(());
            }
            // 闭包变量调用
            if ctx.lookup(name).is_some() || ctx.capture_slot(name).is_some() {
                return self.emit_closure_call(ctx, a, callee, args);
            }
            return Err(format!(
                "WASM 后端暂不支持内置函数 '{}'（7.3 仅 println/print + 用户函数 + 闭包调用；其余在 7.4+ 提供）",
                name
            ));
        }
        // 任意表达式 callee（如 make_adder(5)(10)）——闭包调用
        self.emit_closure_call(ctx, a, callee, args)
    }

    fn check_arity(&self, name: &str, got: usize) -> Result<(), String> {
        let expected = self.fn_arity.get(name).copied().unwrap_or(0);
        if expected != got {
            return Err(format!(
                "WASM 编译：函数 '{}' 期望 {} 个参数，得到 {} 个",
                name, expected, got
            ));
        }
        Ok(())
    }
}

// ===== 运行时 helper 函数体（用 Asm 手写指令流；调用约定见文件头常量表）=====

/// 算术种类（float 路径的指令序列不同：Mod 没有 f64 指令，要合成）
#[derive(Clone, Copy)]
enum ArithKind {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

impl ArithKind {
    fn int_op(self) -> u8 {
        match self {
            ArithKind::Add => op::I64_ADD,
            ArithKind::Sub => op::I64_SUB,
            ArithKind::Mul => op::I64_MUL,
            ArithKind::Div => op::I64_DIV_S,
            ArithKind::Mod => op::I64_REM_S,
        }
    }
}

/// 发射 float 路径：两操作数 promote → f64 运算 → 装盒（栈上留 tagged i64）
fn emit_float_path(a: &mut Asm, kind: ArithKind) {
    match kind {
        ArithKind::Mod => {
            // a % b = a - trunc(a / b) * b（对齐 Rust f64 % 语义）
            a.lget(0).call(RT_PROMOTE_F64); // a
            a.lget(0).call(RT_PROMOTE_F64);
            a.lget(1).call(RT_PROMOTE_F64);
            a.op(op::F64_DIV).op(op::F64_TRUNC); // trunc(a/b)
            a.lget(1).call(RT_PROMOTE_F64); // b
            a.op(op::F64_MUL).op(op::F64_SUB); // a - trunc(a/b)*b
        }
        _ => {
            let fop = match kind {
                ArithKind::Add => op::F64_ADD,
                ArithKind::Sub => op::F64_SUB,
                ArithKind::Mul => op::F64_MUL,
                ArithKind::Div => op::F64_DIV,
                ArithKind::Mod => unreachable!(),
            };
            a.lget(0).call(RT_PROMOTE_F64);
            a.lget(1).call(RT_PROMOTE_F64);
            a.op(fop);
        }
    }
    a.call(RT_BOX_F64);
}

/// rt_add/sub/mul/div/mod: (i64 l, i64 r) -> i64
/// Int/Int 走整数路径；含 Float 走 promote 路径；其他组合 trap
fn build_arith(kind: ArithKind) -> Vec<u8> {
    let mut a = Asm::new();
    a.tag_is(0, TAG_INT).if_i64();
    {
        a.tag_is(1, TAG_INT).if_i64();
        {
            a.lget(0).untag().lget(1).untag().op(kind.int_op()).tag_int();
        }
        a.else_();
        {
            a.tag_is(1, TAG_F64).if_i64();
            emit_float_path(&mut a, kind);
            a.else_().op(op::UNREACHABLE).end();
        }
        a.end();
    }
    a.else_();
    {
        a.tag_is(0, TAG_F64).if_i64();
        {
            a.tag_is(1, TAG_INT).if_i64();
            emit_float_path(&mut a, kind);
            a.else_();
            {
                a.tag_is(1, TAG_F64).if_i64();
                emit_float_path(&mut a, kind);
                a.else_().op(op::UNREACHABLE).end();
            }
            a.end();
        }
        a.else_().op(op::UNREACHABLE).end();
    }
    a.end();
    a.b
}

/// rt_lt/gt/le/ge: (i64 l, i64 r) -> i64(Bool)
/// Int/Int、F64/F64、Bool/Bool 三组同类型对（对齐解释器 eval_compare：混合类型报错→trap）
/// 注意：字符串大小比较 7.2 不支持（trap）；==/!= 走 build_eq 支持字符串
fn build_cmp(int_op: u8, f64_op: u8) -> Vec<u8> {
    let mut a = Asm::new();
    a.tag_is(0, TAG_INT).if_i64();
    {
        a.tag_is(1, TAG_INT).if_i64();
        {
            a.lget(0).untag().lget(1).untag().op(int_op).bool_tag();
        }
        a.else_().op(op::UNREACHABLE).end();
    }
    a.else_();
    {
        a.tag_is(0, TAG_F64).if_i64();
        {
            a.tag_is(1, TAG_F64).if_i64();
            {
                a.lget(0).call(RT_UNBOX_F64).lget(1).call(RT_UNBOX_F64).op(f64_op).bool_tag();
            }
            a.else_().op(op::UNREACHABLE).end();
        }
        a.else_();
        {
            // Bool/Bool：tagged 值即序（false=1 < true=9，与 Rust bool::cmp 一致）
            a.tag_is(0, TAG_BOOL).if_i64();
            {
                a.tag_is(1, TAG_BOOL).if_i64();
                {
                    a.lget(0).lget(1).op(int_op).bool_tag();
                }
                a.else_().op(op::UNREACHABLE).end();
            }
            a.else_().op(op::UNREACHABLE).end();
        }
        a.end();
    }
    a.end();
    a.b
}

/// rt_eq: (i64 l, i64 r) -> i64(Bool)
/// 跨类型恒 false（对齐解释器 values_eq：Int 1 == Float 1.0 是 false，无提升）
fn build_eq() -> Vec<u8> {
    let mut a = Asm::new();
    a.lget(0).tag().lget(1).tag().op(op::I64_NE).if_i64();
    {
        a.i64c(V_FALSE);
    }
    a.else_();
    {
        a.tag_is(0, TAG_F64).if_i64();
        {
            a.lget(0).call(RT_UNBOX_F64).lget(1).call(RT_UNBOX_F64).op(op::F64_EQ).bool_tag();
        }
        a.else_();
        {
            a.tag_is(0, TAG_STR).if_i64();
            {
                a.lget(0).lget(1).call(RT_STR_EQ).bool_tag();
            }
            a.else_();
            {
                // Int/Bool/Unit：tagged 值完全相等即值相等
                a.lget(0).lget(1).op(op::I64_EQ).bool_tag();
            }
            a.end();
        }
        a.end();
    }
    a.end();
    a.b
}

/// rt_ne: rt_eq 结果翻转 Bool 位（v ^ 8：1↔9）
fn build_ne() -> Vec<u8> {
    let mut a = Asm::new();
    a.lget(0).lget(1).call(RT_EQ).i64c(8).op(op::I64_XOR);
    a.b
}

/// rt_neg: (i64) -> i64；Int：0-(x<<3)=(-x)<<3 tag 保持；F64：拆盒取反装盒
fn build_neg() -> Vec<u8> {
    let mut a = Asm::new();
    a.tag_is(0, TAG_INT).if_i64();
    {
        a.i64c(0).lget(0).op(op::I64_SUB);
    }
    a.else_();
    {
        a.tag_is(0, TAG_F64).if_i64();
        {
            a.lget(0).call(RT_UNBOX_F64).op(op::F64_NEG).call(RT_BOX_F64);
        }
        a.else_().op(op::UNREACHABLE).end();
    }
    a.end();
    a.b
}

/// rt_not: (i64) -> i64；Bool：v ^ 8 翻转；其他 trap
fn build_not() -> Vec<u8> {
    let mut a = Asm::new();
    a.tag_is(0, TAG_BOOL).if_i64();
    {
        a.lget(0).i64c(8).op(op::I64_XOR);
    }
    a.else_().op(op::UNREACHABLE).end();
    a.b
}

/// rt_truthy: (i64) -> i32；Bool → 0/1；其他 trap（对齐解释器 is_truthy）
fn build_truthy() -> Vec<u8> {
    let mut a = Asm::new();
    a.tag_is(0, TAG_BOOL).if_i32();
    {
        a.lget(0).i64c(3).op(op::I64_SHR_U).op(op::I32_WRAP_I64);
    }
    a.else_().op(op::UNREACHABLE).end();
    a.b
}

/// rt_print: (i64 v, i64 newline) -> ()；按 tag 分派到宿主导入
/// local 2 (i32)：Str/闭包分支的指针暂存
/// closure_off：静态数据段中 "<闭包>" 字面量的偏移
fn build_print(closure_off: u32) -> Vec<u8> {
    let mut a = Asm::new();
    a.block(); // $done
    {
        a.tag_is(0, TAG_INT).if_();
        {
            a.lget(0).untag().lget(1).call(IMP_PRINT_INT).br(1);
        }
        a.end();
        a.tag_is(0, TAG_BOOL).if_();
        {
            a.lget(0).i64c(3).op(op::I64_SHR_U).lget(1).call(IMP_PRINT_BOOL).br(1);
        }
        a.end();
        a.tag_is(0, TAG_UNIT).if_();
        {
            a.lget(1).call(IMP_PRINT_UNIT).br(1);
        }
        a.end();
        a.tag_is(0, TAG_F64).if_();
        {
            a.lget(0).call(RT_UNBOX_F64).lget(1).call(IMP_PRINT_FLOAT).br(1);
        }
        a.end();
        a.tag_is(0, TAG_STR).if_();
        {
            // ptr = wrap(v >> 3)
            a.lget(0).i64c(3).op(op::I64_SHR_U).op(op::I32_WRAP_I64).lset(2);
            // lom_print(ptr + 4, len)
            a.lget(2).i32c(4).op(op::I32_ADD).lget(2).i32_load(0).call(IMP_PRINT_STR);
            // newline：打数据段 offset 0 处的 '\n'
            a.lget(1).i64c(0).op(op::I64_NE).if_();
            {
                a.i32c(0).i32c(1).call(IMP_PRINT_STR);
            }
            a.end();
            a.br(1);
        }
        a.end();
        a.tag_is(0, TAG_CLOSURE).if_();
        {
            // 打印 "<闭包>"（对齐解释器 to_display）
            a.i32c((closure_off + 4) as i32).i32c(8).call(IMP_PRINT_STR);
            a.lget(1).i64c(0).op(op::I64_NE).if_();
            {
                a.i32c(0).i32c(1).call(IMP_PRINT_STR);
            }
            a.end();
            a.br(1);
        }
        a.end();
        a.op(op::UNREACHABLE); // 其他 tag（List/Map/...）将在 7.6 支持
    }
    a.end();
    a.b
}

/// rt_alloc: (i32 size) -> i32；bump allocator（arena，不释放）
/// local 1 (i32)：旧 hp 暂存（作为返回值）
fn build_alloc() -> Vec<u8> {
    let mut a = Asm::new();
    a.gget(0).lset(1); // result = hp
    a.gget(0).lget(0).op(op::I32_ADD).gset(0); // hp += size
    a.lget(1);
    a.b
}

/// rt_box_f64: (f64) -> i64；mem[hp] = v，返回 (hp<<3)|3，hp += 8
/// local 1 (i64)：tagged 结果暂存
fn build_box_f64() -> Vec<u8> {
    let mut a = Asm::new();
    a.gget(0).lget(0).f64_store(0);
    a.gget(0).op(op::I64_EXTEND_I32_S).tag_int().i64c(TAG_F64).op(op::I64_OR).lset(1);
    a.gget(0).i32c(8).op(op::I32_ADD).gset(0);
    a.lget(1);
    a.b
}

/// rt_unbox_f64: (i64 tagged) -> f64
fn build_unbox_f64() -> Vec<u8> {
    let mut a = Asm::new();
    a.lget(0).i64c(3).op(op::I64_SHR_U).op(op::I32_WRAP_I64).f64_load(0);
    a.b
}

/// rt_promote_f64: (i64) -> f64；Int → convert，F64 → 拆盒，其他 trap
fn build_promote_f64() -> Vec<u8> {
    let mut a = Asm::new();
    a.tag_is(0, TAG_INT).if_f64();
    {
        a.lget(0).untag().op(op::F64_CONVERT_I64_S);
    }
    a.else_();
    {
        a.tag_is(0, TAG_F64).if_f64();
        {
            a.lget(0).call(RT_UNBOX_F64);
        }
        a.else_().op(op::UNREACHABLE).end();
    }
    a.end();
    a.b
}

/// rt_str_eq: (i64 l, i64 r) -> i32；先比长度再逐字节
/// locals: 2=pl, 3=pr, 4=len, 5=i（全 i32）
fn build_str_eq() -> Vec<u8> {
    let mut a = Asm::new();
    a.lget(0).i64c(3).op(op::I64_SHR_U).op(op::I32_WRAP_I64).lset(2);
    a.lget(1).i64c(3).op(op::I64_SHR_U).op(op::I32_WRAP_I64).lset(3);
    a.lget(2).i32_load(0).lset(4);
    // 长度不等 → return 0
    a.lget(4).lget(3).i32_load(0).op(op::I32_NE).if_();
    {
        a.i32c(0).op(op::RETURN);
    }
    a.end();
    // 逐字节循环
    a.block();
    a.loop_();
    {
        a.lget(5).lget(4).op(op::I32_GE_U).br_if(1);
        a.lget(2).lget(5).op(op::I32_ADD).i32_load8_u(4);
        a.lget(3).lget(5).op(op::I32_ADD).i32_load8_u(4);
        a.op(op::I32_NE).if_();
        {
            a.i32c(0).op(op::RETURN);
        }
        a.end();
        a.lget(5).i32c(1).op(op::I32_ADD).lset(5);
        a.br(0);
    }
    a.end();
    a.end();
    a.i32c(1);
    a.b
}

// ===== 闭包支持（Phase 7.3）=====

/// 捕获来源：创建点的值从哪来
enum CapSrc {
    /// 当前函数的 local
    Local(u32),
    /// 当前函数（本身是闭包）的 env 捕获槽位
    Capture(u32),
}

impl Codegen {
    /// 编译闭包字面量：生成 (env, params...) -> i64 的 WASM 函数 + 创建点代码。
    /// 返回 (env 暂存 local, 捕获名列表)（供递归闭包补丁用）。
    /// 栈上留下闭包值（tagged i64，tag 5）。
    fn compile_closure(
        &mut self,
        ctx: &mut FnCtx,
        a: &mut Asm,
        params: &[Param],
        body: &Block,
    ) -> Result<(u32, Vec<String>), String> {
        // 1. 自由变量分析 + 来源解析（当前 local / 外层捕获；fn/变体/prelude 全局解析不捕获）
        let frees = free_vars_block(body, &param_names(params));
        let mut caps: Vec<(String, CapSrc)> = Vec::new();
        for name in &frees {
            if let Some(idx) = ctx.lookup(name) {
                caps.push((name.clone(), CapSrc::Local(idx)));
            } else if let Some(slot) = ctx.capture_slot(name) {
                caps.push((name.clone(), CapSrc::Capture(slot)));
            } else if self.fn_idx.contains_key(name)
                || self.variant_names.contains(name)
                || name == "println"
                || name == "print"
            {
                // 全局符号：函数直调、变体（7.5 报错点在构造处）、prelude——不捕获
            } else {
                return Err(format!("WASM 编译：闭包捕获了未定义变量 '{}'", name));
            }
        }
        // 2. 闭包函数体（local 0 = env，参数从 local 1 起）
        let n = params.len();
        let ty = self.m.add_type(FuncType {
            params: vec![ValType::I64; n + 1],
            results: vec![ValType::I64],
        });
        let mut cctx = FnCtx::new((n + 1) as u32);
        for (i, p) in params.iter().enumerate() {
            cctx.scopes[0].push((p.name.clone(), (i + 1) as u32));
        }
        cctx.captures = caps.iter().enumerate().map(|(i, (nm, _))| (nm.clone(), i as u32)).collect();
        let mut ca = Asm::new();
        ca.block_i64();
        self.compile_block_value(&mut cctx, &mut ca, body)?;
        ca.end();
        let funcidx = 5 + self.m.funcs.len() as u32;
        self.m.funcs.push(Function { type_idx: ty, locals: cctx.locals, body: ca.b });
        let tslot = self.table_entries.len() as u32;
        self.table_entries.push(funcidx);
        // 3. 创建点：env 对象 [n][v0..vn] + 闭包对象 [tslot][env]
        let e = ctx.alloc();
        let p = ctx.alloc();
        a.i32c(4 + 8 * caps.len() as i32).call(RT_ALLOC).op(op::I64_EXTEND_I32_S).lset(e);
        a.lget(e).op(op::I32_WRAP_I64).i32c(caps.len() as i32).i32_store(0);
        for (i, (_, src)) in caps.iter().enumerate() {
            a.lget(e).op(op::I32_WRAP_I64);
            match src {
                CapSrc::Local(idx) => {
                    a.lget(*idx);
                }
                CapSrc::Capture(slot) => {
                    a.lget(0).op(op::I32_WRAP_I64).i64_load(4 + 8 * slot);
                }
            }
            a.i64_store(4 + 8 * i as u32);
        }
        a.i32c(8).call(RT_ALLOC).op(op::I64_EXTEND_I32_S).lset(p);
        a.lget(p).op(op::I32_WRAP_I64).i32c(tslot as i32).i32_store(0);
        a.lget(p).op(op::I32_WRAP_I64).lget(e).op(op::I32_WRAP_I64).i32_store(4);
        a.lget(p).tag_int().i64c(TAG_CLOSURE).op(op::I64_OR);
        Ok((e, caps.iter().map(|(n, _)| n.clone()).collect()))
    }

    /// 具名函数当值：零捕获闭包包 shim（shim 签名 (env, params...) 与闭包一致，忽略 env 直调）
    fn emit_fn_value(&mut self, ctx: &mut FnCtx, a: &mut Asm, name: &str) -> Result<(), String> {
        let tslot = if let Some(&(_, ts)) = self.shim_idx.get(name) {
            ts
        } else {
            let arity = self.fn_arity[name];
            let real = self.fn_idx[name];
            let ty = self.m.add_type(FuncType {
                params: vec![ValType::I64; arity + 1],
                results: vec![ValType::I64],
            });
            let mut s = Asm::new();
            for i in 0..arity {
                s.lget((i + 1) as u32);
            }
            s.call(real);
            let funcidx = 5 + self.m.funcs.len() as u32;
            self.m.funcs.push(Function { type_idx: ty, locals: vec![], body: s.b });
            let ts = self.table_entries.len() as u32;
            self.table_entries.push(funcidx);
            self.shim_idx.insert(name.to_string(), (funcidx, ts));
            ts
        };
        // 零捕获闭包对象：[tslot][env=0]
        let p = ctx.alloc();
        a.i32c(8).call(RT_ALLOC).op(op::I64_EXTEND_I32_S).lset(p);
        a.lget(p).op(op::I32_WRAP_I64).i32c(tslot as i32).i32_store(0);
        a.lget(p).op(op::I32_WRAP_I64).i32c(0).i32_store(4);
        a.lget(p).tag_int().i64c(TAG_CLOSURE).op(op::I64_OR);
        Ok(())
    }

    /// 闭包调用：求值顺序对齐解释器（先 args 后 callee），call_indirect 分派
    fn emit_closure_call(&mut self, ctx: &mut FnCtx, a: &mut Asm, callee: &Expr, args: &[Expr]) -> Result<(), String> {
        // 1. args → 暂存（解释器 eval_call 先求值参数）
        let mut scratches = Vec::with_capacity(args.len());
        for arg in args {
            self.compile_expr(ctx, a, arg)?;
            let s = ctx.alloc();
            a.lset(s);
            scratches.push(s);
        }
        // 2. callee → 暂存
        self.compile_expr(ctx, a, callee)?;
        let cl = ctx.alloc();
        a.lset(cl);
        // 3. 必须是闭包值（解释器报"不能调用 X 类型的值"，7.3 用 trap 兜底）
        a.tag_is(cl, TAG_CLOSURE).if_().else_().op(op::UNREACHABLE).end();
        // 4. 压 env（obj+4 的 i32 → i64）→ 参数 → 表索引（最后压栈）
        a.lget(cl).i64c(3).op(op::I64_SHR_U).op(op::I32_WRAP_I64).i32_load(4).op(op::I64_EXTEND_I32_S);
        for s in &scratches {
            a.lget(*s);
        }
        a.lget(cl).i64c(3).op(op::I64_SHR_U).op(op::I32_WRAP_I64).i32_load(0);
        let ty = self.m.add_type(FuncType {
            params: vec![ValType::I64; args.len() + 1],
            results: vec![ValType::I64],
        });
        a.call_indirect(ty);
        Ok(())
    }
}

// ===== 自由变量分析（闭包捕获用）=====

fn param_names(params: &[Param]) -> std::collections::HashSet<String> {
    params.iter().map(|p| p.name.clone()).collect()
}

struct FvState {
    out: Vec<String>,
    seen: std::collections::HashSet<String>,
}

impl FvState {
    fn note(&mut self, name: &str, bound: &std::collections::HashSet<String>) {
        if !bound.contains(name) && !self.seen.contains(name) {
            self.seen.insert(name.to_string());
            self.out.push(name.to_string());
        }
    }
}

/// 闭包体的自由变量（按首次使用顺序，保证确定性）
fn free_vars_block(body: &Block, params: &std::collections::HashSet<String>) -> Vec<String> {
    let mut st = FvState { out: Vec::new(), seen: std::collections::HashSet::new() };
    let mut bound = params.clone();
    fv_block(body, &mut bound, &mut st);
    st.out
}

fn fv_block(block: &Block, bound: &mut std::collections::HashSet<String>, st: &mut FvState) {
    let mut bound = bound.clone(); // 块级作用域
    for s in &block.stmts {
        fv_stmt(s, &mut bound, st);
    }
    if let Some(t) = &block.tail {
        fv_expr(t, &mut bound, st);
    }
}

fn fv_stmt(s: &Stmt, bound: &mut std::collections::HashSet<String>, st: &mut FvState) {
    match s {
        Stmt::Let { name, value, .. } => {
            fv_expr(value, bound, st);
            bound.insert(name.clone());
        }
        Stmt::LetDestruct { names, value } => {
            fv_expr(value, bound, st);
            for n in names {
                bound.insert(n.clone());
            }
        }
        Stmt::Assign { value, .. } => fv_expr(value, bound, st),
        Stmt::If(i) => fv_if(i, bound, st),
        Stmt::While { cond, body } => {
            fv_expr(cond, bound, st);
            fv_block(body, bound, st);
        }
        Stmt::For { var, iter, body } => {
            fv_expr(iter, bound, st);
            let mut b2 = bound.clone();
            b2.insert(var.clone());
            fv_block(body, &mut b2, st);
        }
        Stmt::Return(Some(e)) => fv_expr(e, bound, st),
        Stmt::Return(None) => {}
        Stmt::Expr(e) => fv_expr(e, bound, st),
        Stmt::Hole { .. } => {}
    }
}

fn fv_if(i: &IfStmt, bound: &mut std::collections::HashSet<String>, st: &mut FvState) {
    for (c, b) in &i.branches {
        fv_expr(c, bound, st);
        fv_block(b, bound, st);
    }
    if let Some(e) = &i.else_branch {
        fv_block(e, bound, st);
    }
}

fn fv_pattern(p: &Pattern, bound: &mut std::collections::HashSet<String>) {
    match p {
        Pattern::Binder(n) => {
            bound.insert(n.clone());
        }
        Pattern::Variant { sub, .. } => {
            for s in sub {
                fv_pattern(s, bound);
            }
        }
        _ => {}
    }
}

fn fv_expr(e: &Expr, bound: &mut std::collections::HashSet<String>, st: &mut FvState) {
    match e {
        Expr::Ident(n) => st.note(n, bound),
        Expr::Binary { left, right, .. } | Expr::Logical { left, right, .. } | Expr::Pipe { left, right } => {
            fv_expr(left, bound, st);
            fv_expr(right, bound, st);
        }
        Expr::Unary { expr, .. } => fv_expr(expr, bound, st),
        Expr::Call { callee, args } => {
            fv_expr(callee, bound, st);
            for arg in args {
                fv_expr(arg, bound, st);
            }
        }
        Expr::Index { expr, index } => {
            fv_expr(expr, bound, st);
            fv_expr(index, bound, st);
        }
        Expr::Field { expr, .. } => fv_expr(expr, bound, st),
        Expr::Group(inner) => fv_expr(inner, bound, st),
        Expr::If(i) => fv_if(i, bound, st),
        Expr::Closure { params, body, .. } => {
            let mut b2 = bound.clone();
            for p in params {
                b2.insert(p.name.clone());
            }
            fv_block(body, &mut b2, st);
        }
        Expr::Match(m) => {
            fv_expr(&m.scrutinee, bound, st);
            for arm in &m.arms {
                let mut b2 = bound.clone();
                fv_pattern(&arm.pattern, &mut b2);
                if let Some(g) = &arm.guard {
                    fv_expr(g, &mut b2, st);
                }
                match &arm.body {
                    MatchArmBody::Expr(e) => fv_expr(e, &mut b2, st),
                    MatchArmBody::Block(b) => fv_block(b, &mut b2, st),
                }
            }
        }
        Expr::Try(inner) => fv_expr(inner, bound, st),
        Expr::Range { start, end } => {
            fv_expr(start, bound, st);
            fv_expr(end, bound, st);
        }
        Expr::Record { fields } => {
            for (_, v) in fields {
                fv_expr(v, bound, st);
            }
        }
        Expr::Tuple { elems } => {
            for e in elems {
                fv_expr(e, bound, st);
            }
        }
        Expr::Int(_) | Expr::Float(_) | Expr::Bool(_) | Expr::Str(_) | Expr::Unit => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    fn compile(src: &str) -> Result<Vec<u8>, String> {
        let prog = Parser::parse_recover(src).program;
        compile_program(&prog)
    }

    #[test]
    fn minimal_main_compiles() {
        let bytes = compile("fn main() -> Unit\nend").unwrap();
        assert_eq!(&bytes[0..4], &[0x00, 0x61, 0x73, 0x6D]); // magic
        // 包含 "main" 导出
        assert!(bytes.windows(4).any(|w| w == b"main"));
    }

    #[test]
    fn compile_is_deterministic() {
        let src = "fn add(a: Int, b: Int) -> Int\n    a + b\nend\nfn main() -> Unit\n    println(add(1, 2))\nend";
        assert_eq!(compile(src).unwrap(), compile(src).unwrap());
    }

    #[test]
    fn missing_main_is_error() {
        assert!(compile("fn f() -> Int\n    1\nend").unwrap_err().contains("main"));
    }

    #[test]
    fn unsupported_constructs_report_phase() {
        // match → 7.5；range → 7.6（闭包自 7.3 起已支持，见 e2e）
        let e = compile("fn main() -> Unit\n    match 1\n        _ => println(1)\n    end\nend").unwrap_err();
        assert!(e.contains("7.5"), "{}", e);
        let e = compile("fn main() -> Unit\n    for i in 1..3\n        println(i)\n    end\nend").unwrap_err();
        assert!(e.contains("7.6"), "{}", e);
    }

    #[test]
    fn closures_compile_since_7_3() {
        // 闭包字面量 + 捕获 + 具名函数当值 + 递归闭包：都应编译通过（运行行为见 e2e）
        compile("fn main() -> Unit\n    let n = 5\n    let f = fn(x: Int) -> Int\n        x + n\n    end\n    println(f(10))\nend").unwrap();
        compile("fn double(x: Int) -> Int\n    x * 2\nend\nfn main() -> Unit\n    let f = double\n    println(f(21))\nend").unwrap();
        compile("fn main() -> Unit\n    let f = fn(n: Int) -> Int\n        if n <= 1\n            1\n        else\n            n * f(n - 1)\n        end\n    end\n    println(f(5))\nend").unwrap();
    }

    #[test]
    fn arity_checked_at_compile_time() {
        let e = compile("fn add(a: Int, b: Int) -> Int\n    a + b\nend\nfn main() -> Unit\n    println(add(1))\nend").unwrap_err();
        assert!(e.contains("期望 2 个参数"), "{}", e);
    }

    #[test]
    fn undefined_variable_is_compile_error() {
        let e = compile("fn main() -> Unit\n    println(x)\nend").unwrap_err();
        assert!(e.contains("未定义变量 'x'"), "{}", e);
    }
}

/// 端到端：编译 → Node 实例化运行 → 比对 stdout（node 不在 PATH 时跳过并提示）
#[cfg(test)]
mod e2e {
    use crate::parser::Parser;
    use crate::wasm_codegen::compile_program;
    use std::process::Command;

    fn node_available() -> bool {
        Command::new("node")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// 编译并运行，返回 stdout；node 缺失返回 None（测试跳过）
    fn run_wasm(src: &str, tag: &str) -> Option<String> {
        if !node_available() {
            eprintln!("[wasm_codegen e2e] node 不在 PATH，跳过（{}）", tag);
            return None;
        }
        let prog = Parser::parse_recover(src).program;
        let bytes = compile_program(&prog).unwrap_or_else(|e| panic!("[{}] 编译失败: {}", tag, e));
        let path = std::env::temp_dir().join(format!("lom_wasm_e2e_{}.wasm", tag));
        std::fs::write(&path, &bytes).unwrap();
        let out = Command::new("node")
            .arg("eval/runner/run_wasm.mjs")
            .arg(&path)
            .output()
            .expect("node 运行失败");
        let _ = std::fs::remove_file(&path);
        assert!(out.status.success(), "[{}] wasm trap: {}", tag, String::from_utf8_lossy(&out.stderr));
        Some(String::from_utf8_lossy(&out.stdout).into_owned())
    }

    fn check(src: &str, tag: &str, expected: &str) {
        if let Some(out) = run_wasm(src, tag) {
            assert_eq!(out, expected, "[{}] 输出不符", tag);
        }
    }

    #[test]
    fn e2e_arithmetic() {
        check(
            "fn main() -> Unit\n    println(1 + 2 * 3)\n    println((1 + 2) * 3)\n    println(10 - 3 - 2)\n    println(20 / 4 / 2)\n    println(17 % 5)\n    println(-5)\nend",
            "arith",
            "7\n9\n5\n2\n2\n-5\n",
        );
    }

    #[test]
    fn e2e_float_and_promotion() {
        check(
            "fn main() -> Unit\n    println(3.14 * 2.0)\n    println(1.5 + 2.5)\n    println(3 + 0.14)\n    println(10 / 4)\n    println(10.0 / 4)\n    println(7.5 % 2.0)\nend",
            "float",
            "6.28\n4.0\n3.14\n2\n2.5\n1.5\n",
        );
    }

    #[test]
    fn e2e_bool_compare() {
        check(
            "fn main() -> Unit\n    println(3 == 3)\n    println(3 != 4)\n    println(2 < 3)\n    println(5 >= 5)\n    println(1 == 1.0)\n    println(True and False)\n    println(True or False)\n    println(!True)\nend",
            "bool",
            "true\ntrue\ntrue\ntrue\nfalse\nfalse\ntrue\nfalse\n",
        );
    }

    #[test]
    fn e2e_string_literal_and_eq() {
        check(
            "fn classify(n: Int) -> String\n    if n < 0\n        \"negative\"\n    elif n == 0\n        \"zero\"\n    else\n        \"positive\"\n    end\nend\nfn main() -> Unit\n    println(classify(-5))\n    println(classify(0))\n    println(classify(42))\n    println(\"ab\" == \"ab\")\n    println(\"ab\" == \"cd\")\nend",
            "str",
            "negative\nzero\npositive\ntrue\nfalse\n",
        );
    }

    #[test]
    fn e2e_while_fib() {
        check(
            "fn fib(n: Int) -> Int\n    if n == 0\n        0\n    elif n == 1\n        1\n    else\n        let mut a = 0\n        let mut b = 1\n        let mut i = 2\n        while i <= n\n            let mut tmp = a + b\n            a = b\n            b = tmp\n            i = i + 1\n        end\n        b\n    end\nend\nfn main() -> Unit\n    println(fib(0))\n    println(fib(10))\n    println(fib(20))\nend",
            "fib",
            "0\n55\n6765\n",
        );
    }

    #[test]
    fn e2e_for_int_iteration() {
        check(
            "fn main() -> Unit\n    let mut s = 0\n    for i in 5\n        s += i\n    end\n    println(s)\nend",
            "for",
            "10\n",
        );
    }

    /// 回归：7.2 曾漏计 if 的 label 深度，if 内的 return 穿透失败（eval 任务 020 抓到）
    #[test]
    fn e2e_early_return_inside_if() {
        check(
            "fn first_even(a: Int, b: Int, c: Int) -> Int\n    if a % 2 == 0\n        return a\n    end\n    if b % 2 == 0\n        return b\n    end\n    if c % 2 == 0\n        return c\n    end\n    -1\nend\nfn main() -> Unit\n    println(first_even(1, 3, 5))\n    println(first_even(1, 4, 7))\n    println(first_even(8, 3, 5))\nend",
            "ret_if",
            "-1\n4\n8\n",
        );
    }

    #[test]
    fn e2e_recursion_and_mutual() {
        check(
            "fn is_odd(n: Int) -> Bool\n    if n == 0\n        False\n    else\n        is_even(n - 1)\n    end\nend\nfn is_even(n: Int) -> Bool\n    if n == 0\n        True\n    else\n        is_odd(n - 1)\n    end\nend\nfn fact(n: Int) -> Int\n    if n <= 1\n        1\n    else\n        n * fact(n - 1)\n    end\nend\nfn main() -> Unit\n    println(is_odd(7))\n    println(fact(10))\nend",
            "rec",
            "true\n3628800\n",
        );
    }

    #[test]
    fn e2e_shadowing_and_block_scope() {
        check(
            "fn main() -> Unit\n    let x = 1\n    if True\n        let x = 2\n        println(x)\n    end\n    println(x)\n    let mut y = 10\n    y = y + 5\n    println(y)\nend",
            "scope",
            "2\n1\n15\n",
        );
    }

    // ===== Phase 7.3: 闭包 =====

    #[test]
    fn e2e_closure_capture() {
        // make_adder：捕获参数；值拷贝语义
        check(
            "fn make_adder(n: Int) -> Fn\n    fn(x: Int) -> Int\n        x + n\n    end\nend\nfn main() -> Unit\n    let add5 = make_adder(5)\n    println(add5(10))\n    println(add5(20))\nend",
            "clo_cap",
            "15\n25\n",
        );
    }

    #[test]
    fn e2e_closure_hof() {
        // 闭包作为参数 + apply_twice/compose 模式
        check(
            "fn apply_twice(f: Fn, x: Int) -> Int\n    f(f(x))\nend\nfn compose(f: Fn, g: Fn, x: Int) -> Int\n    f(g(x))\nend\nfn main() -> Unit\n    let inc = fn(n: Int) -> Int\n        n + 1\n    end\n    let double = fn(n: Int) -> Int\n        n * 2\n    end\n    println(apply_twice(inc, 5))\n    println(compose(double, inc, 5))\n    println(compose(inc, double, 5))\nend",
            "clo_hof",
            "7\n12\n11\n",
        );
    }

    #[test]
    fn e2e_named_fn_as_value() {
        // v0.4.2 语义：具名函数当值（shim 零捕获闭包）
        check(
            "fn double(x: Int) -> Int\n    x * 2\nend\nfn apply(f: Fn, x: Int) -> Int\n    f(x)\nend\nfn main() -> Unit\n    let f = double\n    println(f(21))\n    println(apply(double, 5))\nend",
            "fn_val",
            "42\n10\n",
        );
    }

    #[test]
    fn e2e_recursive_closure() {
        // 递归闭包（预绑定 + env 补丁）
        check(
            "fn main() -> Unit\n    let f = fn(n: Int) -> Int\n        if n <= 1\n            1\n        else\n            n * f(n - 1)\n        end\n    end\n    println(f(5))\nend",
            "clo_rec",
            "120\n",
        );
    }

    #[test]
    fn e2e_closure_returning_closure() {
        // 嵌套闭包：内层捕获外层的参数（Capture 来源链）
        check(
            "fn adder(a: Int) -> Fn\n    fn(b: Int) -> Fn\n        fn(c: Int) -> Int\n            a + b + c\n        end\n    end\nend\nfn main() -> Unit\n    let f = adder(1)\n    let g = f(10)\n    println(g(100))\n    println(adder(1)(10)(100))\nend",
            "clo_nest",
            "111\n111\n",
        );
    }

    #[test]
    fn e2e_closure_print_display() {
        // 打印闭包值：对齐解释器 to_display 的 "<闭包>"
        check(
            "fn main() -> Unit\n    let f = fn(x: Int) -> Int\n        x\n    end\n    println(f)\nend",
            "clo_show",
            "<闭包>\n",
        );
    }
}
