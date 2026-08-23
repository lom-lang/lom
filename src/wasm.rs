// Lom WASM 后端 —— Phase 7.1：二进制 emitter 骨架（手写，零依赖）
//
// 设计依据 RFC-0002：
// - 不依赖任何第三方库（wabt/walrus/binaryen 全拒），直接按 WASM 二进制格式规范写字节
// - 本阶段（7.1）只交付：LEB128 编码、section 拼装、最小模块（hello 级常量打印）
// - 值表示/tagging/编译 Lom AST 是 7.2 之后的事，本文件只提供"容器"能力
//
// 参考：WebAssembly Core Specification 1.0 二进制格式
//   模块 = magic(00 61 73 6D) + version(01 00 00 00) + sections
//   section = id(u8) + payload_len(u32 LEB) + payload
//   vec(T)  = count(u32 LEB) + T*

// 7.1 只交付容器能力：本模块的 API（leb_*、Module 构建器等）在 7.2+ 编译 Lom AST 时
// 才被 main 路径消费，当前仅测试使用——属有意保留的 API 面（与既有 schema 字段同例）。
#![allow(dead_code)]

/// 无符号 LEB128 编码（u32/u64 通用，这里只需要到 u64）
pub fn leb_u(mut v: u64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let byte = (v & 0x7F) as u8;
        v >>= 7;
        if v == 0 {
            out.push(byte);
            return out;
        }
        out.push(byte | 0x80);
    }
}

/// 有符号 LEB128 编码（i32.const / i64.const 的立即数用这个）
pub fn leb_s(mut v: i64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let byte = (v & 0x7F) as u8;
        v >>= 7; // 算术右移，保符号位
        // 终止条件：剩余值全为符号位，且当前字节的 bit6 与符号一致
        let sign_bit_set = byte & 0x40 != 0;
        if (v == 0 && !sign_bit_set) || (v == -1 && sign_bit_set) {
            out.push(byte);
            return out;
        }
        out.push(byte | 0x80);
    }
}

/// WASM 值类型（MVP 子集，按需扩展）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ValType {
    I32,
    I64,
    F64,
}

impl ValType {
    fn encode(self) -> u8 {
        match self {
            ValType::I32 => 0x7F,
            ValType::I64 => 0x7E,
            ValType::F64 => 0x7C,
        }
    }
}

