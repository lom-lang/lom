// src/typechecker/builtins.rs — 内建签名注册（Q2 拆分自 mod.rs，2026-09-08）
//
// Result/Option 内置枚举 + 43 个内建函数的签名表（register_builtins 整方法）。
// 数据源对账哨兵：tests.rs 的 export_builtin_table_for_selfhost 导出本表，
// 自举侧 reg_builtins（self_interp.lom）必须与其同步。

use crate::ast::{Span, Type};

use super::{EnumInfo, FnSig, TypeChecker};

impl TypeChecker {
    pub(crate) fn register_builtins(&mut self) {
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
        // v0.27.0: 码点→单字符 String（纯函数；无效码点是运行时错误，签名层不体现）
        self.functions.insert(
            "char_from_code".to_string(),
            FnSig { params: vec![("cp".to_string(), Type::Int)], ret: Some(Type::String), effects: vec![], span: Span::default() },
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
        // v0.4.3 Phase 5.9: 高阶 list 函数（f 用 Fn 标注,与闭包参数注解一致;实参为闭包/具名函数时是 Unknown,不触发 TYPE003）
        self.functions.insert(
            "list_map".to_string(),
            FnSig { params: vec![("f".to_string(), Type::Named("Fn".to_string())), ("list".to_string(), list_any())], ret: Some(list_any()), effects: vec![], span: Span::default() },
        );
        self.functions.insert(
            "list_filter".to_string(),
            FnSig { params: vec![("f".to_string(), Type::Named("Fn".to_string())), ("list".to_string(), list_any())], ret: Some(list_any()), effects: vec![], span: Span::default() },
        );
        self.functions.insert(
            "list_fold".to_string(),
            FnSig { params: vec![("f".to_string(), Type::Named("Fn".to_string())), ("init".to_string(), Type::Named("_Any".to_string())), ("list".to_string(), list_any())], ret: Some(Type::Named("_Any".to_string())), effects: vec![], span: Span::default() },
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
        // Phase 5.20: map 模块（map_set/map_remove 为引用语义就地修改，其余为只读查询）
        // Map 用 Type::Generic("Map", [_Any]) 表示；map_get 返回 Option<_Any>
        let map_any = || Type::Generic("Map".to_string(), vec![Type::Named("_Any".to_string())]);
        self.functions.insert(
            "map_empty".to_string(),
            FnSig { params: vec![], ret: Some(map_any()), effects: vec![], span: Span::default() },
        );
        self.functions.insert(
            "map_set".to_string(),
            FnSig { params: vec![("map".to_string(), map_any()), ("key".to_string(), Type::String), ("value".to_string(), Type::Named("_Any".to_string()))], ret: Some(Type::Unit), effects: vec![], span: Span::default() },
        );
        self.functions.insert(
            "map_get".to_string(),
            FnSig { params: vec![("map".to_string(), map_any()), ("key".to_string(), Type::String)], ret: Some(Type::Option(Box::new(Type::Named("_Any".to_string())))), effects: vec![], span: Span::default() },
        );
        self.functions.insert(
            "map_has".to_string(),
            FnSig { params: vec![("map".to_string(), map_any()), ("key".to_string(), Type::String)], ret: Some(Type::Bool), effects: vec![], span: Span::default() },
        );
        self.functions.insert(
            "map_remove".to_string(),
            FnSig { params: vec![("map".to_string(), map_any()), ("key".to_string(), Type::String)], ret: Some(Type::Bool), effects: vec![], span: Span::default() },
        );
        let list_any = || Type::Generic("List".to_string(), vec![Type::Named("_Any".to_string())]);
        self.functions.insert(
            "map_keys".to_string(),
            FnSig { params: vec![("map".to_string(), map_any())], ret: Some(list_any()), effects: vec![], span: Span::default() },
        );
        self.functions.insert(
            "map_values".to_string(),
            FnSig { params: vec![("map".to_string(), map_any())], ret: Some(list_any()), effects: vec![], span: Span::default() },
        );
        self.functions.insert(
            "map_size".to_string(),
            FnSig { params: vec![("map".to_string(), map_any())], ret: Some(Type::Int), effects: vec![], span: Span::default() },
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

}
