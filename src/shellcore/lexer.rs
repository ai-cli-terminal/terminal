//! 렉서: 소스를 토큰으로. 바레워드는 숫자/키워드 판별; =·:·,·파이프·괄호류는 전용 토큰.
//! (S0 한계: 외부 인자에 = 포함 시 따옴표 필요 — `"--k=v"`. URL 등 : 포함도 따옴표.)

use anyhow::{bail, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Word(String),
    Int(i64),
    Float(f64),
    Str(String),
    Var(String),
    Pipe,
    Semicolon,
    Newline,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    LParen,
    RParen,
    Colon,
    Comma,
    Equals,
    Let,
    True,
    False,
    Null,
    EqEq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Not,
}

const SPECIAL: &[char] = &[
    '|', ';', '[', ']', '{', '}', '(', ')', ':', ',', '=', '#', '"', '\'', '$', '!', '<', '>',
];

pub fn lex(src: &str) -> Result<Vec<Token>> {
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' | '\r' => {
                i += 1;
            }
            '\n' => {
                out.push(Token::Newline);
                i += 1;
            }
            '#' => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '|' => {
                out.push(Token::Pipe);
                i += 1;
            }
            ';' => {
                out.push(Token::Semicolon);
                i += 1;
            }
            '[' => {
                out.push(Token::LBracket);
                i += 1;
            }
            ']' => {
                out.push(Token::RBracket);
                i += 1;
            }
            '{' => {
                out.push(Token::LBrace);
                i += 1;
            }
            '}' => {
                out.push(Token::RBrace);
                i += 1;
            }
            '(' => {
                out.push(Token::LParen);
                i += 1;
            }
            ')' => {
                out.push(Token::RParen);
                i += 1;
            }
            ':' => {
                out.push(Token::Colon);
                i += 1;
            }
            ',' => {
                out.push(Token::Comma);
                i += 1;
            }
            '=' => {
                if chars.get(i + 1) == Some(&'=') {
                    out.push(Token::EqEq);
                    i += 2;
                } else {
                    out.push(Token::Equals);
                    i += 1;
                }
            }
            '!' => {
                if chars.get(i + 1) == Some(&'=') {
                    out.push(Token::NotEq);
                    i += 2;
                } else {
                    bail!("예상치 못한 '!' (부정은 `not` 키워드)");
                }
            }
            '<' => {
                if chars.get(i + 1) == Some(&'=') {
                    out.push(Token::Le);
                    i += 2;
                } else {
                    out.push(Token::Lt);
                    i += 1;
                }
            }
            '>' => {
                if chars.get(i + 1) == Some(&'=') {
                    out.push(Token::Ge);
                    i += 2;
                } else {
                    out.push(Token::Gt);
                    i += 1;
                }
            }
            '"' | '\'' => {
                let quote = c;
                i += 1;
                let start = i;
                while i < chars.len() && chars[i] != quote {
                    i += 1;
                }
                if i >= chars.len() {
                    bail!("닫히지 않은 문자열");
                }
                let s: String = chars[start..i].iter().collect();
                out.push(Token::Str(s));
                i += 1; // 닫는 따옴표
            }
            '$' => {
                i += 1;
                let start = i;
                while i < chars.len() && is_word_char(chars[i]) {
                    i += 1;
                }
                if start == i {
                    bail!("$ 뒤에 변수 이름이 필요합니다");
                }
                let name: String = chars[start..i].iter().collect();
                out.push(Token::Var(name));
            }
            _ => {
                let start = i;
                while i < chars.len() && !chars[i].is_whitespace() && !SPECIAL.contains(&chars[i]) {
                    i += 1;
                }
                let w: String = chars[start..i].iter().collect();
                out.push(classify_word(w));
            }
        }
    }
    Ok(out)
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn classify_word(w: String) -> Token {
    match w.as_str() {
        "let" => return Token::Let,
        "true" => return Token::True,
        "false" => return Token::False,
        "null" => return Token::Null,
        "and" => return Token::And,
        "or" => return Token::Or,
        "not" => return Token::Not,
        _ => {}
    }
    if let Ok(n) = w.parse::<i64>() {
        return Token::Int(n);
    }
    if w.matches('.').count() == 1 {
        if let Ok(f) = w.parse::<f64>() {
            return Token::Float(f);
        }
    }
    Token::Word(w)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_pipeline_and_args() {
        let t = lex("ls | get name | first 3").unwrap();
        assert_eq!(
            t,
            vec![
                Token::Word("ls".into()),
                Token::Pipe,
                Token::Word("get".into()),
                Token::Word("name".into()),
                Token::Pipe,
                Token::Word("first".into()),
                Token::Int(3),
            ]
        );
    }

    #[test]
    fn tokenizes_let_var_and_literals() {
        let t = lex("let x = 3.5").unwrap();
        assert_eq!(
            t,
            vec![
                Token::Let,
                Token::Word("x".into()),
                Token::Equals,
                Token::Float(3.5)
            ]
        );
        assert_eq!(lex("$y").unwrap(), vec![Token::Var("y".into())]);
        assert_eq!(
            lex("true false null").unwrap(),
            vec![Token::True, Token::False, Token::Null]
        );
        assert_eq!(
            lex("\"hi there\"").unwrap(),
            vec![Token::Str("hi there".into())]
        );
    }

    #[test]
    fn list_record_and_comment() {
        assert_eq!(
            lex("[1 2]").unwrap(),
            vec![
                Token::LBracket,
                Token::Int(1),
                Token::Int(2),
                Token::RBracket
            ]
        );
        assert_eq!(
            lex("{a: 1}").unwrap(),
            vec![
                Token::LBrace,
                Token::Word("a".into()),
                Token::Colon,
                Token::Int(1),
                Token::RBrace
            ]
        );
        assert_eq!(lex("ls # comment").unwrap(), vec![Token::Word("ls".into())]);
    }

    #[test]
    fn tokenizes_comparison_and_boolean() {
        use Token::*;
        assert_eq!(lex("size > 100").unwrap(), vec![Word("size".into()), Gt, Int(100)]);
        assert_eq!(
            lex("a == 1 and b != 2 or not c").unwrap(),
            vec![Word("a".into()), EqEq, Int(1), And, Word("b".into()), NotEq, Int(2), Or, Not, Word("c".into())]
        );
        assert_eq!(lex("x <= 1 >= y < z >").unwrap(), vec![
            Word("x".into()), Le, Int(1), Ge, Word("y".into()), Lt, Word("z".into()), Gt
        ]);
    }

    #[test]
    fn equals_vs_eqeq_and_paths_unaffected() {
        use Token::*;
        assert_eq!(lex("let x = 1").unwrap(), vec![Let, Word("x".into()), Equals, Int(1)]);
        assert_eq!(lex("ls -rf ./src").unwrap(), vec![Word("ls".into()), Word("-rf".into()), Word("./src".into())]);
        assert_eq!(lex("3.5").unwrap(), vec![Float(3.5)]);
    }

    #[test]
    fn path_like_word_and_newline() {
        assert_eq!(
            lex("cd ./src").unwrap(),
            vec![Token::Word("cd".into()), Token::Word("./src".into())]
        );
        assert_eq!(
            lex("a\nb").unwrap(),
            vec![
                Token::Word("a".into()),
                Token::Newline,
                Token::Word("b".into())
            ]
        );
    }
}