/// 本后端用到的 opcode 常量（按需添加，别预先铺满）
#[allow(dead_code)] // 7.1 只用一小部分，其余是 7.2+ 的储备
pub mod op {
    pub const UNREACHABLE: u8 = 0x00;
    pub const BLOCK: u8 = 0x02;
    pub const LOOP: u8 = 0x03;
    pub const IF: u8 = 0x04;
    pub const ELSE: u8 = 0x05;
    pub const END: u8 = 0x0B;
    pub const BR: u8 = 0x0C;
    pub const BR_IF: u8 = 0x0D;
    pub const RETURN: u8 = 0x0F;
    pub const CALL: u8 = 0x10;
    pub const CALL_INDIRECT: u8 = 0x11;
    pub const DROP: u8 = 0x1A;
    pub const LOCAL_GET: u8 = 0x20;
    pub const LOCAL_SET: u8 = 0x21;
    pub const LOCAL_TEE: u8 = 0x22;
    pub const GLOBAL_GET: u8 = 0x23;
    pub const GLOBAL_SET: u8 = 0x24;
    // 内存访问（memarg = align LEB + offset LEB；align 给 0 即 1 字节对齐，永远安全）
    pub const I32_LOAD: u8 = 0x28;
    pub const I64_LOAD: u8 = 0x29;
    pub const F64_LOAD: u8 = 0x2B;
    pub const I32_LOAD8_U: u8 = 0x2D;
    pub const I32_STORE: u8 = 0x36;
    pub const I64_STORE: u8 = 0x37;
    pub const F32_STORE: u8 = 0x38;
    pub const F64_STORE: u8 = 0x39;
    // 常量
    pub const I32_CONST: u8 = 0x41;
    pub const I64_CONST: u8 = 0x42;
    pub const F64_CONST: u8 = 0x44; // 后跟 8 字节小端 f64 位模式（非 LEB）
    // i32 比较/算术（heap 指针用）
    pub const I32_EQZ: u8 = 0x45;
    pub const I32_EQ: u8 = 0x46;
    pub const I32_NE: u8 = 0x47;
    pub const I32_LT_U: u8 = 0x49;
    pub const I32_GE_U: u8 = 0x4F;
    pub const I32_ADD: u8 = 0x6A;
    pub const I32_SUB: u8 = 0x6B;
    // i64 比较（结果 i32，可直接喂 if/br_if）
    pub const I64_EQZ: u8 = 0x50;
    pub const I64_EQ: u8 = 0x51;
    pub const I64_NE: u8 = 0x52;
    pub const I64_LT_S: u8 = 0x53;
    pub const I64_GT_S: u8 = 0x55;
    pub const I64_LE_S: u8 = 0x57;
    pub const I64_GE_S: u8 = 0x59;
    // f64 比较（结果 i32）
    pub const F64_EQ: u8 = 0x61;
    pub const F64_NE: u8 = 0x62;
    pub const F64_LT: u8 = 0x63;
    pub const F64_GT: u8 = 0x64;
    pub const F64_LE: u8 = 0x65;
    pub const F64_GE: u8 = 0x66;
    // i64 算术/位运算
    pub const I64_ADD: u8 = 0x7C;
    pub const I64_SUB: u8 = 0x7D;
    pub const I64_MUL: u8 = 0x7E;
    pub const I64_DIV_S: u8 = 0x7F;
    pub const I64_REM_S: u8 = 0x81;
    pub const I64_AND: u8 = 0x83;
    pub const I64_OR: u8 = 0x84;
    pub const I64_XOR: u8 = 0x85;
    pub const I64_SHL: u8 = 0x86;
    pub const I64_SHR_S: u8 = 0x87;
    pub const I64_SHR_U: u8 = 0x88;
    // f64 算术
    pub const F64_NEG: u8 = 0x9A;
    pub const F64_TRUNC: u8 = 0x9D; // f64 无取余指令：a % b = a - trunc(a/b) * b
    pub const F64_ADD: u8 = 0xA0;
    pub const F64_SUB: u8 = 0xA1;
    pub const F64_MUL: u8 = 0xA2;
    pub const F64_DIV: u8 = 0xA3;
    // 类型转换
    pub const I32_WRAP_I64: u8 = 0xA7;
    pub const I64_EXTEND_I32_S: u8 = 0xAC;
    pub const F64_CONVERT_I64_S: u8 = 0xB9;
}

/// 函数类型：(params) -> results
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FuncType {
    pub params: Vec<ValType>,
    pub results: Vec<ValType>,
}

impl FuncType {
    fn encode(&self, out: &mut Vec<u8>) {
        out.push(0x60); // functype 标记
        out.extend(leb_u(self.params.len() as u64));
        for p in &self.params {
            out.push(p.encode());
        }
        out.extend(leb_u(self.results.len() as u64));
        for r in &self.results {
            out.push(r.encode());
        }
    }
}

/// 导入项（7.1 只支持函数导入；全局/表/内存导入后续按需）
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Import {
    pub module: String,
    pub name: String,
    pub type_idx: u32,
}

/// 导出入口类别
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExportKind {
    Func,
    Memory,
}

impl ExportKind {
    fn encode(self) -> u8 {
        match self {
            ExportKind::Func => 0x00,
            ExportKind::Memory => 0x02,
        }
    }
}

/// 本地函数：类型索引 + 局部变量 + 已编码的指令字节流
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Function {
    pub type_idx: u32,
    /// 局部变量声明（不含参数）；WASM 按"同类型连排"分组编码
    pub locals: Vec<ValType>,
    /// 函数体指令字节（不含结尾的 0x0B，encode 时统一补）
    pub body: Vec<u8>,
}

/// 数据段：线性内存 offset 处的初始字节
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DataSegment {
    pub offset: u32,
    pub bytes: Vec<u8>,
}

/// 全局变量（7.2 起用于 bump allocator 堆指针）
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Global {
    pub ty: ValType,
    pub mutable: bool,
    /// 初始值（i32/i64 按 leb_s 编码，f64 按 8 字节小端）
    pub init: i64,
}

