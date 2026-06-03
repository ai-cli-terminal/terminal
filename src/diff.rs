//! 라인 단위 unified diff (LCS 기반, 순수·결정적). W9 안전 미리보기용.
//!
//! 외부 의존성 없이 표준 LCS DP로 공통 부분수열을 구해 삭제/추가/유지 라인을 출력한다.
//! 대용량은 호출측(`preview.rs`)에서 바이트 상한으로 보호한다.

/// 두 텍스트의 라인 단위 unified diff 문자열. 라인 집합이 동일하면 빈 문자열.
/// `--- <before_label>` / `+++ <after_label>` 헤더 + `-`(삭제)/`+`(추가)/` `(유지) 라인.
pub fn unified_diff(before: &str, after: &str, before_label: &str, after_label: &str) -> String {
    let a: Vec<&str> = before.lines().collect();
    let b: Vec<&str> = after.lines().collect();
    if a == b {
        return String::new();
    }
    let (n, m) = (a.len(), b.len());
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if a[i] == b[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }

    let mut body = String::new();
    let (mut i, mut j) = (0usize, 0usize);
    while i < n && j < m {
        if a[i] == b[j] {
            body.push(' ');
            body.push_str(a[i]);
            body.push('\n');
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            body.push('-');
            body.push_str(a[i]);
            body.push('\n');
            i += 1;
        } else {
            body.push('+');
            body.push_str(b[j]);
            body.push('\n');
            j += 1;
        }
    }
    while i < n {
        body.push('-');
        body.push_str(a[i]);
        body.push('\n');
        i += 1;
    }
    while j < m {
        body.push('+');
        body.push_str(b[j]);
        body.push('\n');
        j += 1;
    }

    format!("--- {before_label}\n+++ {after_label}\n{body}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_is_empty() {
        assert_eq!(unified_diff("a\nb\n", "a\nb\n", "x", "y"), "");
    }

    #[test]
    fn single_line_change_shows_minus_plus() {
        let d = unified_diff("a\nb\nc\n", "a\nB\nc\n", "old", "new");
        assert!(d.contains("--- old"), "{d}");
        assert!(d.contains("+++ new"), "{d}");
        assert!(d.contains("-b"), "{d}");
        assert!(d.contains("+B"), "{d}");
        assert!(d.contains(" a"), "{d}");
        assert!(d.contains(" c"), "{d}");
    }

    #[test]
    fn pure_addition_and_deletion() {
        let add = unified_diff("a\n", "a\nb\n", "o", "n");
        assert!(add.contains("+b"), "{add}");
        let del = unified_diff("a\nb\n", "a\n", "o", "n");
        assert!(del.contains("-b"), "{del}");
    }
}
