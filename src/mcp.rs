//! 통합 MCP 관리 (설계 §27, Phase 2).
//!
//! 여러 MCP 서버를 중앙 등록·집계한다. MCP 도구 호출은 **액션**이므로 일반 명령과 동일한
//! 위험도 분류·컨센트·감사를 받는다(§27, RULES §2). 부작용 도구는 자동 실행하지 않는다.
//! MVP는 설정 파싱·레지스트리·컨센트 판정까지(실제 연결은 후속, `auto_connect=false`).

use anyhow::{anyhow, Result};

/// 등록된 MCP 서버 정의.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpServer {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
}

/// mcp.json(`{"mcpServers": {name: {command, args}}}`)을 파싱한다.
pub fn parse_servers(json: &str) -> Result<Vec<McpServer>> {
    let v: serde_json::Value = serde_json::from_str(json)?;
    let map = v
        .get("mcpServers")
        .and_then(|m| m.as_object())
        .ok_or_else(|| anyhow!("missing 'mcpServers' object"))?;
    let mut servers: Vec<McpServer> = map
        .iter()
        .map(|(name, def)| McpServer {
            name: name.clone(),
            command: def
                .get("command")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string(),
            args: def
                .get("args")
                .and_then(|a| a.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect();
    servers.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(servers)
}

const MUTATING_MARKERS: &[&str] = &[
    "write", "create", "delete", "update", "remove", "send", "exec", "run", "put", "post", "set",
    "move", "rename", "apply", "install",
];

/// 부작용(mutate) 도구인지 — true면 컨센트·감사 필요(§27).
pub fn is_mutating_tool(tool: &str) -> bool {
    let lower = tool.to_lowercase();
    MUTATING_MARKERS.iter().any(|m| lower.contains(m))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mcp_servers() {
        let json = r#"{"mcpServers":{"fs":{"command":"node","args":["server.js","--root","/x"]},"git":{"command":"uvx","args":["mcp-git"]}}}"#;
        let servers = parse_servers(json).unwrap();
        assert_eq!(servers.len(), 2);
        let fs = servers.iter().find(|s| s.name == "fs").unwrap();
        assert_eq!(fs.command, "node");
        assert_eq!(fs.args, vec!["server.js", "--root", "/x"]);
    }

    #[test]
    fn empty_or_missing_is_ok() {
        assert_eq!(parse_servers(r#"{"mcpServers":{}}"#).unwrap().len(), 0);
        assert!(parse_servers("not json").is_err());
    }

    #[test]
    fn detects_mutating_tools() {
        for t in [
            "write_file",
            "create_issue",
            "delete_branch",
            "send_message",
            "run_command",
        ] {
            assert!(is_mutating_tool(t), "{t} should be mutating");
        }
        for t in ["read_file", "list_dir", "get_status", "search"] {
            assert!(!is_mutating_tool(t), "{t} should be read-only");
        }
    }
}
