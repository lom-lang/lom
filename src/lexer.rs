// Lom Lexer — Phase 1 minimal implementation
// 词法分析器：将源代码字符串转换为 Token 流
// Phase 1 手写实现（零依赖），Phase 2 评估迁移到 logos

use std::fmt;

/// 词法 token 类型
/// 注意：包含 Phase 2 的 token（|> ? => 等），前向兼容，Phase 1 parser 只用子集
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // 关键字
    Fn,
    Let,
    Mut,
    If,
    Elif,
    Else,
    While,
    For,
    In,
    Return,
    End,
    True,
    False,
    And,
    Or,
    From, // Phase 2.1.5 显式导入
    Import, // Phase 2.1.5 显式导入
    As,    // Phase 2.1.5 导入别名
    Match, // Phase 2
    Enum,  // Phase 2

    // 字面量
    Int(i64),
    Float(f64),
    Str(String),

    // 标识符（类型名 Int/Float/Bool/String/Unit 也作为 Ident 处理）
    Ident(String),

    // 运算符
    Plus,      // +
    Minus,     // -
    Star,      // *
    Slash,     // /
    Percent,   // %
    PlusEq,    // += (v0.4.1 P0-3 复合赋值)
    MinusEq,   // -=
    StarEq,    // *=
    SlashEq,   // /=
    Eq,        // ==
    NotEq,     // !=
    Lt,        // <
    Gt,        // >
    LtEq,      // <=
    GtEq,      // >=
    Assign,    // =
    Bang,      // !
    Question,  // ? (Phase 2)
    Pipe,      // |> (Phase 2)
    Bar,       // | (Phase 2, enum 变体分隔)
    Arrow,     // -> (闭包返回类型)
    FatArrow,  // => (Phase 2 match)

    // 标点
    LParen,    // (
    RParen,    // )
    LBrace,    // { (Phase 2 记录)
    RBrace,    // }
    LBracket,  // [
    RBracket,  // ]
    Comma,     // ,
    Colon,     // :
    Dot,       // .
    Semi,      // ; (保留，Phase 1 不使用但 lexer 识别)

    // 结束
    Eof,
}

/// 带位置信息的 token
#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    pub token: Token,
    pub line: usize,
    pub col: usize,
}

/// 词法错误
#[derive(Debug)]
pub struct LexError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "词法错误 ({}:{}): {}", self.line, self.col, self.message)
    }
}

impl std::error::Error for LexError {}

