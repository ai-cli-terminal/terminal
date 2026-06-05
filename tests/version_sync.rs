//! VERSION 파일과 Cargo.toml 의 package.version 이 일치하는지 강제한다(릴리즈 단일 진실원).

#[test]
fn version_file_matches_cargo_pkg_version() {
    let cargo_version = env!("CARGO_PKG_VERSION");
    let version_file = include_str!("../VERSION").trim();
    assert_eq!(
        cargo_version, version_file,
        "Cargo.toml version ({cargo_version}) 와 VERSION 파일 ({version_file}) 이 달라요 — 릴리즈 전 동기화하세요."
    );
}