/// WASM 模块构建器
#[derive(Default)]
pub struct Module {
    pub types: Vec<FuncType>,
    pub imports: Vec<Import>,
    pub funcs: Vec<Function>,
    /// 线性内存下限页数（1 页 = 64KiB）；None = 无内存
    pub memory_min_pages: Option<u32>,
    pub globals: Vec<Global>,
    /// funcref 表（call_indirect 用；Phase 7.3 闭包）：(min, max)；None = 无表
    pub table: Option<(u32, Option<u32>)>,
    /// 主动元素段：table 0 从 offset 0 起填入这些 funcidx（闭包/函数值的调用目标）
    pub elems: Vec<u32>,
    pub exports: Vec<(String, ExportKind, u32)>,
    pub data: Vec<DataSegment>,
}

impl Module {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册函数类型，返回类型索引（去重：同型复用）
    pub fn add_type(&mut self, ty: FuncType) -> u32 {
        if let Some(i) = self.types.iter().position(|t| *t == ty) {
            return i as u32;
        }
        self.types.push(ty);
        (self.types.len() - 1) as u32
    }

    /// 函数索引 = 导入函数数 + 本地函数序号（WASM 函数索引空间导入在前）
    pub fn func_index_of_local(&self, local_idx: u32) -> u32 {
        self.imports.len() as u32 + local_idx
    }

    fn encode_str(s: &str, out: &mut Vec<u8>) {
        out.extend(leb_u(s.len() as u64));
        out.extend(s.as_bytes());
    }

    fn section(out: &mut Vec<u8>, id: u8, payload: &[u8]) {
        out.push(id);
        out.extend(leb_u(payload.len() as u64));
        out.extend(payload);
    }

    /// 编码整个模块为 WASM 二进制
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend([0x00, 0x61, 0x73, 0x6D]); // magic "\0asm"
        out.extend([0x01, 0x00, 0x00, 0x00]); // version 1

        // 1. type section
        if !self.types.is_empty() {
            let mut p = leb_u(self.types.len() as u64);
            for t in &self.types {
                t.encode(&mut p);
            }
            Self::section(&mut out, 1, &p);
        }

        // 2. import section
        if !self.imports.is_empty() {
            let mut p = leb_u(self.imports.len() as u64);
            for imp in &self.imports {
                Self::encode_str(&imp.module, &mut p);
                Self::encode_str(&imp.name, &mut p);
                p.push(0x00); // import kind: func
                p.extend(leb_u(imp.type_idx as u64));
            }
            Self::section(&mut out, 2, &p);
        }

        // 3. function section
        if !self.funcs.is_empty() {
            let mut p = leb_u(self.funcs.len() as u64);
            for f in &self.funcs {
                p.extend(leb_u(f.type_idx as u64));
            }
            Self::section(&mut out, 3, &p);
        }

        // 4. table section（funcref 表，call_indirect 用）
        if let Some((min, max)) = self.table {
            let mut p = leb_u(1);
            p.push(0x70); // elemtype: funcref
            match max {
                Some(mx) => {
                    p.push(0x01); // flags: 有 max
                    p.extend(leb_u(min as u64));
                    p.extend(leb_u(mx as u64));
                }
                None => {
                    p.push(0x00);
                    p.extend(leb_u(min as u64));
                }
            }
            Self::section(&mut out, 4, &p);
        }

        // 5. memory section（limits: flags=00, min=页数；首版不设上限）
        if let Some(min) = self.memory_min_pages {
            let mut p = leb_u(1);
            p.push(0x00); // flags: 无 max
            p.extend(leb_u(min as u64));
            Self::section(&mut out, 5, &p);
        }

        // 6. global section
        if !self.globals.is_empty() {
            let mut p = leb_u(self.globals.len() as u64);
            for g in &self.globals {
                p.push(g.ty.encode());
                p.push(if g.mutable { 0x01 } else { 0x00 });
                match g.ty {
                    ValType::I32 => {
                        p.push(op::I32_CONST);
                        p.extend(leb_s(g.init));
                    }
                    ValType::I64 => {
                        p.push(op::I64_CONST);
                        p.extend(leb_s(g.init));
                    }
                    ValType::F64 => {
                        p.push(op::F64_CONST);
                        p.extend(f64::from_bits(g.init as u64).to_le_bytes());
                    }
                }
                p.push(op::END);
            }
            Self::section(&mut out, 6, &p);
        }

