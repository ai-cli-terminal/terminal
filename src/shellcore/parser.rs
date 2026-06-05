//! 파서: 토큰 → AST. 스테이지 선두가 Word면 Command(이름+인자), 아니면 Expr.

use anyhow::{bail, Result};

use crate::shellcore::ast::*;
use crate::shellcore::lexer::Token;

pub fn parse(tokens: Vec<Token>) -> Result<Vec<Stmt>> {
    let mut p = Parser {
        toks: tokens,
        pos: 0,
    };
    let mut stmts = Vec::new();
    p.skip_separators();
    while p.peek().is_some() {
        stmts.push(p.parse_stmt()?);
        p.skip_separators();
    }
    Ok(stmts)
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.toks.get(self.pos)
    }
    fn next(&mut self) -> Option<Token> {
        let t = self.toks.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }
    fn skip_separators(&mut self) {
        while matches!(self.peek(), Some(Token::Newline) | Some(Token::Semicolon)) {
            self.pos += 1;
        }
    }
    fn at_stage_end(&self) -> bool {
        matches!(
            self.peek(),
            None | Some(Token::Pipe)
                | Some(Token::Newline)
                | Some(Token::Semicolon)
                | Some(Token::RBracket)
                | Some(Token::RBrace)
                | Some(Token::RParen)
        )
    }

    fn parse_stmt(&mut self) -> Result<Stmt> {
        if matches!(self.peek(), Some(Token::Let)) {
            self.next();
            let name = match self.next() {
                Some(Token::Word(w)) => w,
                other => bail!("let: 변수 이름 기대, got {other:?}"),
            };
            match self.next() {
                Some(Token::Equals) => {}
                other => bail!("let: '=' 기대, got {other:?}"),
            }
            let value = self.parse_pipeline()?;
            return Ok(Stmt::Let { name, value });
        }
        Ok(Stmt::Pipeline(self.parse_pipeline()?))
    }

    fn parse_pipeline(&mut self) -> Result<Pipeline> {
        let mut stages = vec![self.parse_stage()?];
        while matches!(self.peek(), Some(Token::Pipe)) {
            self.next();
            stages.push(self.parse_stage()?);
        }
        Ok(Pipeline { stages })
    }

    fn parse_stage(&mut self) -> Result<Stage> {
        let is_where = matches!(self.peek(), Some(Token::Word(w)) if w == "where");
        if is_where {
            self.next();
            return Ok(Stage::Where(self.parse_expr()?));
        }
        if let Some(Token::Word(_)) = self.peek() {
            let name = match self.next() {
                Some(Token::Word(w)) => w,
                _ => unreachable!(),
            };
            let mut args = Vec::new();
            while !self.at_stage_end() {
                args.push(self.parse_atom()?);
            }
            Ok(Stage::Command(Command { name, args }))
        } else {
            Ok(Stage::Expr(self.parse_expr()?))
        }
    }

    fn parse_expr(&mut self) -> Result<Expr> {
        self.parse_or()
    }
    fn parse_or(&mut self) -> Result<Expr> {
        let mut lhs = self.parse_and()?;
        while matches!(self.peek(), Some(Token::Or)) {
            self.next();
            let rhs = self.parse_and()?;
            lhs = Expr::Binary {
                op: BinOp::Or,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }
    fn parse_and(&mut self) -> Result<Expr> {
        let mut lhs = self.parse_not()?;
        while matches!(self.peek(), Some(Token::And)) {
            self.next();
            let rhs = self.parse_not()?;
            lhs = Expr::Binary {
                op: BinOp::And,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }
    fn parse_not(&mut self) -> Result<Expr> {
        if matches!(self.peek(), Some(Token::Not)) {
            self.next();
            let expr = self.parse_not()?;
            Ok(Expr::Unary {
                op: UnOp::Not,
                expr: Box::new(expr),
            })
        } else {
            self.parse_cmp()
        }
    }
    fn parse_cmp(&mut self) -> Result<Expr> {
        let mut lhs = self.parse_atom()?;
        while let Some(op) = self.peek().and_then(cmp_op) {
            self.next();
            let rhs = self.parse_atom()?;
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_atom(&mut self) -> Result<Expr> {
        match self.next() {
            Some(Token::Int(n)) => Ok(Expr::Int(n)),
            Some(Token::Float(f)) => Ok(Expr::Float(f)),
            Some(Token::Str(s)) => Ok(Expr::Str(s)),
            Some(Token::True) => Ok(Expr::Bool(true)),
            Some(Token::False) => Ok(Expr::Bool(false)),
            Some(Token::Null) => Ok(Expr::Null),
            Some(Token::Var(v)) => Ok(Expr::Var(v)),
            Some(Token::Word(w)) => Ok(Expr::Word(w)),
            Some(Token::LBracket) => self.parse_list(),
            Some(Token::LBrace) => self.parse_record(),
            Some(Token::LParen) => {
                let e = self.parse_expr()?;
                match self.next() {
                    Some(Token::RParen) => Ok(e),
                    other => bail!("')' 기대, got {other:?}"),
                }
            }
            other => bail!("표현식 기대, got {other:?}"),
        }
    }

    fn parse_list(&mut self) -> Result<Expr> {
        let mut items = Vec::new();
        while !matches!(self.peek(), Some(Token::RBracket) | None) {
            items.push(self.parse_atom()?);
            if matches!(self.peek(), Some(Token::Comma)) {
                self.next();
            }
        }
        match self.next() {
            Some(Token::RBracket) => Ok(Expr::List(items)),
            other => bail!("']' 기대, got {other:?}"),
        }
    }

    fn parse_record(&mut self) -> Result<Expr> {
        let mut pairs = Vec::new();
        while !matches!(self.peek(), Some(Token::RBrace) | None) {
            let key = match self.next() {
                Some(Token::Word(w)) => w,
                Some(Token::Str(s)) => s,
                other => bail!("레코드 키(이름) 기대, got {other:?}"),
            };
            match self.next() {
                Some(Token::Colon) => {}
                other => bail!("레코드 ':' 기대, got {other:?}"),
            }
            let val = self.parse_atom()?;
            pairs.push((key, val));
            if matches!(self.peek(), Some(Token::Comma)) {
                self.next();
            }
        }
        match self.next() {
            Some(Token::RBrace) => Ok(Expr::Record(pairs)),
            other => bail!("'}}' 기대, got {other:?}"),
        }
    }
}

fn cmp_op(t: &Token) -> Option<BinOp> {
    Some(match t {
        Token::EqEq => BinOp::Eq,
        Token::NotEq => BinOp::Ne,
        Token::Lt => BinOp::Lt,
        Token::Le => BinOp::Le,
        Token::Gt => BinOp::Gt,
        Token::Ge => BinOp::Ge,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*; // ast::* 포함(파서가 glob import) — 중복 import 금지(clippy)
    use crate::shellcore::lexer::lex;

    fn p(src: &str) -> Vec<Stmt> {
        parse(lex(src).unwrap()).unwrap()
    }

    #[test]
    fn parses_pipeline_of_commands() {
        let stmts = p("ls | get name | first 3");
        assert_eq!(stmts.len(), 1);
        let Stmt::Pipeline(pl) = &stmts[0] else {
            panic!("pipeline 기대")
        };
        assert_eq!(pl.stages.len(), 3);
        let Stage::Command(c0) = &pl.stages[0] else {
            panic!()
        };
        assert_eq!(c0.name, "ls");
        assert!(c0.args.is_empty());
        let Stage::Command(c1) = &pl.stages[1] else {
            panic!()
        };
        assert_eq!(c1.name, "get");
        assert_eq!(c1.args, vec![Expr::Word("name".into())]);
        let Stage::Command(c2) = &pl.stages[2] else {
            panic!()
        };
        assert_eq!(c2.args, vec![Expr::Int(3)]);
    }

    #[test]
    fn parses_let_and_leading_expr() {
        let stmts = p("let x = 5");
        assert_eq!(
            stmts[0],
            Stmt::Let {
                name: "x".into(),
                value: Pipeline {
                    stages: vec![Stage::Expr(Expr::Int(5))]
                }
            }
        );
        let stmts = p("$x");
        let Stmt::Pipeline(pl) = &stmts[0] else {
            panic!()
        };
        assert_eq!(pl.stages[0], Stage::Expr(Expr::Var("x".into())));
    }

    #[test]
    fn parses_list_and_record_literals() {
        let stmts = p("[1 2]");
        let Stmt::Pipeline(pl) = &stmts[0] else {
            panic!()
        };
        assert_eq!(
            pl.stages[0],
            Stage::Expr(Expr::List(vec![Expr::Int(1), Expr::Int(2)]))
        );
        let stmts = p("{a: 1, b: hi}");
        let Stmt::Pipeline(pl) = &stmts[0] else {
            panic!()
        };
        assert_eq!(
            pl.stages[0],
            Stage::Expr(Expr::Record(vec![
                ("a".into(), Expr::Int(1)),
                ("b".into(), Expr::Word("hi".into()))
            ]))
        );
    }

    #[test]
    fn multiple_statements_split_by_newline_and_semicolon() {
        assert_eq!(p("print 1; print 2").len(), 2);
        assert_eq!(p("print 1\nprint 2").len(), 2);
    }

    #[test]
    fn parses_where_and_precedence() {
        let stmts = p("ls | where size > 100");
        let Stmt::Pipeline(pl) = &stmts[0] else {
            panic!()
        };
        assert_eq!(pl.stages.len(), 2);
        let Stage::Where(cond) = &pl.stages[1] else {
            panic!("where 기대")
        };
        assert_eq!(
            *cond,
            Expr::Binary {
                op: BinOp::Gt,
                lhs: Box::new(Expr::Word("size".into())),
                rhs: Box::new(Expr::Int(100)),
            }
        );
        let stmts = p("where a == 1 and b == 2");
        let Stmt::Pipeline(pl) = &stmts[0] else {
            panic!()
        };
        let Stage::Where(c) = &pl.stages[0] else {
            panic!()
        };
        assert!(matches!(c, Expr::Binary { op: BinOp::And, .. }));
        let stmts = p("where not a == b");
        let Stmt::Pipeline(pl) = &stmts[0] else {
            panic!()
        };
        let Stage::Where(c) = &pl.stages[0] else {
            panic!()
        };
        assert!(
            matches!(c, Expr::Unary { op: UnOp::Not, expr } if matches!(**expr, Expr::Binary { op: BinOp::Eq, .. }))
        );
    }

    #[test]
    fn command_args_have_no_operators() {
        let stmts = p("ls -rf");
        let Stmt::Pipeline(pl) = &stmts[0] else {
            panic!()
        };
        let Stage::Command(c) = &pl.stages[0] else {
            panic!()
        };
        assert_eq!(c.name, "ls");
        assert_eq!(c.args, vec![Expr::Word("-rf".into())]);
    }
}
