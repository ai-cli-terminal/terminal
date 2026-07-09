//! `ai` 바이너리의 CLI 표면 — 서브커맨드 정의·디스패치·핸들러.
//! `main.rs`에서 분리(move-only, Wave 2). 각 모듈은 자신의 테스트를 포함한다.

pub mod command;
pub mod doctor;
pub mod gate;
pub mod hooks;
pub mod inspect;
pub mod io;
pub mod remote;
pub mod run;
pub mod shell_run;
