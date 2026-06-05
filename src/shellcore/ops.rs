//! 연산자 의미: 비교(==,!=,<,<=,>,>=) 적용 + 불리언 강제. 순수 함수.
//! and/or 단축평가·not 은 엔진에서 처리(여기선 as_bool 제공).

use std::cmp::Ordering;

use anyhow::{bail, Result};

use crate::shellcore::ast::BinOp;
use crate::shellcore::value::Value;

/// 비교 연산(Eq/Ne/Lt/Le/Gt/Ge)을 적용해 Bool 을 반환한다.
pub fn apply_compare(op: BinOp, lhs: &Value, rhs: &Value) -> Result<Value> {
    let b = match op {
        BinOp::Eq => values_equal(lhs, rhs),
        BinOp::Ne => !values_equal(lhs, rhs),
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => match compare_order(lhs, rhs)? {
            None => false, // NaN 개입
            Some(o) => match op {
                BinOp::Lt => o == Ordering::Less,
                BinOp::Le => o != Ordering::Greater,
                BinOp::Gt => o == Ordering::Greater,
                BinOp::Ge => o != Ordering::Less,
                _ => unreachable!(),
            },
        },
        BinOp::And | BinOp::Or => unreachable!("불리언은 엔진에서 단축평가"),
    };
    Ok(Value::Bool(b))
}

// Float 동등은 IEEE 의미(NaN != NaN)가 의도된 동작 — clippy float_cmp 허용.
#[allow(clippy::float_cmp)]
fn values_equal(a: &Value, b: &Value) -> bool {
    use Value::*;
    match (a, b) {
        (Int(x), Int(y)) => x == y,
        (Float(x), Float(y)) => x == y, // IEEE: NaN != NaN
        (Int(x), Float(y)) | (Float(y), Int(x)) => (*x as f64) == *y,
        (Bool(x), Bool(y)) => x == y,
        (String(x), String(y)) => x == y,
        (Nothing, Nothing) => true,
        (List(x), List(y)) => x == y,
        (Record(x), Record(y)) => x == y,
        _ => false, // 타입 불일치 = not equal
    }
}

fn compare_order(a: &Value, b: &Value) -> Result<Option<Ordering>> {
    use Value::*;
    let r = match (a, b) {
        (Int(x), Int(y)) => x.partial_cmp(y),
        (Float(x), Float(y)) => x.partial_cmp(y),
        (Int(x), Float(y)) => (*x as f64).partial_cmp(y),
        (Float(x), Int(y)) => x.partial_cmp(&(*y as f64)),
        (String(x), String(y)) => x.partial_cmp(y),
        _ => bail!("비교할 수 없는 타입: {} 와 {}", a.type_name(), b.type_name()),
    };
    Ok(r)
}

/// 불리언 컨텍스트에서 Bool 을 강제한다(암묵 truthiness 없음).
pub fn as_bool(v: &Value) -> Result<bool> {
    match v {
        Value::Bool(b) => Ok(*b),
        other => bail!("bool 이 필요합니다 ({})", other.type_name()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shellcore::ast::BinOp;
    use crate::shellcore::value::Value;

    #[test]
    fn equality_across_types_and_floats() {
        assert_eq!(apply_compare(BinOp::Eq, &Value::Int(1), &Value::Int(1)).unwrap(), Value::Bool(true));
        assert_eq!(apply_compare(BinOp::Ne, &Value::Int(1), &Value::Int(2)).unwrap(), Value::Bool(true));
        assert_eq!(apply_compare(BinOp::Eq, &Value::Int(1), &Value::String("1".into())).unwrap(), Value::Bool(false));
        assert_eq!(apply_compare(BinOp::Eq, &Value::Int(2), &Value::Float(2.0)).unwrap(), Value::Bool(true));
        assert_eq!(apply_compare(BinOp::Eq, &Value::Float(f64::NAN), &Value::Float(f64::NAN)).unwrap(), Value::Bool(false));
    }

    #[test]
    fn ordering_numbers_and_strings_and_errors() {
        assert_eq!(apply_compare(BinOp::Gt, &Value::Int(200), &Value::Int(100)).unwrap(), Value::Bool(true));
        assert_eq!(apply_compare(BinOp::Le, &Value::Float(1.5), &Value::Int(2)).unwrap(), Value::Bool(true));
        assert_eq!(apply_compare(BinOp::Lt, &Value::String("a".into()), &Value::String("b".into())).unwrap(), Value::Bool(true));
        assert!(apply_compare(BinOp::Lt, &Value::Bool(true), &Value::Int(1)).is_err());
        assert_eq!(apply_compare(BinOp::Gt, &Value::Float(f64::NAN), &Value::Int(1)).unwrap(), Value::Bool(false));
    }

    #[test]
    fn as_bool_strict() {
        assert!(as_bool(&Value::Bool(true)).unwrap());
        assert!(as_bool(&Value::Int(1)).is_err());
    }
}
