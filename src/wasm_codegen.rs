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
const IMP_FTOA: u32 = 5; // (f64 v, i32 buf) -> i32 —— f64 格式化写入 buf，返回长度（7.4）
/// 导入总数（funcidx 换算基座）
const N_IMPORTS: u32 = 6;

// ===== 运行时 helper 函数索引 =====
const RT_BOX_F64: u32 = 6; // (f64) -> i64      动态装盒（bump alloc 8 字节）
const RT_UNBOX_F64: u32 = 7; // (i64) -> f64    拆盒
const RT_PROMOTE_F64: u32 = 8; // (i64) -> f64  Int→convert / F64→unbox / 其他 trap
const RT_STR_EQ: u32 = 9; // (i64, i64) -> i32 字符串按字节相等
const RT_ADD: u32 = 10; // (i64, i64) -> i64
const RT_SUB: u32 = 11;
const RT_MUL: u32 = 12;
const RT_DIV: u32 = 13;
const RT_MOD: u32 = 14;
const RT_LT: u32 = 15; // (i64, i64) -> i64（Bool tagged）
const RT_GT: u32 = 16;
const RT_LE: u32 = 17;
const RT_GE: u32 = 18;
const RT_EQ: u32 = 19;
const RT_NE: u32 = 20;
const RT_NEG: u32 = 21; // (i64) -> i64
const RT_NOT: u32 = 22; // (i64) -> i64
const RT_TRUTHY: u32 = 23; // (i64) -> i32      非 Bool 时 trap
const RT_PRINT: u32 = 24; // (i64 v, i64 newline) -> ()
const RT_ALLOC: u32 = 25; // (i32 size) -> i32  bump allocator（7.3 起：闭包 env/对象分配）
// ===== 7.4：字符串 / stdlib helper =====
const RT_STR_CONCAT: u32 = 26; // (i64, i64) -> i64     两字符串拼接（新堆对象）
const RT_DISPLAY: u32 = 27; // (i64) -> i64            to_display → tagged Str
const RT_ITOA: u32 = 28; // (i64 tagged Int) -> i64    十进制格式化
const RT_STR_LEN: u32 = 29; // (i64 Str) -> i64 Int    字符数（数非续字节，对齐 chars().count()）
const RT_STOI: u32 = 30; // (i64 Str) -> i64           解析整数；失败返回 Unit（对齐解释器）
const RT_TRIM: u32 = 31; // (i64 Str) -> i64           ASCII 空白（注意：解释器是 Unicode 空白，差异已记录）
const RT_UPPER: u32 = 32; // (i64 Str) -> i64          ASCII 大小写（Unicode 差异已记录）
const RT_LOWER: u32 = 33; // (i64 Str) -> i64
const RT_CONTAINS: u32 = 34; // (i64, i64) -> i64 Bool 朴素子串查找（UTF-8 安全：模式是合法 UTF-8）
const RT_STARTS: u32 = 35; // (i64, i64) -> i64 Bool
const RT_ENDS: u32 = 36; // (i64, i64) -> i64 Bool
const RT_REPLACE: u32 = 37; // (i64, i64, i64) -> i64  全量替换（两遍扫描）
const RT_STR_CMP: u32 = 38; // (i64, i64) -> i32       字节序比较（对齐 Rust str Ord）→ -1/0/1
const RT_STR_CHAR_AT: u32 = 39; // (i64 Str, i64 byte_off) -> i64 Str  按 UTF-8 头字节取单字符
const RT_FTOA_STR: u32 = 40; // (f64) -> i64          经 lom_ftoa 导入格式化 → 堆字符串
// ===== 7.5：枚举 / match / ? =====
const RT_ENUM_PRINT: u32 = 41; // (i64 v, i64 newline) -> ()  枚举打印（含参数递归）；体在 finalize 填（需变体名表）
const RT_ENUM_STR: u32 = 42; // (i64) -> i64                 枚举 → Str（display 用）
const RT_ENUM_EQ: u32 = 43; // (i64, i64) -> i32              枚举递归相等
/// 第一个用户函数的 funcidx
const FIRST_USER_FN: u32 = 44;

/// 闭包值 tag：堆对象布局 [table_idx: i32][env: i32]（env 指向 [n: i32][v0..vn: i64]）
const TAG_CLOSURE: i64 = 5;
/// 枚举值 tag：堆对象布局 [variant_idx: i32][n_args: i32][args: i64×n]（7.5）
const TAG_ENUM: i64 = 6;

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
    fn i32_store8(&mut self, offset: u32) -> &mut Self {
        self.op(op::I32_STORE8);
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
    /// 枚举变体：名 → (全局索引, 参数个数)。内建 Ok=0/Err=1/Some=2/None=3，用户从 4 起
    variant_idx: std::collections::HashMap<String, (u32, usize)>,
    /// 导入别名 → 真实内建名（from string import { len as slen }）
    import_aliases: std::collections::HashMap<String, String>,
    /// 已导入可用的内建函数（别名后的可用名）
    available_builtins: std::collections::HashSet<String>,
    /// 静态数据镜像（offset 0 起）；字节 0 固定是 '\n'（rt_print 换行用）
    data: Vec<u8>,
    /// "(" / ")" / ", " 三个静态串的偏移（枚举打印用）
    paren_offs: (u32, u32, u32),
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
                    let idx = cg.variant_idx.len() as u32;
                    cg.variant_idx.entry(v.name.clone()).or_insert((idx, v.fields.len()));
                }
            }
            Item::Import(imp) => {
                // io：println/print 在 prelude 已可用，显式导入等价 no-op
                // 7.4 起支持 string / math；list/json/map 待 7.6，file/env 待 7.8
                const STRING_BUILTINS: &[&str] = &[
                    "len", "int_to_string", "string_to_int", "trim", "upper", "lower",
                    "contains", "replace", "starts_with", "ends_with",
                ];
                // 已导出但 WASM 侧未就绪（依赖 List/Json 等 7.6 类型）
                const DEFERRED: &[&str] = &["split"];
                const MATH_BUILTINS: &[&str] = &["sqrt", "abs", "min", "max"];
                let exports: Option<&[&str]> = match imp.module.as_str() {
                    "io" => Some(&["println", "print"]),
                    "string" => Some(STRING_BUILTINS),
                    "math" => Some(MATH_BUILTINS),
                    "list" | "json" | "map" => {
                        return Err(format!("WASM 后端暂不支持导入模块 '{}'（将在 7.6 支持）", imp.module))
                    }
                    "file" | "env" => {
                        return Err(format!("WASM 后端暂不支持导入模块 '{}'（将在 7.8 支持）", imp.module))
                    }
                    _ => None,
                };
                match exports {
                    Some(list) => {
                        for it in &imp.items {
                            if DEFERRED.contains(&it.name.as_str()) {
                                return Err(format!(
                                    "WASM 后端暂不支持内置函数 '{}'（将在 7.6 支持）",
                                    it.name
                                ));
                            }
                            if !list.contains(&it.name.as_str()) {
                                return Err(format!(
                                    "WASM 编译：模块 '{}' 不导出符号 '{}'",
                                    imp.module, it.name
                                ));
                            }
                            if it.alias != it.name {
                                cg.import_aliases.insert(it.alias.clone(), it.name.clone());
                            }
                            cg.available_builtins.insert(it.alias.clone());
                        }
                    }
                    None => {
                        return Err(format!("WASM 编译：未知模块 '{}'", imp.module));
                    }
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
            let slot = (cg.fn_idx[&f.name] - N_IMPORTS) as usize; // funcidx - 导入数 = funcs 下标
            cg.m.funcs[slot] = func;
        }
    }

    // 收尾：funcref 表 + 变体名表 + 数据段 + 全局（hp 堆指针 + ftoa 64 字节 scratch 缓冲）
    if !cg.table_entries.is_empty() {
        let n = cg.table_entries.len() as u32;
        cg.m.table = Some((n, Some(n)));
        cg.m.elems = cg.table_entries.clone();
    }
    // 变体名表（7.5）：按 idx 顺序的 [name_off: i32] 数组；枚举打印/串化 helper 的体在这里填
    let variant_table_off = cg.build_variant_table();
    {
        let (po, pc, co) = cg.paren_offs;
        let pi = (RT_ENUM_PRINT - N_IMPORTS) as usize;
        cg.m.funcs[pi].body = build_enum_print(variant_table_off, po, pc, co);
        let si = (RT_ENUM_STR - N_IMPORTS) as usize;
        cg.m.funcs[si].body = build_enum_str(variant_table_off, po, pc, co);
    }
    let data_end = cg.data.len() as u32;
    let heap_base = data_end + 64; // 64 字节 ftoa scratch（global 1）
    let pages = (heap_base / 65536) + 1;
    cg.m.memory_min_pages = Some(pages.max(1));
    cg.m.globals.push(Global { ty: ValType::I32, mutable: true, init: heap_base as i64 }); // global 0 = hp
    cg.m.globals.push(Global { ty: ValType::I32, mutable: false, init: data_end as i64 }); // global 1 = ftoa buf
    cg.m.exports.push(("memory".into(), ExportKind::Memory, 0));
    let main_idx = cg.fn_idx["main"];
    cg.m.exports.push(("main".into(), ExportKind::Func, main_idx));
    cg.m.data.push(DataSegment { offset: 0, bytes: std::mem::take(&mut cg.data) });

    Ok(cg.m.encode())
}

