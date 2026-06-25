//! 셸 값 모델: 구조화 파이프라인이 흘리는 데이터 타입 + 순서 보존 레코드 맵.

use crate::shellcore::ast::Expr;

/// 순서 보존 맵(레코드/변수 스코프용). 새 의존성(indexmap) 회피용 경량 구현.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OrderedMap {
    entries: Vec<(String, Value)>,
}

impl OrderedMap {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
    /// 키가 있으면 값 갱신(순서 유지), 없으면 끝에 추가.
    pub fn insert(&mut self, key: impl Into<String>, val: Value) {
        let key = key.into();
        if let Some(e) = self.entries.iter_mut().find(|(k, _)| *k == key) {
            e.1 = val;
        } else {
            self.entries.push((key, val));
        }
    }
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|(k, _)| k.as_str())
    }
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// 셸 값. 테이블 = `List`(원소가 모두 `Record`). 별도 타입 아님.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Nothing,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<Value>),
    Record(OrderedMap),
    Closure(Box<Expr>),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Nothing => "nothing",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::List(_) => "list",
            Value::Record(_) => "record",
            Value::Closure(_) => "closure",
        }
    }
    /// 외부 명령 인자·필드명 등에서 쓰는 문자열 강제.
    pub fn coerce_string(&self) -> String {
        match self {
            Value::Nothing => String::new(),
            Value::Bool(b) => b.to_string(),
            Value::Int(n) => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::String(s) => s.clone(),
            Value::Closure(_) => "<closure>".to_string(),
            other => format!("{other:?}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordered_map_preserves_order_and_overwrites() {
        let mut m = OrderedMap::new();
        m.insert("b", Value::Int(1));
        m.insert("a", Value::Int(2));
        m.insert("b", Value::Int(9)); // 덮어쓰기(순서 유지)
        let keys: Vec<&str> = m.keys().collect();
        assert_eq!(keys, vec!["b", "a"]);
        assert_eq!(m.get("b"), Some(&Value::Int(9)));
        assert_eq!(m.get("missing"), None);
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn value_type_name_and_coerce_string() {
        assert_eq!(Value::Nothing.type_name(), "nothing");
        assert_eq!(Value::List(vec![]).type_name(), "list");
        assert_eq!(Value::Int(42).coerce_string(), "42");
        assert_eq!(Value::Bool(true).coerce_string(), "true");
        assert_eq!(Value::String("x".into()).coerce_string(), "x");
        assert_eq!(Value::Nothing.coerce_string(), "");
    }
}