        // 7. export section
        if !self.exports.is_empty() {
            let mut p = leb_u(self.exports.len() as u64);
            for (name, kind, idx) in &self.exports {
                Self::encode_str(name, &mut p);
                p.push(kind.encode());
                p.extend(leb_u(*idx as u64));
            }
            Self::section(&mut out, 7, &p);
        }

        // 9. element section（MVP 主动段：table 0, offset 0）
        if !self.elems.is_empty() {
            let mut p = leb_u(1); // 一个主动元素段
            p.push(0x00); // flags: active, table 0
            p.push(op::I32_CONST);
            p.extend(leb_s(0));
            p.push(op::END);
            p.extend(leb_u(self.elems.len() as u64));
            for f in &self.elems {
                p.extend(leb_u(*f as u64));
            }
            Self::section(&mut out, 9, &p);
        }

        // 10. code section
        if !self.funcs.is_empty() {
            let mut p = leb_u(self.funcs.len() as u64);
            for f in &self.funcs {
                // locals 分组编码：同类型连排合并
                let mut groups: Vec<(u32, ValType)> = Vec::new();
                for &l in &f.locals {
                    if let Some(last) = groups.last_mut() {
                        if last.1 == l {
                            last.0 += 1;
                            continue;
                        }
                    }
                    groups.push((1, l));
                }
                let mut body = leb_u(groups.len() as u64);
                for (count, ty) in &groups {
                    body.extend(leb_u(*count as u64));
                    body.push(ty.encode());
                }
                body.extend(&f.body);
                body.push(op::END);
                p.extend(leb_u(body.len() as u64));
                p.extend(body);
            }
            Self::section(&mut out, 10, &p);
        }

        // 11. data section（MVP active segment：flags=00 + offset 表达式）
        if !self.data.is_empty() {
            let mut p = leb_u(self.data.len() as u64);
            for seg in &self.data {
                p.push(0x00); // active, memory 0
                p.push(op::I32_CONST);
                p.extend(leb_s(seg.offset as i64));
                p.push(op::END);
                p.extend(leb_u(seg.bytes.len() as u64));
                p.extend(&seg.bytes);
            }
            Self::section(&mut out, 11, &p);
        }

        out
    }
}