impl Codegen {
    fn new() -> Self {
        let mut m = Module::new();
        // 静态数据开头：offset 0 = '\n'（rt_print 换行）；随后是 "<闭包>"（rt_print tag5）
        // 与 rt_display 用的 "true"/"false"/"()" 字面量
        let mut data: Vec<u8> = vec![b'\n'];
        let mut str_off: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        fn intern_static(data: &mut Vec<u8>, str_off: &mut std::collections::HashMap<String, u32>, s: &str) -> u32 {
            let off = data.len() as u32;
            data.extend((s.len() as u32).to_le_bytes());
            data.extend(s.as_bytes());
            str_off.insert(s.to_string(), off);
            off
        }
        let closure_off = intern_static(&mut data, &mut str_off, "<闭包>");
        let true_off = intern_static(&mut data, &mut str_off, "true");
        let false_off = intern_static(&mut data, &mut str_off, "false");
        let unit_off = intern_static(&mut data, &mut str_off, "()");
        // 7.5 枚举打印用："(", ")", ", "
        let paren_open_off = intern_static(&mut data, &mut str_off, "(");
        let paren_close_off = intern_static(&mut data, &mut str_off, ")");
        let comma_off = intern_static(&mut data, &mut str_off, ", ");
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
        let ty_ftoa = m.add_type(FuncType { params: vec![ValType::F64, ValType::I32], results: vec![ValType::I32] });
        let ty_iii_i64 = m.add_type(FuncType { params: vec![ValType::I64, ValType::I64, ValType::I64], results: vec![ValType::I64] });

        // 导入（funcidx 0-5）
        for (name, ty) in [
            ("lom_print_int", ty_ii_unit),
            ("lom_print_float", ty_fi_unit),
            ("lom_print_bool", ty_ii_unit),
            ("lom_print_unit", ty_i_unit),
            ("lom_print", ty_pp_unit),
            ("lom_ftoa", ty_ftoa),
        ] {
            m.imports.push(Import { module: "env".into(), name: name.into(), type_idx: ty });
        }

        // 运行时 helper（funcidx 6-40；新增 helper 必须同步 FIRST_USER_FN 与上面的 RT_* 常量）
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
            (ty_ii_i64, vec![], build_cmp(CmpKind::Lt)),
            (ty_ii_i64, vec![], build_cmp(CmpKind::Gt)),
            (ty_ii_i64, vec![], build_cmp(CmpKind::Le)),
            (ty_ii_i64, vec![], build_cmp(CmpKind::Ge)),
            (ty_ii_i64, vec![], build_eq()),
            (ty_ii_i64, vec![], build_ne()),
            (ty_i64_i64, vec![], build_neg()),
            (ty_i64_i64, vec![], build_not()),
            (ty_i64_i32, vec![], build_truthy()),
            (ty_ii_unit, vec![ValType::I32], build_print(closure_off)),
            (ty_i32_i32, vec![ValType::I32], build_alloc()),
            // ===== 7.4：字符串 / stdlib =====
            (ty_ii_i64, vec![ValType::I32; 6], build_str_concat()),
            (ty_i64_i64, vec![], build_display(true_off, false_off, unit_off)),
            (ty_i64_i64, vec![ValType::I64, ValType::I32, ValType::I32, ValType::I32, ValType::I32, ValType::I32], build_itoa()),
            (ty_i64_i64, vec![ValType::I32; 4], build_str_len()),
            (ty_i64_i64, vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32, ValType::I32, ValType::I64], build_stoi()),
            (ty_i64_i64, vec![ValType::I32; 7], build_trim()),
            (ty_i64_i64, vec![ValType::I32; 5], build_case(true)),  // upper
            (ty_i64_i64, vec![ValType::I32; 5], build_case(false)), // lower
            (ty_ii_i64, vec![ValType::I32; 6], build_contains()),
            (ty_ii_i64, vec![ValType::I32; 5], build_starts_ends(true)),
            (ty_ii_i64, vec![ValType::I32; 5], build_starts_ends(false)),
            (ty_iii_i64, vec![ValType::I32; 13], build_replace()),
            (ty_ii_i32, vec![ValType::I32; 6], build_str_cmp()),
            (ty_ii_i64, vec![ValType::I32; 5], build_str_char_at()),
            (ty_f_i64, vec![ValType::I32; 3], build_ftoa_str()),
            // ===== 7.5：枚举 / match / ? =====
            // RT_ENUM_PRINT / RT_ENUM_STR 的体在 finalize 填（需要变体名表偏移）
            (ty_ii_unit, vec![ValType::I32; 5], vec![]),
            (ty_i64_i64, vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32, ValType::I32, ValType::I32, ValType::I64], vec![]),
            (ty_ii_i32, vec![ValType::I32; 4], build_enum_eq()),
        ];
        for (ty, locals, body) in helpers {
            m.funcs.push(Function { type_idx: ty, locals, body });
        }

        // 内建变体：Ok=0 Err=1 Some=2 None=3（None 零参）
        let mut variant_idx: std::collections::HashMap<String, (u32, usize)> = std::collections::HashMap::new();
        for (i, (v, arity)) in [("Ok", 1), ("Err", 1), ("Some", 1), ("None", 0)].iter().enumerate() {
            variant_idx.insert(v.to_string(), (i as u32, *arity));
        }
        let mut available_builtins = std::collections::HashSet::new();
        for b in ["println", "print"] {
            available_builtins.insert(b.to_string());
        }
        Codegen {
            m,
            fn_idx: std::collections::HashMap::new(),
            fn_arity: std::collections::HashMap::new(),
            shim_idx: std::collections::HashMap::new(),
            table_entries: Vec::new(),
            variant_idx,
            import_aliases: std::collections::HashMap::new(),
            available_builtins,
            data,
            paren_offs: (paren_open_off, paren_close_off, comma_off),
            str_off,
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

    /// 枚举构造：参数求值 → [variant_idx: i32][n: i32][args: i64×n] 堆对象（tag 6）
    fn emit_enum_construct(&mut self, ctx: &mut FnCtx, a: &mut Asm, vidx: u32, args: &[Expr]) -> Result<(), String> {
        let mut scratches = Vec::new();
        for arg in args {
            self.compile_expr(ctx, a, arg)?;
            let s = ctx.alloc();
            a.lset(s);
            scratches.push(s);
        }
        let p = ctx.alloc();
        a.i32c(8 + 8 * args.len() as i32).call(RT_ALLOC).op(op::I64_EXTEND_I32_S).lset(p);
        a.lget(p).op(op::I32_WRAP_I64).i32c(vidx as i32).i32_store(0);
        a.lget(p).op(op::I32_WRAP_I64).i32c(args.len() as i32).i32_store(4);
        for (k, s) in scratches.iter().enumerate() {
            a.lget(p).op(op::I32_WRAP_I64).lget(*s).i64_store(8 + 8 * k as u32);
        }
        a.lget(p).tag_int().i64c(TAG_ENUM).op(op::I64_OR);
        Ok(())
    }

    /// match 表达式：$done 块带值，臂链依序测试，无匹配 trap（对齐解释器运行时错误）
    fn compile_match(&mut self, ctx: &mut FnCtx, a: &mut Asm, m: &MatchExpr) -> Result<(), String> {
        self.compile_expr(ctx, a, &m.scrutinee)?;
        let s = ctx.alloc();
        a.lset(s);
        a.block_i64();
        ctx.labels.push(Label::Block); // $done
        let done_pos = ctx.labels.len() - 1;
        for arm in &m.arms {
            ctx.scopes.push(Vec::new()); // 臂作用域（模式绑定在这里）
            self.compile_pattern_test(ctx, a, &arm.pattern, s)?;
            a.if_();
            ctx.labels.push(Label::If);
            if let Some(g) = &arm.guard {
                // guard 为假 → 穿透到下一臂（v0.4.2 语义）
                self.compile_expr(ctx, a, g)?;
                a.call(RT_TRUTHY).if_();
                ctx.labels.push(Label::If);
                self.compile_arm_body(ctx, a, &arm.body)?;
                a.br(ctx.depth(done_pos));
                ctx.labels.pop();
                a.end();
            } else {
                self.compile_arm_body(ctx, a, &arm.body)?;
                a.br(ctx.depth(done_pos));
            }
            ctx.labels.pop();
            a.end();
            ctx.scopes.pop();
        }
        a.op(op::UNREACHABLE); // 无臂匹配（解释器：运行时错误）
        a.end();
        ctx.labels.pop();
        Ok(())
    }

    fn compile_arm_body(&mut self, ctx: &mut FnCtx, a: &mut Asm, body: &MatchArmBody) -> Result<(), String> {
        match body {
            MatchArmBody::Expr(e) => self.compile_expr(ctx, a, e),
            MatchArmBody::Block(b) => self.compile_block_value(ctx, a, b),
        }
    }

    /// 模式测试：栈上留 i32 条件；绑定变量写入当前臂作用域
    fn compile_pattern_test(&mut self, ctx: &mut FnCtx, a: &mut Asm, pat: &Pattern, s: u32) -> Result<(), String> {
        match pat {
            Pattern::Wildcard => {
                a.i32c(1);
                Ok(())
            }
            Pattern::Binder(name) => {
                if let Some(&(vidx, 0)) = self.variant_idx.get(name.as_str()) {
                    // 零参变体模式（None 等）：tag6 + idx 相等（对齐解释器 nullary_variants 判定）
                    self.emit_variant_test(a, s, vidx);
                } else {
                    // 绑定：被测值绑到新 local，恒真
                    let idx = ctx.bind(name);
                    a.lget(s).lset(idx);
                    a.i32c(1);
                }
                Ok(())
            }
            Pattern::Lit(e) => {
                // 字面量模式：值相等（rt_eq；Int/Float/Bool/Str 四种子集，对齐解释器）
                match e {
                    Expr::Int(n) => {
                        a.i64c(n << 3);
                    }
                    Expr::Float(f) => {
                        let v = self.intern_f64(*f);
                        a.i64c(v);
                    }
                    Expr::Bool(bv) => {
                        a.i64c(if *bv { V_TRUE } else { V_FALSE });
                    }
                    Expr::Str(txt) => {
                        let v = self.intern_str(txt);
                        a.i64c(v);
                    }
                    _ => return Err(format!("WASM 编译：不支持的字面量模式 {:?}", e)),
                }
                a.lget(s).call(RT_EQ).untag().op(op::I32_WRAP_I64);
                Ok(())
            }
            Pattern::Variant { name, sub } => {
                let &(vidx, arity) = match self.variant_idx.get(name.as_str()) {
                    Some(v) => v,
                    None => return Err(format!("WASM 编译：未知变体 '{}'", name)),
                };
                if sub.len() != arity {
                    return Err(format!(
                        "WASM 编译：变体 '{}' 期望 {} 个子模式，得到 {} 个",
                        name, arity, sub.len()
                    ));
                }
                self.emit_variant_test(a, s, vidx);
                for (k, sp) in sub.iter().enumerate() {
                    // arg_k 装入新 local 再递归测试
                    let arg_l = ctx.alloc();
                    a.lget(s).i64c(3).op(op::I64_SHR_U).op(op::I32_WRAP_I64).i64_load(8 + 8 * k as u32).lset(arg_l);
                    self.compile_pattern_test(ctx, a, sp, arg_l)?;
                    a.op(op::I32_AND);
                }
                Ok(())
            }
        }
    }

    /// 变体测试（tag6 且 variant_idx 相等）→ i32
    fn emit_variant_test(&mut self, a: &mut Asm, s: u32, vidx: u32) {
        a.lget(s).tag().i64c(TAG_ENUM).op(op::I64_EQ);
        a.lget(s).i64c(3).op(op::I64_SHR_U).op(op::I32_WRAP_I64).i32_load(0).i32c(vidx as i32).op(op::I32_EQ);
        a.op(op::I32_AND);
    }

    /// 变体名表：按 idx 顺序把变体名 intern 进数据段，再写 [name_off: i32] 数组；返回表偏移
    fn build_variant_table(&mut self) -> u32 {
        // 按 idx 排序（内建 0-3 在前，用户变体按声明序）
        let mut by_idx: Vec<(u32, String)> = self.variant_idx.iter().map(|(n, &(i, _))| (i, n.clone())).collect();
        by_idx.sort_by_key(|(i, _)| *i);
        // 先 intern 所有名字（复用 str_off 去重），记录偏移
        let mut name_offs = Vec::with_capacity(by_idx.len());
        for (_, name) in &by_idx {
            let off = self.intern_str(name);
            name_offs.push(((off - TAG_STR) >> 3) as u32); // 去掉 tag 还原裸偏移
        }
        let table_off = self.data.len() as u32;
        for no in name_offs {
            self.data.extend(no.to_le_bytes());
        }
        table_off
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
                // 支持 for i in <Int>（0..n）与 for c in <String>（逐字符，7.4）；List 迭代在 7.6
                let it = ctx.alloc();
                let limit = ctx.alloc();
                let cnt = ctx.alloc();
                let var_idx = ctx.alloc();
                self.compile_expr(ctx, a, iter)?;
                a.lset(it);
                a.tag_is(it, TAG_INT).if_();
                {
                    // Int 迭代：0..n
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
                }
                a.else_();
                {
                    a.tag_is(it, TAG_STR).if_();
                    {
                        // String 迭代：按 UTF-8 字符（cnt = 字节偏移；步进 = 当前字符字节数）
                        a.lget(it).i64c(3).op(op::I64_SHR_U).op(op::I32_WRAP_I64).i32_load(0).op(op::I64_EXTEND_I32_S).lset(limit);
                        a.i64c(0).lset(cnt);
                        a.block();
                        ctx.labels.push(Label::Block);
                        a.loop_();
                        ctx.labels.push(Label::Loop);
                        a.lget(cnt).lget(limit).op(op::I64_GE_S);
                        let bd = ctx.break_depth();
                        a.br_if(bd);
                        a.lget(it).lget(cnt).call(RT_STR_CHAR_AT).lset(var_idx);
                        ctx.scopes.push(vec![(var.clone(), var_idx)]);
                        for s in &body.stmts {
                            self.compile_stmt(ctx, a, s)?;
                        }
                        if let Some(e) = &body.tail {
                            self.compile_expr(ctx, a, e)?;
                            a.op(op::DROP);
                        }
                        ctx.scopes.pop();
                        // cnt += 当前字符字节数（从 var 的字符串头读）
                        a.lget(var_idx).i64c(3).op(op::I64_SHR_U).op(op::I32_WRAP_I64).i32_load(0)
                            .op(op::I64_EXTEND_I32_S).lget(cnt).op(op::I64_ADD).lset(cnt);
                        a.br(ctx.depth(ctx.labels.len() - 1));
                        ctx.labels.pop();
                        a.end();
                        ctx.labels.pop();
                        a.end();
                    }
                    a.else_();
                    {
                        a.op(op::UNREACHABLE); // List 等其他可迭代类型在 7.6
                    }
                    a.end();
                }
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
                if let Some(&(vidx, arity)) = self.variant_idx.get(name) {
                    // 零参变体作值（None 等）：构造 0 参数枚举对象
                    if arity == 0 {
                        return self.emit_enum_construct(ctx, a, vidx, &[]);
                    }
                    return Err(format!("WASM 编译：变体 '{}' 需要 {} 个参数（构造写 {}(...)）", name, arity, name));
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
            Expr::Match(m) => self.compile_match(ctx, a, m),
            Expr::Try(inner) => {
                // `?`：Ok(v)/Some(v) → 解包；Err(e)/None → 整个值 br $ret（对齐解释器 EarlyReturn）
                self.compile_expr(ctx, a, inner)?;
                let t = ctx.alloc();
                a.lset(t);
                a.tag_is(t, TAG_ENUM).if_().else_().op(op::UNREACHABLE).end();
                // idx = load32(ptr)；Ok=0/Some=2 解包，Err=1/None=3 早退
                let vi = ctx.alloc();
                a.lget(t).i64c(3).op(op::I64_SHR_U).op(op::I32_WRAP_I64).i32_load(0).op(op::I64_EXTEND_I32_S).lset(vi);
                a.lget(vi).i64c(0).op(op::I64_EQ);
                a.lget(vi).i64c(2).op(op::I64_EQ);
                a.op(op::I32_OR).if_i64();
                ctx.labels.push(Label::If); // if 也是 label，br 深度要计入（7.2 的坑）
                {
                    a.lget(t).i64c(3).op(op::I64_SHR_U).op(op::I32_WRAP_I64).i64_load(8);
                }
                a.else_();
                {
                    a.lget(t);
                    a.br(ctx.depth(0)); // br $ret：整个 Err/None 值
                }
                a.end();
                ctx.labels.pop();
                Ok(())
            }
            Expr::Record { .. } => unsupported("记录", "Phase 7.6"),
            Expr::Tuple { .. } => unsupported("元组", "Phase 7.6"),
            Expr::Index { .. } => unsupported("索引", "Phase 7.6"),
            Expr::Field { .. } => unsupported("字段访问", "Phase 7.6"),
            Expr::Range { .. } => unsupported("range 表达式 a..b", "Phase 7.6"),
        }
    }

    fn compile_call(&mut self, ctx: &mut FnCtx, a: &mut Asm, callee: &Expr, args: &[Expr]) -> Result<(), String> {
        if let Expr::Ident(name) = callee {
            let orig: &str = name;
            // 导入别名解析（log → println 等）；变体/用户函数/闭包判断用 orig，内建分派用真名 real
            // real 用 owned String 避免借用 self 卡住后续可变调用
            let real: String = self.import_aliases.get(orig).cloned().unwrap_or_else(|| orig.to_string());
            // 顺序对齐解释器 eval_call：变体（未被遮蔽）→ 内建 → 用户函数 → 闭包变量
            // 枚举变体构造（未被局部变量遮蔽时）
            if let Some(&(vidx, arity)) = self.variant_idx.get(orig) {
                if ctx.lookup(orig).is_none() && ctx.capture_slot(orig).is_none() {
                    if arity != args.len() {
                        return Err(format!("WASM 编译：变体 '{}' 期望 {} 个参数，得到 {} 个", orig, arity, args.len()));
                    }
                    return self.emit_enum_construct(ctx, a, vidx, args);
                }
            }
            // println / print（prelude）
            if real == "println" || real == "print" {
                if args.len() != 1 {
                    return Err(format!("WASM 编译：{} 期望 1 个参数，得到 {} 个", orig, args.len()));
                }
                self.compile_expr(ctx, a, &args[0])?;
                a.i64c(if real == "println" { 1 } else { 0 });
                a.call(RT_PRINT);
                a.i64c(V_UNIT); // println 返回 Unit
                return Ok(());
            }
            // 内建函数（string/math，7.4）：可用性看导入时的名字（orig），分派用真名（real）
            if self.available_builtins.contains(orig) {
                if self.compile_builtin(ctx, a, real.as_str(), args)? {
                    return Ok(());
                }
                return Err(format!(
                    "WASM 后端暂不支持内置函数 '{}'（split/list/json/map/file/env 系列在 7.6/7.8 提供）",
                    real
                ));
            }
            // 用户函数（具名直调；注意解释器里具名函数优先于同名闭包变量，保持一致）
            if let Some(&idx) = self.fn_idx.get(orig) {
                self.check_arity(orig, args.len())?;
                for arg in args {
                    self.compile_expr(ctx, a, arg)?;
                }
                a.call(idx);
                return Ok(());
            }
            // 闭包变量调用
            if ctx.lookup(orig).is_some() || ctx.capture_slot(orig).is_some() {
                return self.emit_closure_call(ctx, a, callee, args);
            }
            return Err(format!(
                "WASM 编译：未定义函数或内建未导入 '{}'（string/math 内建需 from ... import 显式导入）",
                orig
            ));
        }
        // 任意表达式 callee（如 make_adder(5)(10)）——闭包调用
        self.emit_closure_call(ctx, a, callee, args)
    }

    /// 7.4 内建函数编译（调用点已确认名字已导入可用）。返回 Ok(true)=已处理。
    /// 类型检查策略：tag 不符即 trap（对齐解释器运行时错误；结构化错误消息在 7.9 对齐）。
    fn compile_builtin(&mut self, ctx: &mut FnCtx, a: &mut Asm, name: &str, args: &[Expr]) -> Result<bool, String> {
        // 单字符串参数辅助：编译 + tag 检查 + call helper
        let str_unary = |cg: &mut Self, ctx: &mut FnCtx, a: &mut Asm, args: &[Expr], helper: u32, what: &str| -> Result<(), String> {
            if args.len() != 1 {
                return Err(format!("WASM 编译：{} 期望 1 个参数，得到 {} 个", what, args.len()));
            }
            cg.compile_expr(ctx, a, &args[0])?;
            let s = ctx.alloc();
            a.lset(s);
            a.tag_is(s, TAG_STR).if_().else_().op(op::UNREACHABLE).end();
            a.lget(s).call(helper);
            Ok(())
        };
        match name {
            "len" => str_unary(self, ctx, a, args, RT_STR_LEN, "len")?,
            "int_to_string" => {
                if args.len() != 1 {
                    return Err(format!("WASM 编译：int_to_string 期望 1 个参数，得到 {} 个", args.len()));
                }
                self.compile_expr(ctx, a, &args[0])?;
                let s = ctx.alloc();
                a.lset(s);
                a.tag_is(s, TAG_INT).if_().else_().op(op::UNREACHABLE).end();
                a.lget(s).call(RT_ITOA);
            }
            "string_to_int" => str_unary(self, ctx, a, args, RT_STOI, "string_to_int")?,
            "trim" => str_unary(self, ctx, a, args, RT_TRIM, "trim")?,
            "upper" => str_unary(self, ctx, a, args, RT_UPPER, "upper")?,
            "lower" => str_unary(self, ctx, a, args, RT_LOWER, "lower")?,
            "contains" | "starts_with" | "ends_with" | "replace" => {
                let (want, helper) = match name {
                    "contains" => (2, RT_CONTAINS),
                    "starts_with" => (2, RT_STARTS),
                    "ends_with" => (2, RT_ENDS),
                    _ => (3, RT_REPLACE),
                };
                if args.len() != want {
                    return Err(format!("WASM 编译：{} 期望 {} 个参数，得到 {} 个", name, want, args.len()));
                }
                // 每个参数只求值一次：先进 scratch（带 tag 检查），再按序压栈
                let mut scratches = Vec::new();
                for arg in args {
                    self.compile_expr(ctx, a, arg)?;
                    let s = ctx.alloc();
                    a.lset(s);
                    a.tag_is(s, TAG_STR).if_().else_().op(op::UNREACHABLE).end();
                    scratches.push(s);
                }
                for s in scratches {
                    a.lget(s);
                }
                a.call(helper);
            }
            "sqrt" => {
                // Int/Float → Float（promote 内部对非数值 trap）
                if args.len() != 1 {
                    return Err(format!("WASM 编译：sqrt 期望 1 个参数，得到 {} 个", args.len()));
                }
                self.compile_expr(ctx, a, &args[0])?;
                a.call(RT_PROMOTE_F64).op(op::F64_SQRT).call(RT_BOX_F64);
            }
            "abs" => {
                // Int→Int（无分支绝对值：(v^(v>>63))-(v>>63)）；Float→Float
                if args.len() != 1 {
                    return Err(format!("WASM 编译：abs 期望 1 个参数，得到 {} 个", args.len()));
                }
                self.compile_expr(ctx, a, &args[0])?;
                let s = ctx.alloc();
                a.lset(s);
                a.tag_is(s, TAG_INT).if_i64();
                {
                    a.lget(s).untag(); // v
                    a.lget(s).untag().i64c(63).op(op::I64_SHR_S); // t = v>>63
                    a.op(op::I64_XOR); // v^t
                    a.lget(s).untag().i64c(63).op(op::I64_SHR_S); // t
                    a.op(op::I64_SUB).tag_int();
                }
                a.else_();
                {
                    a.tag_is(s, TAG_F64).if_i64();
                    {
                        a.lget(s).call(RT_UNBOX_F64).op(op::F64_ABS).call(RT_BOX_F64);
                    }
                    a.else_().op(op::UNREACHABLE).end();
                }
                a.end();
            }
            "min" | "max" => {
                // 同类型对（Int,Int）/（Float,Float)；混合 trap（对齐解释器）
                if args.len() != 2 {
                    return Err(format!("WASM 编译：{} 期望 2 个参数，得到 {} 个", name, args.len()));
                }
                let (i64cmp, f64op) = if name == "min" {
                    (op::I64_LT_S, op::F64_MIN)
                } else {
                    (op::I64_GT_S, op::F64_MAX)
                };
                let sa = ctx.alloc();
                let sb = ctx.alloc();
                self.compile_expr(ctx, a, &args[0])?;
                a.lset(sa);
                self.compile_expr(ctx, a, &args[1])?;
                a.lset(sb);
                a.tag_is(sa, TAG_INT).if_i64();
                {
                    a.tag_is(sb, TAG_INT).if_i64();
                    {
                        // select(a, b, a<b) —— min；max 用 a>b
                        a.lget(sa).untag().lget(sb).untag();
                        a.lget(sa).untag().lget(sb).untag().op(i64cmp);
                        a.op(op::SELECT).tag_int();
                    }
                    a.else_().op(op::UNREACHABLE).end();
                }
                a.else_();
                {
                    a.tag_is(sa, TAG_F64).if_i64();
                    {
                        a.tag_is(sb, TAG_F64).if_i64();
                        {
                            a.lget(sa).call(RT_UNBOX_F64).lget(sb).call(RT_UNBOX_F64).op(f64op).call(RT_BOX_F64);
                        }
                        a.else_().op(op::UNREACHABLE).end();
                    }
                    a.else_().op(op::UNREACHABLE).end();
                    a.end();
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
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
/// Add 特例（v0.4.1 拼接提升）：任一侧是 Str → 两侧 to_display 后拼接
fn build_arith(kind: ArithKind) -> Vec<u8> {
    let mut a = Asm::new();
    if matches!(kind, ArithKind::Add) {
        // tag4(l) or tag4(r) → concat(display(l), display(r))
        a.lget(0).tag().i64c(TAG_STR).op(op::I64_EQ);
        a.lget(1).tag().i64c(TAG_STR).op(op::I64_EQ);
        a.op(op::I32_OR).if_i64();
        {
            a.lget(0).call(RT_DISPLAY).lget(1).call(RT_DISPLAY).call(RT_STR_CONCAT);
        }
        a.else_();
        build_arith_numeric(&mut a, kind);
        a.end();
        return a.b;
    }
    build_arith_numeric(&mut a, kind);
    a.b
}

/// 算术的数值路径（Int/Int → 整数；含 F64 → promote；其他 trap）
fn build_arith_numeric(a: &mut Asm, kind: ArithKind) {
    a.tag_is(0, TAG_INT).if_i64();
    {
        a.tag_is(1, TAG_INT).if_i64();
        {
            a.lget(0).untag().lget(1).untag().op(kind.int_op()).tag_int();
        }
        a.else_();
        {
            a.tag_is(1, TAG_F64).if_i64();
            emit_float_path(a, kind);
            a.else_().op(op::UNREACHABLE).end();
        }
        a.end();
    }
    a.else_();
    {
        a.tag_is(0, TAG_F64).if_i64();
        {
            a.tag_is(1, TAG_INT).if_i64();
            emit_float_path(a, kind);
            a.else_();
            {
                a.tag_is(1, TAG_F64).if_i64();
                emit_float_path(a, kind);
                a.else_().op(op::UNREACHABLE).end();
            }
            a.end();
        }
        a.else_().op(op::UNREACHABLE).end();
    }
    a.end();
}

/// 比较种类（7.4 重构：build_cmp 按种类选 i64/f64/i32 三套 opcode）
#[derive(Clone, Copy)]
enum CmpKind {
    Lt,
    Gt,
    Le,
    Ge,
}

impl CmpKind {
    fn i64_op(self) -> u8 {
        match self {
            CmpKind::Lt => op::I64_LT_S,
            CmpKind::Gt => op::I64_GT_S,
            CmpKind::Le => op::I64_LE_S,
            CmpKind::Ge => op::I64_GE_S,
        }
    }
    fn f64_op(self) -> u8 {
        match self {
            CmpKind::Lt => op::F64_LT,
            CmpKind::Gt => op::F64_GT,
            CmpKind::Le => op::F64_LE,
            CmpKind::Ge => op::F64_GE,
        }
    }
    /// 对 rt_str_cmp 的 -1/0/1 结果做判断（i32 与 0 比较）
    fn i32_op(self) -> u8 {
        match self {
            CmpKind::Lt => op::I32_LT_S,
            CmpKind::Gt => op::I32_GT_S,
            CmpKind::Le => op::I32_LE_S,
            CmpKind::Ge => op::I32_GE_S,
        }
    }
}

/// rt_lt/gt/le/ge: (i64 l, i64 r) -> i64(Bool)
/// Int/Int、F64/F64、Bool/Bool、Str/Str（7.4 起，字节序对齐 Rust str Ord）四组同类型对
/// （对齐解释器 eval_compare：混合类型报错→trap）
fn build_cmp(kind: CmpKind) -> Vec<u8> {
    let mut a = Asm::new();
    a.tag_is(0, TAG_INT).if_i64();
    {
        a.tag_is(1, TAG_INT).if_i64();
        {
            a.lget(0).untag().lget(1).untag().op(kind.i64_op()).bool_tag();
        }
        a.else_().op(op::UNREACHABLE).end();
    }
    a.else_();
    {
        a.tag_is(0, TAG_F64).if_i64();
        {
            a.tag_is(1, TAG_F64).if_i64();
            {
                a.lget(0).call(RT_UNBOX_F64).lget(1).call(RT_UNBOX_F64).op(kind.f64_op()).bool_tag();
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
                    a.lget(0).lget(1).op(kind.i64_op()).bool_tag();
                }
                a.else_().op(op::UNREACHABLE).end();
            }
            a.else_();
            {
                // Str/Str：字节序比较（7.4）
                a.tag_is(0, TAG_STR).if_i64();
                {
                    a.tag_is(1, TAG_STR).if_i64();
                    {
                        a.lget(0).lget(1).call(RT_STR_CMP).i32c(0).op(kind.i32_op()).bool_tag();
                    }
                    a.else_().op(op::UNREACHABLE).end();
                }
                a.else_().op(op::UNREACHABLE).end();
            }
            a.end();
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
                // 枚举：变体 idx + 参数递归相等（7.5，对齐解释器 values_eq）
                a.tag_is(0, TAG_ENUM).if_i64();
                {
                    a.lget(0).lget(1).call(RT_ENUM_EQ).bool_tag();
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
        a.tag_is(0, TAG_ENUM).if_();
        {
            a.lget(0).lget(1).call(RT_ENUM_PRINT).br(1);
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

// ===== 7.4：字符串 / stdlib helper 函数体 =====

/// 发射：tagged Str 参数 → 堆指针（i32）存入 local
fn emit_str_ptr(a: &mut Asm, param: u32, local: u32) {
    a.lget(param).i64c(3).op(op::I64_SHR_U).op(op::I32_WRAP_I64).lset(local);
}

/// rt_str_concat: (i64 a, i64 b) -> i64
/// locals: 2=pa, 3=pb, 4=la, 5=lb, 6=pout, 7=i（全 i32）
fn build_str_concat() -> Vec<u8> {
    let mut a = Asm::new();
    emit_str_ptr(&mut a, 0, 2);
    emit_str_ptr(&mut a, 1, 3);
    a.lget(2).i32_load(0).lset(4); // la
    a.lget(3).i32_load(0).lset(5); // lb
    // pout = alloc(4 + la + lb)
    a.lget(4).lget(5).op(op::I32_ADD).i32c(4).op(op::I32_ADD).call(RT_ALLOC).lset(6);
    a.lget(6).lget(4).lget(5).op(op::I32_ADD).i32_store(0);
    // copy a
    a.i32c(0).lset(7);
    a.block();
    a.loop_();
    {
        a.lget(7).lget(4).op(op::I32_GE_U).br_if(1);
        a.lget(6).i32c(4).op(op::I32_ADD).lget(7).op(op::I32_ADD); // dest = pout+4+i
        a.lget(2).i32c(4).op(op::I32_ADD).lget(7).op(op::I32_ADD).i32_load8_u(0); // byte
        a.i32_store8(0);
        a.lget(7).i32c(1).op(op::I32_ADD).lset(7);
        a.br(0);
    }
    a.end();
    a.end();
    // copy b（dest 偏移 la）
    a.i32c(0).lset(7);
    a.block();
    a.loop_();
    {
        a.lget(7).lget(5).op(op::I32_GE_U).br_if(1);
        a.lget(6).i32c(4).op(op::I32_ADD).lget(4).op(op::I32_ADD).lget(7).op(op::I32_ADD);
        a.lget(3).i32c(4).op(op::I32_ADD).lget(7).op(op::I32_ADD).i32_load8_u(0);
        a.i32_store8(0);
        a.lget(7).i32c(1).op(op::I32_ADD).lset(7);
        a.br(0);
    }
    a.end();
    a.end();
    a.lget(6).op(op::I64_EXTEND_I32_S).tag_int().i64c(TAG_STR).op(op::I64_OR);
    a.b
}

/// rt_display: (i64 v) -> i64 Str；对齐解释器 to_display
/// Int→itoa，F64→ftoa，Bool→"true"/"false"，Unit→"()"，Str→原样
fn build_display(true_off: u32, false_off: u32, unit_off: u32) -> Vec<u8> {
    let tag_str_of = |off: u32| -> i64 { ((off as i64) << 3) | TAG_STR };
    let mut a = Asm::new();
    a.tag_is(0, TAG_INT).if_i64();
    {
        a.lget(0).call(RT_ITOA);
    }
    a.else_();
    {
        a.tag_is(0, TAG_STR).if_i64();
        {
            a.lget(0); // Str 原样
        }
        a.else_();
        {
            a.tag_is(0, TAG_BOOL).if_i64();
            {
                // (v>>3)!=0 → true
                a.lget(0).i64c(3).op(op::I64_SHR_U).op(op::I32_WRAP_I64).if_i64();
                a.i64c(tag_str_of(true_off));
                a.else_();
                a.i64c(tag_str_of(false_off));
                a.end();
            }
            a.else_();
            {
                a.tag_is(0, TAG_UNIT).if_i64();
                {
                    a.i64c(tag_str_of(unit_off));
                }
                a.else_();
                {
                    a.tag_is(0, TAG_F64).if_i64();
                    {
                        a.lget(0).call(RT_UNBOX_F64).call(RT_FTOA_STR);
                    }
                    a.else_();
                    {
                        // 枚举：rt_enum_str（7.5）
                        a.tag_is(0, TAG_ENUM).if_i64();
                        {
                            a.lget(0).call(RT_ENUM_STR);
                        }
                        a.else_().op(op::UNREACHABLE).end(); // List/Map/闭包等的 display 在 7.6
                        a.end();
                    }
                    a.end();
            }
            a.end();
        }
        a.end();
    }
    a.end();
    }
    a.b
}

/// rt_itoa: (i64 tagged Int) -> i64 Str
/// locals: 1=mag(i64), 2=pos(i32), 3=neg(i32), 4=len(i32), 5=p(i32), 6=i(i32)
/// 数字倒序写入 ftoa scratch 尾部（global 1 + 64 处往前），再正向拷到堆对象
fn build_itoa() -> Vec<u8> {
    let mut a = Asm::new();
    a.lget(0).untag().lset(1); // 原值
    // neg = n < 0
    a.lget(1).i64c(0).op(op::I64_LT_S).lset(3);
    // mag = neg ? 0 - n : n（wrapping，覆盖 i64::MIN）
    a.lget(3).if_i64();
    a.i64c(0).lget(1).op(op::I64_SUB);
    a.else_();
    a.lget(1);
    a.end();
    a.lset(1);
    // pos = buf + 64（从尾部往前写）
    a.gget(1).i32c(64).op(op::I32_ADD).lset(2);
    // do-while：写数字
    a.loop_();
    {
        a.lget(2).i32c(1).op(op::I32_SUB).lset(2);
        a.lget(2).lget(1).i64c(10).op(op::I64_REM_U).i64c(48).op(op::I64_ADD).op(op::I32_WRAP_I64).i32_store8(0);
        a.lget(1).i64c(10).op(op::I64_DIV_U).lset(1);
        a.lget(1).i64c(0).op(op::I64_GT_U).br_if(0);
    }
    a.end();
    // 负号
    a.lget(3).if_();
    {
        a.lget(2).i32c(1).op(op::I32_SUB).lset(2);
        a.lget(2).i32c(45).i32_store8(0);
    }
    a.end();
    // len = (buf+64) - pos
    a.gget(1).i32c(64).op(op::I32_ADD).lget(2).op(op::I32_SUB).lset(4);
    // p = alloc(4 + len)；store len；正向拷贝
    a.lget(4).i32c(4).op(op::I32_ADD).call(RT_ALLOC).lset(5);
    a.lget(5).lget(4).i32_store(0);
    a.i32c(0).lset(6);
    a.block();
    a.loop_();
    {
        a.lget(6).lget(4).op(op::I32_GE_U).br_if(1);
        a.lget(5).i32c(4).op(op::I32_ADD).lget(6).op(op::I32_ADD);
        a.lget(2).lget(6).op(op::I32_ADD).i32_load8_u(0);
        a.i32_store8(0);
        a.lget(6).i32c(1).op(op::I32_ADD).lset(6);
        a.br(0);
    }
    a.end();
    a.end();
    a.lget(5).op(op::I64_EXTEND_I32_S).tag_int().i64c(TAG_STR).op(op::I64_OR);
    a.b
}

/// rt_ftoa_str: (f64) -> i64 Str；经宿主 lom_ftoa 导入写 scratch，再拷到堆对象
/// locals: 1=len, 2=p, 3=i（全 i32）
fn build_ftoa_str() -> Vec<u8> {
    let mut a = Asm::new();
    a.lget(0).gget(1).call(IMP_FTOA).lset(1); // len = ftoa(v, buf)
    a.lget(1).i32c(4).op(op::I32_ADD).call(RT_ALLOC).lset(2);
    a.lget(2).lget(1).i32_store(0);
    a.i32c(0).lset(3);
    a.block();
    a.loop_();
    {
        a.lget(3).lget(1).op(op::I32_GE_U).br_if(1);
        a.lget(2).i32c(4).op(op::I32_ADD).lget(3).op(op::I32_ADD);
        a.gget(1).lget(3).op(op::I32_ADD).i32_load8_u(0);
        a.i32_store8(0);
        a.lget(3).i32c(1).op(op::I32_ADD).lset(3);
        a.br(0);
    }
    a.end();
    a.end();
    a.lget(2).op(op::I64_EXTEND_I32_S).tag_int().i64c(TAG_STR).op(op::I64_OR);
    a.b
}

/// rt_str_len: (i64 Str) -> i64 Int；字符数 = 非 UTF-8 续字节计数（对齐 chars().count()）
/// locals: 1=ptr, 2=len, 3=i, 4=count（全 i32）
fn build_str_len() -> Vec<u8> {
    let mut a = Asm::new();
    emit_str_ptr(&mut a, 0, 1);
    a.lget(1).i32_load(0).lset(2);
    a.i32c(0).lset(3).i32c(0).lset(4);
    a.block();
    a.loop_();
    {
        a.lget(3).lget(2).op(op::I32_GE_U).br_if(1);
        // (b & 0xC0) != 0x80 → count++（非续字节 = 字符头）
        a.lget(1).i32c(4).op(op::I32_ADD).lget(3).op(op::I32_ADD).i32_load8_u(0);
        a.i32c(0xC0).op(op::I32_AND).i32c(0x80).op(op::I32_NE).if_();
        {
            a.lget(4).i32c(1).op(op::I32_ADD).lset(4);
        }
        a.end();
        a.lget(3).i32c(1).op(op::I32_ADD).lset(3);
        a.br(0);
    }
    a.end();
    a.end();
    a.lget(4).op(op::I64_EXTEND_I32_S).tag_int();
    a.b
}

/// rt_stoi: (i64 Str) -> i64；解析 [+-]?digits，失败/溢出返回 Unit（对齐解释器）
/// locals: 1=ptr, 2=len, 3=i, 4=neg, 5=c（全 i32），6=acc（i64）
fn build_stoi() -> Vec<u8> {
    let mut a = Asm::new();
    emit_str_ptr(&mut a, 0, 1);
    a.lget(1).i32_load(0).lset(2);
    // 空串 → Unit
    a.lget(2).op(op::I32_EQZ).if_();
    a.i64c(V_UNIT).op(op::RETURN);
    a.end();
    a.i32c(0).lset(3).i32c(0).lset(4);
    // 符号
    a.lget(1).i32_load8_u(4).i32c(45).op(op::I32_EQ).if_(); // '-'
    {
        a.i32c(1).lset(4).i32c(1).lset(3);
    }
    a.else_();
    {
        a.lget(1).i32_load8_u(4).i32c(43).op(op::I32_EQ).if_(); // '+'
        a.i32c(1).lset(3);
        a.end();
    }
    a.end();
    // 只有符号位 → Unit
    a.lget(3).lget(2).op(op::I32_EQ).if_();
    a.i64c(V_UNIT).op(op::RETURN);
    a.end();
    // 数字循环
    a.i64c(0).lset(6);
    a.block();
    a.loop_();
    {
        a.lget(3).lget(2).op(op::I32_GE_U).br_if(1);
        // c = byte - 48；c >u 9 → Unit（无符号比较同时挡掉负数）
        a.lget(1).i32c(4).op(op::I32_ADD).lget(3).op(op::I32_ADD).i32_load8_u(0).i32c(48).op(op::I32_SUB).lset(5);
        a.lget(5).i32c(9).op(op::I32_GT_U).if_();
        a.i64c(V_UNIT).op(op::RETURN);
        a.end();
        // 溢出预检：acc >u (u64::MAX-9)/10 = 1844674407370955161 → Unit
        a.lget(6).i64c(1844674407370955161).op(op::I64_GT_U).if_();
        a.i64c(V_UNIT).op(op::RETURN);
        a.end();
        a.lget(6).i64c(10).op(op::I64_MUL).lget(5).op(op::I64_EXTEND_I32_S).op(op::I64_ADD).lset(6);
        a.lget(3).i32c(1).op(op::I32_ADD).lset(3);
        a.br(0);
    }
    a.end();
    a.end();
    // 范围检查 + 符号
    a.lget(4).if_i64();
    {
        // neg：acc >u 2^63（位型 = i64::MIN）→ Unit；否则 0 - acc
        a.lget(6).i64c(i64::MIN).op(op::I64_GT_U).if_();
        a.i64c(V_UNIT).op(op::RETURN);
        a.end();
        a.i64c(0).lget(6).op(op::I64_SUB).tag_int();
    }
    a.else_();
    {
        a.lget(6).i64c(i64::MAX).op(op::I64_GT_U).if_();
        a.i64c(V_UNIT).op(op::RETURN);
        a.end();
        a.lget(6).tag_int();
    }
    a.end();
    a.b
}

/// 发射"byte 在 local `b` 是 ASCII 空白（32/9/10/13）"的 i32 条件
fn emit_is_ws(a: &mut Asm, b: u32) {
    a.lget(b).i32c(32).op(op::I32_EQ);
    a.lget(b).i32c(9).op(op::I32_EQ).op(op::I32_OR);
    a.lget(b).i32c(10).op(op::I32_EQ).op(op::I32_OR);
    a.lget(b).i32c(13).op(op::I32_EQ).op(op::I32_OR);
}

/// rt_trim: (i64 Str) -> i64；ASCII 空白（与解释器 Unicode trim 的差异已记录）
/// locals: 1=ptr, 2=len, 3=start, 4=end, 5=b, 6=pout, 7=i（全 i32）
fn build_trim() -> Vec<u8> {
    let mut a = Asm::new();
    emit_str_ptr(&mut a, 0, 1);
    a.lget(1).i32_load(0).lset(2);
    a.i32c(0).lset(3);
    a.lget(2).lset(4);
    // 前扫
    a.block();
    a.loop_();
    {
        a.lget(3).lget(4).op(op::I32_GE_U).br_if(1);
        a.lget(1).i32c(4).op(op::I32_ADD).lget(3).op(op::I32_ADD).i32_load8_u(0).lset(5);
        emit_is_ws(&mut a, 5);
        a.op(op::I32_EQZ).br_if(1); // 非空白 → 停
        a.lget(3).i32c(1).op(op::I32_ADD).lset(3);
        a.br(0);
    }
    a.end();
    a.end();
    // 后扫
    a.block();
    a.loop_();
    {
        a.lget(4).lget(3).op(op::I32_LE_S).br_if(1);
        a.lget(1).i32c(4).op(op::I32_ADD).lget(4).op(op::I32_ADD).i32c(1).op(op::I32_SUB).i32_load8_u(0).lset(5);
        emit_is_ws(&mut a, 5);
        a.op(op::I32_EQZ).br_if(1);
        a.lget(4).i32c(1).op(op::I32_SUB).lset(4);
        a.br(0);
    }
    a.end();
    a.end();
    // out = alloc(4 + (end-start))
    a.lget(4).lget(3).op(op::I32_SUB).i32c(4).op(op::I32_ADD).call(RT_ALLOC).lset(6);
    a.lget(6).lget(4).lget(3).op(op::I32_SUB).i32_store(0);
    a.i32c(0).lset(7);
    a.block();
    a.loop_();
    {
        a.lget(7).lget(4).lget(3).op(op::I32_SUB).op(op::I32_GE_U).br_if(1);
        a.lget(6).i32c(4).op(op::I32_ADD).lget(7).op(op::I32_ADD);
        a.lget(1).i32c(4).op(op::I32_ADD).lget(3).op(op::I32_ADD).lget(7).op(op::I32_ADD).i32_load8_u(0);
        a.i32_store8(0);
        a.lget(7).i32c(1).op(op::I32_ADD).lset(7);
        a.br(0);
    }
    a.end();
    a.end();
    a.lget(6).op(op::I64_EXTEND_I32_S).tag_int().i64c(TAG_STR).op(op::I64_OR);
    a.b
}

/// rt_upper/rt_lower: (i64 Str) -> i64；ASCII-only（与解释器 Unicode 大小写的差异已记录）
/// locals: 1=ptr, 2=len, 3=i, 4=pout, 5=b（全 i32）
fn build_case(upper: bool) -> Vec<u8> {
    let (lo, hi) = if upper { (97, 122) } else { (65, 90) };
    let delta: i32 = if upper { -32 } else { 32 };
    let mut a = Asm::new();
    emit_str_ptr(&mut a, 0, 1);
    a.lget(1).i32_load(0).lset(2);
    a.lget(2).i32c(4).op(op::I32_ADD).call(RT_ALLOC).lset(4);
    a.lget(4).lget(2).i32_store(0);
    a.i32c(0).lset(3);
    a.block();
    a.loop_();
    {
        a.lget(3).lget(2).op(op::I32_GE_U).br_if(1);
        a.lget(1).i32c(4).op(op::I32_ADD).lget(3).op(op::I32_ADD).i32_load8_u(0).lset(5);
        // dest 地址先压栈
        a.lget(4).i32c(4).op(op::I32_ADD).lget(3).op(op::I32_ADD);
        // value：在范围内则 ±32
        a.lget(5).i32c(lo).op(op::I32_GE_S);
        a.lget(5).i32c(hi).op(op::I32_LE_S);
        a.op(op::I32_AND).if_i32();
        a.lget(5).i32c(delta).op(op::I32_ADD);
        a.else_();
        a.lget(5);
        a.end();
        a.i32_store8(0);
        a.lget(3).i32c(1).op(op::I32_ADD).lset(3);
        a.br(0);
    }
    a.end();
    a.end();
    a.lget(4).op(op::I64_EXTEND_I32_S).tag_int().i64c(TAG_STR).op(op::I64_OR);
    a.b
}

/// rt_contains: (i64 s, i64 sub) -> i64 Bool；朴素子串查找
/// locals: 2=ps, 3=ls, 4=psub, 5=lsub, 6=i, 7=j（全 i32）
fn build_contains() -> Vec<u8> {
    let mut a = Asm::new();
    emit_str_ptr(&mut a, 0, 2);
    emit_str_ptr(&mut a, 1, 4);
    a.lget(2).i32_load(0).lset(3);
    a.lget(4).i32_load(0).lset(5);
    // lsub == 0 → true；lsub > ls → false
    a.lget(5).op(op::I32_EQZ).if_();
    a.i64c(V_TRUE).op(op::RETURN);
    a.end();
    a.lget(5).lget(3).op(op::I32_GT_S).if_();
    a.i64c(V_FALSE).op(op::RETURN);
    a.end();
    a.i32c(0).lset(6);
    a.block(); // $done
    a.loop_();
    {
        // i + lsub > ls → break
        a.lget(6).lget(5).op(op::I32_ADD).lget(3).op(op::I32_GT_S).br_if(1);
        // j 扫描：全部相等 → return true
        a.i32c(0).lset(7);
        a.block(); // $next
        a.loop_();
        {
            a.lget(7).lget(5).op(op::I32_GE_U).if_();
            a.i64c(V_TRUE).op(op::RETURN);
            a.end();
            a.lget(2).i32c(4).op(op::I32_ADD).lget(6).op(op::I32_ADD).lget(7).op(op::I32_ADD).i32_load8_u(0);
            a.lget(4).i32c(4).op(op::I32_ADD).lget(7).op(op::I32_ADD).i32_load8_u(0);
            a.op(op::I32_NE).br_if(1); // 不等 → 下一个 i
            a.lget(7).i32c(1).op(op::I32_ADD).lset(7);
            a.br(0);
        }
        a.end();
        a.end();
        a.lget(6).i32c(1).op(op::I32_ADD).lset(6);
        a.br(0);
    }
    a.end();
    a.end();
    a.i64c(V_FALSE);
    a.b
}

/// rt_starts_with / rt_ends_with: (i64 s, i64 sub) -> i64 Bool
/// locals: 2=ps, 3=ls, 4=psub, 5=lsub, 6=i（全 i32）
fn build_starts_ends(is_starts: bool) -> Vec<u8> {
    let mut a = Asm::new();
    emit_str_ptr(&mut a, 0, 2);
    emit_str_ptr(&mut a, 1, 4);
    a.lget(2).i32_load(0).lset(3);
    a.lget(4).i32_load(0).lset(5);
    // lsub > ls → false
    a.lget(5).lget(3).op(op::I32_GT_S).if_();
    a.i64c(V_FALSE).op(op::RETURN);
    a.end();
    a.i32c(0).lset(6);
    a.block();
    a.loop_();
    {
        a.lget(6).lget(5).op(op::I32_GE_U).br_if(1);
        // s 侧偏移：starts 是 i；ends 是 (ls-lsub)+i
        a.lget(2).i32c(4).op(op::I32_ADD);
        if !is_starts {
            a.lget(3).lget(5).op(op::I32_SUB);
            a.op(op::I32_ADD); // ps+4 + (ls-lsub)——曾漏这一个 ADD（地址算成 (ls-lsub)+i）
        }
        a.lget(6).op(op::I32_ADD).i32_load8_u(0);
        a.lget(4).i32c(4).op(op::I32_ADD).lget(6).op(op::I32_ADD).i32_load8_u(0);
        a.op(op::I32_NE).if_();
        a.i64c(V_FALSE).op(op::RETURN);
        a.end();
        a.lget(6).i32c(1).op(op::I32_ADD).lset(6);
        a.br(0);
    }
    a.end();
    a.end();
    a.i64c(V_TRUE);
    a.b
}

/// rt_replace: (i64 s, i64 from, i64 to) -> i64 Str；全量替换（两遍扫描）
/// locals: 3=ps, 4=ls, 5=pf, 6=lf, 7=pt, 8=lt, 9=i, 10=j, 11=cnt, 12=olen, 13=pout, 14=o, 15=mf
fn build_replace() -> Vec<u8> {
    let mut a = Asm::new();
    emit_str_ptr(&mut a, 0, 3);
    emit_str_ptr(&mut a, 1, 5);
    emit_str_ptr(&mut a, 2, 7);
    a.lget(3).i32_load(0).lset(4); // ls
    a.lget(5).i32_load(0).lset(6); // lf
    a.lget(7).i32_load(0).lset(8); // lt
    // 特例：from 为空（Rust 语义：每个间隙插入 to，含两端）
    a.lget(6).op(op::I32_EQZ).if_();
    {
        // olen = ls + lt * (ls + 1)
        a.lget(4).lget(8).lget(4).i32c(1).op(op::I32_ADD).op(op::I32_MUL).op(op::I32_ADD).lset(12);
        a.lget(12).i32c(4).op(op::I32_ADD).call(RT_ALLOC).lset(13);
        a.lget(13).lget(12).i32_store(0);
        a.i32c(0).lset(9); // i
        a.i32c(0).lset(14); // o
        a.block();
        a.loop_();
        {
            a.lget(9).lget(4).op(op::I32_GT_S).br_if(1); // i in 0..=ls
            // 写 to
            a.i32c(0).lset(10);
            a.block();
            a.loop_();
            {
                a.lget(10).lget(8).op(op::I32_GE_U).br_if(1);
                a.lget(13).i32c(4).op(op::I32_ADD).lget(14).op(op::I32_ADD);
                a.lget(7).i32c(4).op(op::I32_ADD).lget(10).op(op::I32_ADD).i32_load8_u(0);
                a.i32_store8(0);
                a.lget(14).i32c(1).op(op::I32_ADD).lset(14);
                a.lget(10).i32c(1).op(op::I32_ADD).lset(10);
                a.br(0);
            }
            a.end();
            a.end();
            // 写 s[i]（i < ls 时）
            a.lget(9).lget(4).op(op::I32_LT_S).if_();
            {
                a.lget(13).i32c(4).op(op::I32_ADD).lget(14).op(op::I32_ADD);
                a.lget(3).i32c(4).op(op::I32_ADD).lget(9).op(op::I32_ADD).i32_load8_u(0);
                a.i32_store8(0);
                a.lget(14).i32c(1).op(op::I32_ADD).lset(14);
            }
            a.end();
            a.lget(9).i32c(1).op(op::I32_ADD).lset(9);
            a.br(0);
        }
        a.end();
        a.end();
        a.lget(13).op(op::I64_EXTEND_I32_S).tag_int().i64c(TAG_STR).op(op::I64_OR).op(op::RETURN);
    }
    a.end();
    // pass 1：数非重叠匹配
    a.i32c(0).lset(9).i32c(0).lset(11);
    a.block();
    a.loop_();
    {
        // i + lf > ls → break
        a.lget(9).lget(6).op(op::I32_ADD).lget(4).op(op::I32_GT_S).br_if(1);
        // match_at(i) → mf
        emit_match_at(&mut a, 3, 5, 6, 9, 10, 15);
        a.lget(15).if_();
        {
            a.lget(11).i32c(1).op(op::I32_ADD).lset(11);
            a.lget(9).lget(6).op(op::I32_ADD).lset(9);
        }
        a.else_();
        {
            a.lget(9).i32c(1).op(op::I32_ADD).lset(9);
        }
        a.end();
        a.br(0);
    }
    a.end();
    a.end();
    // olen = ls - cnt*lf + cnt*lt
    a.lget(4).lget(11).lget(6).op(op::I32_MUL).op(op::I32_SUB).lget(11).lget(8).op(op::I32_MUL).op(op::I32_ADD).lset(12);
    a.lget(12).i32c(4).op(op::I32_ADD).call(RT_ALLOC).lset(13);
    a.lget(13).lget(12).i32_store(0);
    // pass 2：填充
    a.i32c(0).lset(9).i32c(0).lset(14);
    a.block();
    a.loop_();
    {
        a.lget(9).lget(4).op(op::I32_GE_U).br_if(1); // i >= ls → done
        // 可匹配？
        a.lget(9).lget(6).op(op::I32_ADD).lget(4).op(op::I32_LE_S).if_();
        {
            emit_match_at(&mut a, 3, 5, 6, 9, 10, 15);
        }
        a.else_();
        {
            a.i32c(0).lset(15);
        }
        a.end();
        a.lget(15).if_();
        {
            // 写 to 全文
            a.i32c(0).lset(10);
            a.block();
            a.loop_();
            {
                a.lget(10).lget(8).op(op::I32_GE_U).br_if(1);
                a.lget(13).i32c(4).op(op::I32_ADD).lget(14).op(op::I32_ADD);
                a.lget(7).i32c(4).op(op::I32_ADD).lget(10).op(op::I32_ADD).i32_load8_u(0);
                a.i32_store8(0);
                a.lget(14).i32c(1).op(op::I32_ADD).lset(14);
                a.lget(10).i32c(1).op(op::I32_ADD).lset(10);
                a.br(0);
            }
            a.end();
            a.end();
            a.lget(9).lget(6).op(op::I32_ADD).lset(9);
        }
        a.else_();
        {
            // 抄一个字节
            a.lget(13).i32c(4).op(op::I32_ADD).lget(14).op(op::I32_ADD);
            a.lget(3).i32c(4).op(op::I32_ADD).lget(9).op(op::I32_ADD).i32_load8_u(0);
            a.i32_store8(0);
            a.lget(14).i32c(1).op(op::I32_ADD).lset(14);
            a.lget(9).i32c(1).op(op::I32_ADD).lset(9);
        }
        a.end();
        a.br(0);
    }
    a.end();
    a.end();
    a.lget(13).op(op::I64_EXTEND_I32_S).tag_int().i64c(TAG_STR).op(op::I64_OR);
    a.b
}

/// 发射 match_at(i)：ps 从偏移 i 起与 pf 逐字节比较（lf 长度），结果(1/0)写 local mf
/// 参数：ps/lf/i/j/mf 的 local 编号
fn emit_match_at(a: &mut Asm, ps: u32, pf: u32, lf: u32, i: u32, j: u32, mf: u32) {
    a.i32c(1).lset(mf);
    a.i32c(0).lset(j);
    a.block();
    a.loop_();
    {
        a.lget(j).lget(lf).op(op::I32_GE_U).br_if(1);
        a.lget(ps).i32c(4).op(op::I32_ADD).lget(i).op(op::I32_ADD).lget(j).op(op::I32_ADD).i32_load8_u(0);
        a.lget(pf).i32c(4).op(op::I32_ADD).lget(j).op(op::I32_ADD).i32_load8_u(0);
        a.op(op::I32_NE).if_();
        {
            a.i32c(0).lset(mf);
            a.br(2); // 跳出 block（loop 是 0，block 是 1，br(2)?? —— 见下方注释）
        }
        a.end();
        a.lget(j).i32c(1).op(op::I32_ADD).lset(j);
        a.br(0);
    }
    a.end();
    a.end();
}

/// rt_str_cmp: (i64 a, i64 b) -> i32(-1/0/1)；字节序（对齐 Rust str Ord）
/// locals: 2=pa, 3=la, 4=pb, 5=lb, 6=i, 7=min（全 i32）
fn build_str_cmp() -> Vec<u8> {
    let mut a = Asm::new();
    emit_str_ptr(&mut a, 0, 2);
    emit_str_ptr(&mut a, 1, 4);
    a.lget(2).i32_load(0).lset(3);
    a.lget(4).i32_load(0).lset(5);
    // min = la < lb ? la : lb
    a.lget(3).lget(5).lget(3).lget(5).op(op::I32_LT_S).op(op::SELECT).lset(7);
    a.i32c(0).lset(6);
    a.block();
    a.loop_();
    {
        a.lget(6).lget(7).op(op::I32_GE_U).br_if(1);
        // ca vs cb
        a.lget(2).i32c(4).op(op::I32_ADD).lget(6).op(op::I32_ADD).i32_load8_u(0); // ca
        a.lget(4).i32c(4).op(op::I32_ADD).lget(6).op(op::I32_ADD).i32_load8_u(0); // cb
        // ca < cb → -1；ca > cb → 1
        a.op(op::I32_SUB); // ca - cb（栈上一个值）
        // 用 tee 思路：重复装入太啰嗦——直接两分支重算
        // 栈上有 diff；diff < 0 → return -1
        // i32 没有 tee 便捷法，用 if 双分支+重算开销可接受（比较循环不长）
        // 这里 diff 已在栈：直接判断
        a.i32c(0).op(op::I32_LT_S).if_();
        a.i32c(-1).op(op::RETURN);
        a.end();
        // 重算 diff > 0
        a.lget(2).i32c(4).op(op::I32_ADD).lget(6).op(op::I32_ADD).i32_load8_u(0);
        a.lget(4).i32c(4).op(op::I32_ADD).lget(6).op(op::I32_ADD).i32_load8_u(0);
        a.op(op::I32_SUB).i32c(0).op(op::I32_GT_S).if_();
        a.i32c(1).op(op::RETURN);
        a.end();
        a.lget(6).i32c(1).op(op::I32_ADD).lset(6);
        a.br(0);
    }
    a.end();
    a.end();
    // 等长前缀相等：短者小
    a.lget(3).lget(5).op(op::I32_LT_S).if_();
    a.i32c(-1).op(op::RETURN);
    a.end();
    a.lget(3).lget(5).op(op::I32_GT_S).if_();
    a.i32c(1).op(op::RETURN);
    a.end();
    a.i32c(0);
    a.b
}

/// rt_str_char_at: (i64 Str, i64 byte_off) -> i64 Str；按 UTF-8 头字节取单字符
/// locals: 2=ptr, 3=o, 4=clen, 5=p, 6=k（全 i32）
fn build_str_char_at() -> Vec<u8> {
    let mut a = Asm::new();
    emit_str_ptr(&mut a, 0, 2);
    a.lget(1).op(op::I32_WRAP_I64).lset(3);
    // clen：0xF0+ → 4，0xE0+ → 3，0xC0+ → 2，否则 1
    a.i32c(1).lset(4);
    a.lget(2).i32c(4).op(op::I32_ADD).lget(3).op(op::I32_ADD).i32_load8_u(0).i32c(0xF0).op(op::I32_GE_U).if_();
    a.i32c(4).lset(4);
    a.else_();
    {
        a.lget(2).i32c(4).op(op::I32_ADD).lget(3).op(op::I32_ADD).i32_load8_u(0).i32c(0xE0).op(op::I32_GE_U).if_();
        a.i32c(3).lset(4);
        a.else_();
        {
            a.lget(2).i32c(4).op(op::I32_ADD).lget(3).op(op::I32_ADD).i32_load8_u(0).i32c(0xC0).op(op::I32_GE_U).if_();
            a.i32c(2).lset(4);
            a.end();
        }
        a.end();
    }
    a.end();
    // p = alloc(4 + clen)；store len；逐字节拷贝
    a.lget(4).i32c(4).op(op::I32_ADD).call(RT_ALLOC).lset(5);
    a.lget(5).lget(4).i32_store(0);
    a.i32c(0).lset(6);
    a.block();
    a.loop_();
    {
        a.lget(6).lget(4).op(op::I32_GE_U).br_if(1);
        a.lget(5).i32c(4).op(op::I32_ADD).lget(6).op(op::I32_ADD);
        a.lget(2).i32c(4).op(op::I32_ADD).lget(3).op(op::I32_ADD).lget(6).op(op::I32_ADD).i32_load8_u(0);
        a.i32_store8(0);
        a.lget(6).i32c(1).op(op::I32_ADD).lset(6);
        a.br(0);
    }
    a.end();
    a.end();
    a.lget(5).op(op::I64_EXTEND_I32_S).tag_int().i64c(TAG_STR).op(op::I64_OR);
    a.b
}

// ===== 7.5：枚举 helper（enum_eq 体直接构建；print/str 的体在 finalize 延迟填充）=====

/// rt_enum_eq: (i64 a, i64 b) -> i32；变体 idx + 参数个数 + 参数递归 rt_eq
/// locals: 2=pa, 3=pb, 4=na, 5=k（全 i32）
fn build_enum_eq() -> Vec<u8> {
    let mut a = Asm::new();
    emit_str_ptr(&mut a, 0, 2); // 复用：tagged → 堆指针（枚举对象同布局）
    emit_str_ptr(&mut a, 1, 3);
    // 变体 idx 不等 → 0
    a.lget(2).i32_load(0).lget(3).i32_load(0).op(op::I32_NE).if_();
    a.i32c(0).op(op::RETURN);
    a.end();
    // 参数个数不等 → 0
    a.lget(2).i32_load(4).lget(3).i32_load(4).op(op::I32_NE).if_();
    a.i32c(0).op(op::RETURN);
    a.end();
    a.lget(2).i32_load(4).lset(4); // na
    a.i32c(0).lset(5);
    a.block();
    a.loop_();
    {
        a.lget(5).lget(4).op(op::I32_GE_U).br_if(1);
        // rt_eq(arg_a_k, arg_b_k) == false → return 0
        a.lget(2).lget(5).i32c(8).op(op::I32_MUL).op(op::I32_ADD).i64_load(8);
        a.lget(3).lget(5).i32c(8).op(op::I32_MUL).op(op::I32_ADD).i64_load(8);
        a.call(RT_EQ).untag().op(op::I32_WRAP_I64).op(op::I32_EQZ).if_();
        a.i32c(0).op(op::RETURN);
        a.end();
        a.lget(5).i32c(1).op(op::I32_ADD).lset(5);
        a.br(0);
    }
    a.end();
    a.end();
    a.i32c(1);
    a.b
}

/// rt_enum_print: (i64 v, i64 nl) -> ()；变体名 + (arg, ...)，参数递归 rt_print
/// locals: 2=p, 3=idx, 4=n, 5=k, 6=name_off（全 i32）
fn build_enum_print(table_off: u32, open_off: u32, close_off: u32, comma_off: u32) -> Vec<u8> {
    let mut a = Asm::new();
    emit_str_ptr(&mut a, 0, 2);
    a.lget(2).i32_load(0).lset(3); // idx
    a.lget(2).i32_load(4).lset(4); // n
    // name_off = load(table + idx*4)
    a.i32c(table_off as i32).lget(3).i32c(4).op(op::I32_MUL).op(op::I32_ADD).i32_load(0).lset(6);
    // lom_print(name_off+4, name_len)
    a.lget(6).i32c(4).op(op::I32_ADD).lget(6).i32_load(0).call(IMP_PRINT_STR);
    // 参数部分
    a.lget(4).i32c(0).op(op::I32_GT_S).if_();
    {
        a.i32c((open_off + 4) as i32).i32c(1).call(IMP_PRINT_STR); // "("
        a.i32c(0).lset(5);
        a.block();
        a.loop_();
        {
            a.lget(5).lget(4).op(op::I32_GE_U).br_if(1);
            // rt_print(arg_k, 0)
            a.lget(2).lget(5).i32c(8).op(op::I32_MUL).op(op::I32_ADD).i64_load(8);
            a.i64c(0).call(RT_PRINT);
            // k+1 < n → ", "
            a.lget(5).i32c(1).op(op::I32_ADD).lget(4).op(op::I32_LT_S).if_();
            {
                a.i32c((comma_off + 4) as i32).i32c(2).call(IMP_PRINT_STR);
            }
            a.end();
            a.lget(5).i32c(1).op(op::I32_ADD).lset(5);
            a.br(0);
        }
        a.end();
        a.end();
        a.i32c((close_off + 4) as i32).i32c(1).call(IMP_PRINT_STR); // ")"
    }
    a.end();
    // 换行
    a.lget(1).i64c(0).op(op::I64_NE).if_();
    {
        a.i32c(0).i32c(1).call(IMP_PRINT_STR);
    }
    a.end();
    a.b
}

/// rt_enum_str: (i64 v) -> i64 Str；枚举 → to_display 字符串（concat 链）
/// locals: 2=p, 3=idx, 4=n, 5=k, 6=name_off（i32），7=acc（i64）
fn build_enum_str(table_off: u32, open_off: u32, close_off: u32, comma_off: u32) -> Vec<u8> {
    let tag_str = |off: u32| -> i64 { ((off as i64) << 3) | TAG_STR };
    let mut a = Asm::new();
    emit_str_ptr(&mut a, 0, 2);
    a.lget(2).i32_load(0).lset(3);
    a.lget(2).i32_load(4).lset(4);
    a.i32c(table_off as i32).lget(3).i32c(4).op(op::I32_MUL).op(op::I32_ADD).i32_load(0).lset(6);
    // acc = 变体名（静态串直接作值）
    a.lget(6).op(op::I64_EXTEND_I32_S).tag_int().i64c(TAG_STR).op(op::I64_OR).lset(7);
    a.lget(4).op(op::I32_EQZ).if_i64();
    {
        a.lget(7);
    }
    a.else_();
    {
        // acc += "("
        a.lget(7).i64c(tag_str(open_off)).call(RT_STR_CONCAT).lset(7);
        a.i32c(0).lset(5);
        a.block();
        a.loop_();
        {
            a.lget(5).lget(4).op(op::I32_GE_U).br_if(1);
            // acc += display(arg_k)
            a.lget(7).lget(2).lget(5).i32c(8).op(op::I32_MUL).op(op::I32_ADD).i64_load(8).call(RT_DISPLAY).call(RT_STR_CONCAT).lset(7);
            // k+1 < n → acc += ", "
            a.lget(5).i32c(1).op(op::I32_ADD).lget(4).op(op::I32_LT_S).if_();
            {
                a.lget(7).i64c(tag_str(comma_off)).call(RT_STR_CONCAT).lset(7);
            }
            a.end();
            a.lget(5).i32c(1).op(op::I32_ADD).lset(5);
            a.br(0);
        }
        a.end();
        a.end();
        // acc += ")"
        a.lget(7).i64c(tag_str(close_off)).call(RT_STR_CONCAT).lset(7);
        a.lget(7);
    }
    a.end();
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
                || self.variant_idx.contains_key(name)
                || name == "println"
                || name == "print"
                || self.available_builtins.contains(name)
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
        let funcidx = N_IMPORTS + self.m.funcs.len() as u32;
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
            let funcidx = N_IMPORTS + self.m.funcs.len() as u32;
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
        // range/记录 → 7.6（match 自 7.5 起已支持，见 e2e）
        let e = compile("fn main() -> Unit\n    for i in 1..3\n        println(i)\n    end\nend").unwrap_err();
        assert!(e.contains("7.6"), "{}", e);
        let e = compile("fn main() -> Unit\n    let r = {x: 1}\n    println(r)\nend").unwrap_err();
        assert!(e.contains("7.6"), "{}", e);
        // match 现在编译通过
        compile("fn main() -> Unit\n    match 1\n        _ => println(1)\n    end\nend").unwrap();
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

    // ===== Phase 7.4: 字符串 / stdlib =====

    #[test]
    fn e2e_string_concat_promotion() {
        // v0.4.1 拼接提升：任一侧 String 即可拼接（Int/Float/Bool/Unit 全测）
        check(
            "fn main() -> Unit\n    println(\"n = \" + 42)\n    println(\"f = \" + 1.5)\n    println(\"b = \" + True)\n    println(1 + 2)\nend",
            "concat",
            "n = 42\nf = 1.5\nb = true\n3\n",
        );
    }

    #[test]
    fn e2e_string_builtins() {
        check(
            "from string import { len, int_to_string, string_to_int, trim, upper, lower, contains, replace, starts_with, ends_with }\nfn main() -> Unit\n    println(len(\"hello\"))\n    println(int_to_string(42) + \"!\")\n    println(string_to_int(\"123\") + 1)\n    println(string_to_int(\"abc\"))\n    println(trim(\"  hi  \"))\n    println(upper(\"hello\"))\n    println(lower(\"HeLLo\"))\n    println(contains(\"hello world\", \"o w\"))\n    println(starts_with(\"hello\", \"he\"))\n    println(ends_with(\"hello\", \"lo\"))\n    println(replace(\"a-b-c\", \"-\", \"+\"))\n    println(replace(\"abc\", \"\", \"x\"))\nend",
            "strlib",
            "5\n42!\n124\n()\nhi\nHELLO\nhello\ntrue\ntrue\ntrue\na+b+c\nxaxbxcx\n",
        );
    }

    #[test]
    fn e2e_math_builtins() {
        check(
            "from math import { sqrt, abs, min, max }\nfn main() -> Unit\n    println(sqrt(16.0))\n    println(sqrt(9))\n    println(abs(-7))\n    println(abs(-7.5))\n    println(min(3, 7))\n    println(max(3.5, 1.5))\nend",
            "math",
            "4.0\n3.0\n7\n7.5\n3\n3.5\n",
        );
    }

    #[test]
    fn e2e_string_compare_and_for_char() {
        // 字符串大小比较（7.4 补齐 7.2 的缺口）+ for 逐字符迭代
        check(
            "fn main() -> Unit\n    println(\"abc\" < \"abd\")\n    println(\"b\" > \"a\")\n    println(\"abc\" <= \"abc\")\n    let mut s = \"\"\n    for c in \"hey\"\n        s = s + \"[\" + c + \"]\"\n    end\n    println(s)\nend",
            "strcmp",
            "true\ntrue\ntrue\n[h][e][y]\n",
        );
    }

    #[test]
    fn e2e_import_alias() {
        // from io import { println as log }（别名解析）
        check(
            "from io import { println as log }\nfrom string import { len as slen }\nfn main() -> Unit\n    log(\"hi\")\n    log(slen(\"hello\"))\nend",
            "alias",
            "hi\n5\n",
        );
    }

    // ===== Phase 7.5: 枚举 / match / ? =====

    #[test]
    fn e2e_match_option_result() {
        check(
            "fn parse_int(s: String) -> Result<Int, String>\n    from_string_check(s)\nend\nfn from_string_check(s: String) -> Result<Int, String>\n    if s == \"x\"\n        Err(\"bad\")\n    else\n        Ok(42)\n    end\nend\nfn main() -> Unit\n    match parse_int(\"x\")\n        Ok(n) => println(n)\n        Err(e) => println(\"err: \" + e)\n    end\n    match parse_int(\"y\")\n        Ok(n) => println(n)\n        Err(e) => println(\"err: \" + e)\n    end\nend",
            "match_res",
            "err: bad\n42\n",
        );
    }

    #[test]
    fn e2e_match_user_enum_and_literals() {
        check(
            "enum Color = Red | Green | Blue\nfn code(c: Color) -> Int\n    match c\n        Red => 1\n        Green => 2\n        Blue => 3\n    end\nend\nfn main() -> Unit\n    println(code(Green))\n    match 2\n        1 => println(\"one\")\n        2 => println(\"two\")\n        _ => println(\"other\")\n    end\n    match \"hi\"\n        \"hello\" => println(\"hello\")\n        _ => println(\"fallback\")\n    end\nend",
            "match_enum",
            "2\ntwo\nfallback\n",
        );
    }

    #[test]
    fn e2e_match_guard() {
        // guard 为假穿透下一臂（v0.4.2 语义）
        check(
            "fn classify(n: Int) -> String\n    match n\n        x if x < 0 => \"neg\"\n        x if x == 0 => \"zero\"\n        _ => \"pos\"\n    end\nend\nfn main() -> Unit\n    println(classify(-5))\n    println(classify(0))\n    println(classify(9))\nend",
            "guard",
            "neg\nzero\npos\n",
        );
    }

    #[test]
    fn e2e_try_propagation() {
        // `?`：Ok 解包；Err/None 提前返回（对齐解释器 EarlyReturn）
        check(
            "fn fail() -> Result<Int, String>\n    Err(\"boom\")\nend\nfn caller() -> Result<Int, String>\n    let x = fail()?\n    Ok(x + 1)\nend\nfn maybe(b: Bool) -> Option<Int>\n    if b\n        Some(7)\n    else\n        None\n    end\nend\nfn use_maybe(b: Bool) -> Option<Int>\n    let v = maybe(b)?\n    Some(v * 2)\nend\nfn main() -> Unit\n    match caller()\n        Ok(n) => println(n)\n        Err(e) => println(\"err: \" + e)\n    end\n    println(use_maybe(True))\n    println(use_maybe(False))\nend",
            "try",
            "err: boom\nSome(14)\nNone\n",
        );
    }

    #[test]
    fn e2e_enum_print_and_eq() {
        // 枚举打印（递归参数）+ 枚举相等（结构递归）
        check(
            "fn main() -> Unit\n    println(Ok(42))\n    println(None)\n    println(Ok(1) == Ok(1))\n    println(Ok(1) == Ok(2))\n    println(None == None)\n    println(Some(\"hi\") == Some(\"hi\"))\nend",
            "enum_pe",
            "Ok(42)\nNone\ntrue\nfalse\ntrue\ntrue\n",
        );
    }
}
