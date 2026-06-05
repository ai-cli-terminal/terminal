//! 공용 유틸.

use std::path::PathBuf;

/// 홈 디렉터리(HOME → USERPROFILE). 둘 다 없으면 None.
pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}
