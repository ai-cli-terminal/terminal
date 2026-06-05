//! 값 렌더링: 스칼라는 한 줄, 리스트/레코드/테이블은 정렬 표(MVP — ASCII).

use crate::shellcore::value::{OrderedMap, Value};

/// 값을 사람이 읽는 문자열로 렌더한다.
pub fn format_value(v: &Value) -> String {
    match v {
        Value::Nothing => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(s) => s.clone(),
        Value::Record(r) => format_record(r),
        Value::List(items) => format_list(items),
    }
}

fn format_list(items: &[Value]) -> String {
    if items.is_empty() {
        return "[]".to_string();
    }
    if items.iter().all(|v| matches!(v, Value::Record(_))) {
        return format_table(items);
    }
    let mut rows = vec![vec!["#".to_string(), "value".to_string()]];
    for (i, v) in items.iter().enumerate() {
        rows.push(vec![i.to_string(), format_value(v)]);
    }
    render_grid(&rows)
}

fn format_table(items: &[Value]) -> String {
    let mut cols: Vec<String> = Vec::new();
    for v in items {
        if let Value::Record(r) = v {
            for k in r.keys() {
                if !cols.iter().any(|c| c == k) {
                    cols.push(k.to_string());
                }
            }
        }
    }
    let mut header = vec!["#".to_string()];
    header.extend(cols.iter().cloned());
    let mut rows = vec![header];
    for (i, v) in items.iter().enumerate() {
        if let Value::Record(r) = v {
            let mut row = vec![i.to_string()];
            for c in &cols {
                row.push(r.get(c).map(format_value).unwrap_or_default());
            }
            rows.push(row);
        }
    }
    render_grid(&rows)
}

fn format_record(r: &OrderedMap) -> String {
    let mut rows = vec![vec!["key".to_string(), "value".to_string()]];
    for (k, v) in r.iter() {
        rows.push(vec![k.to_string(), format_value(v)]);
    }
    render_grid(&rows)
}

fn render_grid(rows: &[Vec<String>]) -> String {
    let ncol = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut widths = vec![0usize; ncol];
    for r in rows {
        for (i, c) in r.iter().enumerate() {
            widths[i] = widths[i].max(c.chars().count());
        }
    }
    let mut out = String::new();
    for r in rows {
        for (i, c) in r.iter().enumerate() {
            if i > 0 {
                out.push_str("  ");
            }
            out.push_str(c);
            for _ in c.chars().count()..widths[i] {
                out.push(' ');
            }
        }
        out.push('\n');
    }
    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shellcore::value::{OrderedMap, Value};

    #[test]
    fn scalars_render_inline() {
        assert_eq!(format_value(&Value::Int(5)), "5");
        assert_eq!(format_value(&Value::String("hi".into())), "hi");
        assert_eq!(format_value(&Value::Nothing), "");
    }

    #[test]
    fn table_has_header_and_rows() {
        let mut r = OrderedMap::new();
        r.insert("name", Value::String("README.md".into()));
        r.insert("size", Value::Int(12));
        let table = Value::List(vec![Value::Record(r)]);
        let out = format_value(&table);
        assert!(out.contains("name"), "헤더 포함: {out}");
        assert!(out.contains("README.md"), "값 포함: {out}");
        assert!(out.contains("size"), "헤더 포함: {out}");
    }

    #[test]
    fn scalar_list_is_indexed() {
        let out = format_value(&Value::List(vec![Value::Int(1), Value::Int(2)]));
        assert!(out.contains('1') && out.contains('2'), "{out}");
    }
}