/// 词法分析器
pub struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Lexer {
            src: src.as_bytes(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    /// 词法分析入口：返回所有 token（含 Eof）
    pub fn tokenize(mut self) -> Result<Vec<SpannedToken>, LexError> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            if self.is_at_end() {
                tokens.push(SpannedToken {
                    token: Token::Eof,
                    line: self.line,
                    col: self.col,
                });
                break;
            }
            let tok = self.next_token()?;
            tokens.push(tok);
        }
        Ok(tokens)
    }

    /// 容错词法分析（Phase 2.2）：遇到词法错误不终止，记录错误并跳过坏字节继续。
    /// 返回 (tokens, errors)。tokens 始终以 Eof 结尾，可用于后续解析。
    /// 与 tokenize() 的区别：tokenize 遇到第一个词法错误即返回 Err；
    /// tokenize_recover 收集所有词法错误，仍产出尽可能完整的 token 流。
    pub fn tokenize_recover(mut self) -> (Vec<SpannedToken>, Vec<LexError>) {
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            if self.is_at_end() {
                tokens.push(SpannedToken {
                    token: Token::Eof,
                    line: self.line,
                    col: self.col,
                });
                break;
            }
            // 保存进入 next_token 前的位置，用于错误后回退并跳过坏字节
            let saved_pos = self.pos;
            let saved_line = self.line;
            let saved_col = self.col;
            match self.next_token() {
                Ok(tok) => tokens.push(tok),
                Err(e) => {
                    errors.push(e);
                    // 回退到 next_token 调用前的状态，再前进一字节，保证进度
                    self.pos = saved_pos;
                    self.line = saved_line;
                    self.col = saved_col;
                    self.advance();
                }
            }
        }
        (tokens, errors)
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.src.len()
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<u8> {
        self.src.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let c = self.peek();
        if let Some(ch) = c {
            self.pos += 1;
            if ch == b'\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        c
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(b' ') | Some(b'\t') | Some(b'\r') | Some(b'\n') => {
                    self.advance();
                }
                Some(b'#') => {
                    // 注释：# 行注释 或 #- 块注释 -#
                    if self.peek_next() == Some(b'-') {
                        // 块注释 #- ... -#
                        self.advance(); // #
                        self.advance(); // -
                        self.skip_block_comment();
                    } else {
                        // 行注释
                        while let Some(c) = self.peek() {
                            if c == b'\n' {
                                break;
                            }
                            self.advance();
                        }
                    }
                }
                _ => break,
            }
        }
    }

    fn skip_block_comment(&mut self) {
        // 已消费 #-，现在找 -#
        loop {
            if self.is_at_end() {
                break; // 未闭合的块注释，静默结束
            }
            if self.peek() == Some(b'-') && self.peek_next() == Some(b'#') {
                self.advance(); // -
                self.advance(); // #
                break;
            }
            self.advance();
        }
    }

    fn next_token(&mut self) -> Result<SpannedToken, LexError> {
        let line = self.line;
        let col = self.col;
        let c = self.peek().unwrap();

        // 数字字面量
        if c.is_ascii_digit() {
            return self.lex_number(line, col);
        }

        // 字符串字面量
        if c == b'"' {
            return self.lex_string(line, col);
        }

        // 标识符 / 关键字
        if c.is_ascii_alphabetic() || c == b'_' {
            return self.lex_ident(line, col);
        }

        // 运算符和标点
        let tok = match c {
            b'+' => {
                self.advance();
                if self.peek() == Some(b'=') {
                    self.advance();
                    return Ok(SpannedToken {
                        token: Token::PlusEq,
                        line,
                        col,
                    });
                }
                return Ok(SpannedToken {
                    token: Token::Plus,
                    line,
                    col,
                });
            }
            b'-' => {
                self.advance();
                if self.peek() == Some(b'>') {
                    self.advance();
                    return Ok(SpannedToken {
                        token: Token::Arrow,
                        line,
                        col,
                    });
                }
                if self.peek() == Some(b'=') {
                    self.advance();
                    return Ok(SpannedToken {
                        token: Token::MinusEq,
                        line,
                        col,
                    });
                }
                return Ok(SpannedToken {
                    token: Token::Minus,
                    line,
                    col,
                });
            }
            b'*' => {
                self.advance();
                if self.peek() == Some(b'=') {
                    self.advance();
                    return Ok(SpannedToken {
                        token: Token::StarEq,
                        line,
                        col,
                    });
                }
                return Ok(SpannedToken {
                    token: Token::Star,
                    line,
                    col,
                });
            }
            b'/' => {
                self.advance();
                if self.peek() == Some(b'=') {
                    self.advance();
                    return Ok(SpannedToken {
                        token: Token::SlashEq,
                        line,
                        col,
                    });
                }
                return Ok(SpannedToken {
                    token: Token::Slash,
                    line,
                    col,
                });
            }
            b'%' => Token::Percent,
            b'!' => {
                self.advance();
                if self.peek() == Some(b'=') {
                    self.advance();
                    return Ok(SpannedToken {
                        token: Token::NotEq,
                        line,
                        col,
                    });
                }
                return Ok(SpannedToken {
                    token: Token::Bang,
                    line,
                    col,
                });
            }
            b'=' => {
                self.advance();
                if self.peek() == Some(b'=') {
                    self.advance();
                    return Ok(SpannedToken {
                        token: Token::Eq,
                        line,
                        col,
                    });
                }
                if self.peek() == Some(b'>') {
                    self.advance();
                    return Ok(SpannedToken {
                        token: Token::FatArrow,
                        line,
                        col,
                    });
                }
                return Ok(SpannedToken {
                    token: Token::Assign,
                    line,
                    col,
                });
            }
            b'<' => {
                self.advance();
                if self.peek() == Some(b'=') {
                    self.advance();
                    return Ok(SpannedToken {
                        token: Token::LtEq,
                        line,
                        col,
                    });
                }
                return Ok(SpannedToken {
                    token: Token::Lt,
                    line,
                    col,
                });
            }
            b'>' => {
                self.advance();
                if self.peek() == Some(b'=') {
                    self.advance();
                    return Ok(SpannedToken {
                        token: Token::GtEq,
                        line,
                        col,
                    });
                }
                return Ok(SpannedToken {
                    token: Token::Gt,
                    line,
                    col,
                });
            }
            b'|' => {
                self.advance();
                if self.peek() == Some(b'>') {
                    self.advance();
                    return Ok(SpannedToken {
                        token: Token::Pipe,
                        line,
                        col,
                    });
                }
                return Ok(SpannedToken {
                    token: Token::Bar,
                    line,
                    col,
                });
            }
            b'?' => Token::Question,
            b'(' => Token::LParen,
            b')' => Token::RParen,
            b'{' => Token::LBrace,
            b'}' => Token::RBrace,
            b'[' => Token::LBracket,
            b']' => Token::RBracket,
            b',' => Token::Comma,
            b':' => Token::Colon,
            b'.' => Token::Dot,
            b';' => Token::Semi,
            _ => {
                return Err(LexError {
                    message: format!("意外字符 '{}'", c as char),
                    line,
                    col,
                });
            }
        };
        self.advance();
        Ok(SpannedToken {
            token: tok,
            line,
            col,
        })
    }

    fn lex_number(&mut self, line: usize, col: usize) -> Result<SpannedToken, LexError> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }
        // 浮点数
        if self.peek() == Some(b'.') && self.peek_next().map_or(false, |c| c.is_ascii_digit()) {
            self.advance(); // .
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }
            let s = std::str::from_utf8(&self.src[start..self.pos]).unwrap();
            let val: f64 = s.parse().map_err(|_| LexError {
                message: format!("无效浮点数: {}", s),
                line,
                col,
            })?;
            return Ok(SpannedToken {
                token: Token::Float(val),
                line,
                col,
            });
        }
        let s = std::str::from_utf8(&self.src[start..self.pos]).unwrap();
        let val: i64 = s.parse().map_err(|_| LexError {
            message: format!("无效整数: {}", s),
            line,
            col,
        })?;
        Ok(SpannedToken {
            token: Token::Int(val),
            line,
            col,
        })
    }

    fn lex_string(&mut self, line: usize, col: usize) -> Result<SpannedToken, LexError> {
        self.advance(); // 开头 "
        let mut s = String::new();
        loop {
            match self.peek() {
                None => {
                    return Err(LexError {
                        message: "未闭合的字符串".to_string(),
                        line,
                        col,
                    });
                }
                Some(b'"') => {
                    self.advance();
                    break;
                }
                Some(b'\\') => {
                    self.advance();
                    match self.peek() {
                        Some(b'n') => s.push('\n'),
                        Some(b't') => s.push('\t'),
                        Some(b'r') => s.push('\r'),
                        Some(b'"') => s.push('"'),
                        Some(b'\\') => s.push('\\'),
                        Some(c) => s.push(c as char),
                        None => {
                            return Err(LexError {
                                message: "未闭合的字符串转义".to_string(),
                                line,
                                col,
                            });
                        }
                    }
                    self.advance();
                }
                Some(c) => {
                    s.push(c as char);
                    self.advance();
                }
            }
        }
        Ok(SpannedToken {
            token: Token::Str(s),
            line,
            col,
        })
    }

    fn lex_ident(&mut self, line: usize, col: usize) -> Result<SpannedToken, LexError> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == b'_' {
                self.advance();
            } else {
                break;
            }
        }
        let s = std::str::from_utf8(&self.src[start..self.pos]).unwrap();
        let token = match s {
            "fn" => Token::Fn,
            "let" => Token::Let,
            "mut" => Token::Mut,
            "if" => Token::If,
            "elif" => Token::Elif,
            "else" => Token::Else,
            "while" => Token::While,
            "for" => Token::For,
            "in" => Token::In,
            "return" => Token::Return,
            "end" => Token::End,
            "True" => Token::True,
            "False" => Token::False,
            "and" => Token::And,
            "or" => Token::Or,
            "from" => Token::From,
            "import" => Token::Import,
            "as" => Token::As,
            "match" => Token::Match,
            "enum" => Token::Enum,
            _ => Token::Ident(s.to_string()),
        };
        Ok(SpannedToken {
            token,
            line,
            col,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let src = "fn add(x: Int, y: Int) -> Int\n    x + y\nend";
        let tokens = Lexer::new(src).tokenize().unwrap();
        assert_eq!(tokens[0].token, Token::Fn);
        assert!(matches!(tokens[1].token, Token::Ident(ref s) if s == "add"));
        assert_eq!(tokens[2].token, Token::LParen);
    }

    #[test]
    fn test_operators() {
        let src = "1 + 2 == 3";
        let tokens = Lexer::new(src).tokenize().unwrap();
        assert_eq!(tokens[0].token, Token::Int(1));
        assert_eq!(tokens[1].token, Token::Plus);
        assert_eq!(tokens[2].token, Token::Int(2));
        assert_eq!(tokens[3].token, Token::Eq);
        assert_eq!(tokens[4].token, Token::Int(3));
    }

    #[test]
    fn test_compound_assign_tokens() {
        // v0.4.1 P0-3: += -= *= /= 是独立 token(最长匹配)
        let src = "x += 1\nx -= 2\nx *= 3\nx /= 4";
        let tokens = Lexer::new(src).tokenize().unwrap();
        assert_eq!(tokens[1].token, Token::PlusEq);
        assert_eq!(tokens[4].token, Token::MinusEq);
        assert_eq!(tokens[7].token, Token::StarEq);
        assert_eq!(tokens[10].token, Token::SlashEq);
    }

    #[test]
    fn test_arrow_minus_eq_distinct() {
        // -> 与 -= 不混淆;单独 - 仍是 Minus
        // token 序列:fn f ( ) -> Int(标识符) 1 - 2 end
        let src = "fn f() -> Int\n    1 - 2\nend";
        let tokens = Lexer::new(src).tokenize().unwrap();
        assert_eq!(tokens[4].token, Token::Arrow);
        assert_eq!(tokens[7].token, Token::Minus);
    }

    #[test]
    fn test_string() {
        let src = "\"hello\\nworld\"";
        let tokens = Lexer::new(src).tokenize().unwrap();
        assert_eq!(tokens[0].token, Token::Str("hello\nworld".to_string()));
    }

    #[test]
    fn test_float() {
        let src = "3.14";
        let tokens = Lexer::new(src).tokenize().unwrap();
        assert_eq!(tokens[0].token, Token::Float(3.14));
    }

    #[test]
    fn test_comments() {
        let src = "# 注释\nlet x = 1 #- 块注释 -# + 2";
        let tokens = Lexer::new(src).tokenize().unwrap();
        assert_eq!(tokens[0].token, Token::Let);
        assert!(matches!(tokens[1].token, Token::Ident(ref s) if s == "x"));
        assert_eq!(tokens[2].token, Token::Assign);
        assert_eq!(tokens[3].token, Token::Int(1));
        assert_eq!(tokens[4].token, Token::Plus);
        assert_eq!(tokens[5].token, Token::Int(2));
    }
}