/// 7.1 验收用最小模块：导入 env.lom_print(ptr, len)，
/// 导出 main 与 memory，main 把数据段里的 text 打出来。
pub fn hello_module(text: &str) -> Vec<u8> {
    let mut m = Module::new();
    let print_ty = m.add_type(FuncType {
        params: vec![ValType::I32, ValType::I32],
        results: vec![],
    });
    let main_ty = m.add_type(FuncType {
        params: vec![],
        results: vec![],
    });
    m.imports.push(Import {
        module: "env".to_string(),
        name: "lom_print".to_string(),
        type_idx: print_ty,
    });
    m.funcs.push(Function {
        type_idx: main_ty,
        locals: vec![],
        body: {
            let mut b = vec![op::I32_CONST];
            b.extend(leb_s(0)); // ptr = 0（数据段起点）
            b.push(op::I32_CONST);
            b.extend(leb_s(text.len() as i64)); // len
            b.push(op::CALL);
            b.extend(leb_u(0)); // funcidx 0 = 导入的 lom_print
            b
        },
    });
    m.memory_min_pages = Some(1);
    m.exports.push(("memory".to_string(), ExportKind::Memory, 0));
    let main_idx = m.func_index_of_local(0);
    m.exports.push(("main".to_string(), ExportKind::Func, main_idx));
    m.data.push(DataSegment {
        offset: 0,
        bytes: text.as_bytes().to_vec(),
    });
    m.encode()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leb_u_known_vectors() {
        // 规范里的经典测试向量
        assert_eq!(leb_u(0), [0x00]);
        assert_eq!(leb_u(1), [0x01]);
        assert_eq!(leb_u(127), [0x7F]);
        assert_eq!(leb_u(128), [0x80, 0x01]);
        assert_eq!(leb_u(624485), [0xE5, 0x8E, 0x26]);
        assert_eq!(leb_u(u64::MAX), [0xFF; 9].into_iter().chain([0x01]).collect::<Vec<u8>>());
    }

    #[test]
    fn leb_s_known_vectors() {
        assert_eq!(leb_s(0), [0x00]);
        assert_eq!(leb_s(1), [0x01]);
        assert_eq!(leb_s(-1), [0x7F]);
        assert_eq!(leb_s(63), [0x3F]);
        assert_eq!(leb_s(64), [0xC0, 0x00]); // 64 需要两字节（bit6 是符号位）
        assert_eq!(leb_s(-64), [0x40]);
        assert_eq!(leb_s(-123456), [0xC0, 0xBB, 0x78]);
    }

    #[test]
    fn hello_module_byte_golden() {
        // "hi" 的逐字节 golden——验证 section 顺序、长度前缀、函数索引空间（导入在前）
        let bytes = hello_module("hi");
        let expected: Vec<u8> = vec![
            0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, // magic + version
            // type section (id=1, len=9)：2 个类型
            0x01, 0x09, 0x02, //   count=2
            0x60, 0x02, 0x7F, 0x7F, 0x00, //   type0: (i32,i32)->()
            0x60, 0x00, 0x00, //             type1: ()->()
            // import section (id=2, len=17)：env.lom_print : type0
            0x02, 0x11, 0x01, 0x03, 0x65, 0x6E, 0x76, //   "env"
            0x09, 0x6C, 0x6F, 0x6D, 0x5F, 0x70, 0x72, 0x69, 0x6E, 0x74, // "lom_print"
            0x00, 0x00, //   kind=func, typeidx=0
            // function section (id=3, len=2)：1 个本地函数用 type1
            0x03, 0x02, 0x01, 0x01,
            // memory section (id=5, len=3)：min=1 页
            0x05, 0x03, 0x01, 0x00, 0x01,
            // export section (id=7, len=17)："memory"→mem0，"main"→func1（导入占 func0）
            0x07, 0x11, 0x02, 0x06, 0x6D, 0x65, 0x6D, 0x6F, 0x72, 0x79, 0x02, 0x00,
            0x04, 0x6D, 0x61, 0x69, 0x6E, 0x00, 0x01,
            // code section (id=10, len=10)：locals=0; i32.const 0; i32.const 2; call 0; end
            0x0A, 0x0A, 0x01, 0x08, 0x00, 0x41, 0x00, 0x41, 0x02, 0x10, 0x00, 0x0B,
            // data section (id=11, len=8)：offset 0, "hi"
            0x0B, 0x08, 0x01, 0x00, 0x41, 0x00, 0x0B, 0x02, 0x68, 0x69,
        ];
        assert_eq!(bytes, expected);
    }

    #[test]
    fn add_type_dedup() {
        let mut m = Module::new();
        let a = m.add_type(FuncType { params: vec![], results: vec![] });
        let b = m.add_type(FuncType { params: vec![ValType::I32], results: vec![] });
        let c = m.add_type(FuncType { params: vec![], results: vec![] });
        assert_eq!((a, b, c), (0, 1, 0));
    }

    #[test]
    fn func_index_space_imports_first() {
        let mut m = Module::new();
        let ty = m.add_type(FuncType { params: vec![], results: vec![] });
        m.imports.push(Import { module: "env".into(), name: "a".into(), type_idx: ty });
        m.imports.push(Import { module: "env".into(), name: "b".into(), type_idx: ty });
        m.funcs.push(Function { type_idx: ty, locals: vec![], body: vec![] });
        assert_eq!(m.func_index_of_local(0), 2); // 两个导入占 0/1
    }

    #[test]
    fn empty_body_still_ends() {
        // 空函数体也必须以 0x0B 结尾（code entry = locals_vec + body + END）
        let mut m = Module::new();
        let ty = m.add_type(FuncType { params: vec![], results: vec![] });
        m.funcs.push(Function { type_idx: ty, locals: vec![], body: vec![] });
        let bytes = m.encode();
        // code section 内容：count=1, body_len=2, locals=0, END
        let pos = bytes.windows(2).position(|w| w == [0x0A, 0x04]).expect("code section");
        assert_eq!(&bytes[pos..], &[0x0A, 0x04, 0x01, 0x02, 0x00, 0x0B]);
    }
}
