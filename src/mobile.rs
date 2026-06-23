//! Mobile embedding boundary for Android/iOS shellcore spikes.
//!
//! This is not a JNI/UniFFI layer yet. It is the stable Rust-side contract that
//! mobile bindings should wrap: one session owns pure `shellcore` state, each
//! line returns structured output plus updated state, and host process spawn is
//! disabled by construction.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::shellcore::engine::{eval_line, Engine};
use crate::shellcore::external::ExecutionCapabilities;
use crate::shellcore::format::format_value;
use crate::shellcore::value::{OrderedMap, Value};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MobileSessionState {
    pub cwd: String,
    pub vars: serde_json::Value,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MobileEvalResult {
    pub ok: bool,
    pub output_json: serde_json::Value,
    pub output_text: String,
    pub error: Option<String>,
    pub state: MobileSessionState,
}

pub struct MobileShell {
    engine: Engine,
}

impl Default for MobileShell {
    fn default() -> Self {
        Self::new()
    }
}

impl MobileShell {
    pub fn new() -> Self {
        Self {
            engine: Engine::pure(),
        }
    }

    pub fn from_state(state: MobileSessionState) -> Self {
        let mut engine = Engine::pure();
        engine.cwd = PathBuf::from(state.cwd);
        engine.vars = ordered_map_from_json(&state.vars).unwrap_or_default();
        engine.exit_code = state.exit_code;
        Self { engine }
    }

    pub fn capabilities(&self) -> ExecutionCapabilities {
        self.engine.execution_capabilities()
    }

    pub fn state(&self) -> MobileSessionState {
        MobileSessionState {
            cwd: self.engine.cwd.display().to_string(),
            vars: value_to_json(&Value::Record(self.engine.vars.clone())),
            exit_code: self.engine.exit_code,
        }
    }

    pub fn eval_line(&mut self, input: &str) -> MobileEvalResult {
        let evaluated = catch_unwind(AssertUnwindSafe(|| eval_line(input, &mut self.engine)));
        match evaluated {
            Ok(Ok(value)) => {
                let output_text = if matches!(value, Value::Nothing) {
                    String::new()
                } else {
                    format_value(&value)
                };
                MobileEvalResult {
                    ok: true,
                    output_json: value_to_json(&value),
                    output_text,
                    error: None,
                    state: self.state(),
                }
            }
            Ok(Err(err)) => MobileEvalResult {
                ok: false,
                output_json: serde_json::Value::Null,
                output_text: String::new(),
                error: Some(err.to_string()),
                state: self.state(),
            },
            Err(_) => MobileEvalResult {
                ok: false,
                output_json: serde_json::Value::Null,
                output_text: String::new(),
                error: Some("mobile shell panic isolated".to_string()),
                state: self.state(),
            },
        }
    }
}

fn value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Nothing => serde_json::Value::Null,
        Value::Bool(v) => serde_json::Value::Bool(*v),
        Value::Int(v) => serde_json::json!(v),
        Value::Float(v) => serde_json::json!(v),
        Value::String(v) => serde_json::Value::String(v.clone()),
        Value::List(items) => serde_json::Value::Array(items.iter().map(value_to_json).collect()),
        Value::Record(rec) => {
            let mut out = serde_json::Map::new();
            for (key, val) in rec.iter() {
                out.insert(key.to_string(), value_to_json(val));
            }
            serde_json::Value::Object(out)
        }
    }
}

fn ordered_map_from_json(json: &serde_json::Value) -> Option<OrderedMap> {
    let serde_json::Value::Object(obj) = json else {
        return None;
    };
    let mut out = OrderedMap::new();
    for (key, val) in obj {
        out.insert(key.clone(), value_from_json(val));
    }
    Some(out)
}

fn value_from_json(json: &serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null => Value::Nothing,
        serde_json::Value::Bool(v) => Value::Bool(*v),
        serde_json::Value::Number(v) => {
            if let Some(i) = v.as_i64() {
                Value::Int(i)
            } else if let Some(f) = v.as_f64() {
                Value::Float(f)
            } else {
                Value::String(v.to_string())
            }
        }
        serde_json::Value::String(v) => Value::String(v.clone()),
        serde_json::Value::Array(items) => Value::List(items.iter().map(value_from_json).collect()),
        serde_json::Value::Object(_) => {
            Value::Record(ordered_map_from_json(json).unwrap_or_default())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mobile_shell_evaluates_pure_structured_pipeline() {
        let mut shell = MobileShell::new();
        assert_eq!(shell.capabilities(), ExecutionCapabilities::pure_core());

        let out = shell.eval_line("[{size: 50} {size: 200}] | where size > 100");
        assert!(out.ok, "{out:?}");
        assert_eq!(out.output_json, serde_json::json!([{ "size": 200 }]));
        assert!(out.output_text.contains("200"), "{out:?}");
    }

    #[test]
    fn mobile_shell_persists_session_state() {
        let mut shell = MobileShell::new();
        assert!(shell.eval_line("let limit = 100").ok);
        let state = shell.state();
        assert_eq!(state.vars, serde_json::json!({ "limit": 100 }));

        let mut restored = MobileShell::from_state(state);
        let out = restored.eval_line("[{size: 200}] | where size > $limit | length");
        assert_eq!(out.output_json, serde_json::json!(1));
    }

    #[test]
    fn mobile_shell_blocks_external_execution_before_path_lookup() {
        let mut shell = MobileShell::new();
        let out = shell.eval_line("definitely-not-a-builtin");
        assert!(!out.ok);
        assert!(
            out.error
                .as_deref()
                .is_some_and(|e| e.contains("external execution disabled")),
            "{out:?}"
        );
    }
}
