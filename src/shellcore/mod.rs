//! 독립 셸 코어(S0): 값 모델·렉서·파서·평가기·빌트인·외부 실행·포매터·REPL.

pub mod ast;
pub mod builtins;
pub mod engine;
pub mod external;
pub mod format;
pub mod lexer;
pub mod ops;
pub mod parser;
pub mod repl;
pub mod util;
pub mod value;
