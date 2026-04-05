use anyhow::anyhow;
use serde_json::Value;

use super::McpHub;

impl McpHub {
    pub(crate) async fn call_tool(
        &self,
        local_name: &str,
        arguments: Value,
    ) -> anyhow::Result<Value> {
        let target = self
            .tool_targets
            .get(local_name)
            .ok_or_else(|| anyhow!("unknown mcp tool: {local_name}"))?
            .clone();

        let server = self
            .servers
            .get(&target.server)
            .ok_or_else(|| anyhow!("unknown mcp server: {}", target.server))?;

        server
            .stdio
            .client
            .request(
                "tools/call",
                Some(serde_json::json!({
                    "name": target.remote_name,
                    "arguments": arguments,
                })),
            )
            .await
    }

    pub(crate) async fn resources_list(
        &self,
        server: Option<&str>,
        cursor: Option<&str>,
    ) -> anyhow::Result<Value> {
        self.forward_list_request("resources/list", "resources", server, cursor)
            .await
    }

    pub(crate) async fn prompts_list(
        &self,
        server: Option<&str>,
        cursor: Option<&str>,
    ) -> anyhow::Result<Value> {
        self.forward_list_request("prompts/list", "prompts", server, cursor)
            .await
    }

    async fn forward_list_request(
        &self,
        method: &str,
        field: &str,
        server: Option<&str>,
        cursor: Option<&str>,
    ) -> anyhow::Result<Value> {
        let cursor = cursor.map(|c| c.trim()).filter(|c| !c.is_empty());
        let params = cursor.map(|cursor| serde_json::json!({ "cursor": cursor }));

        match server.map(|s| s.trim()).filter(|s| !s.is_empty()) {
            Some(name) => {
                let server = self
                    .servers
                    .get(name)
                    .ok_or_else(|| anyhow!("unknown mcp server: {name}"))?;
                let result = server.stdio.client.request(method, params).await?;
                Ok(serde_json::json!({ "server": name, "result": result }))
            }
            None => {
                let mut all: Vec<Value> = Vec::new();
                for name in self.servers.keys() {
                    let server = self
                        .servers
                        .get(name)
                        .ok_or_else(|| anyhow!("unknown mcp server: {name}"))?;
                    let result = server.stdio.client.request(method, params.clone()).await?;
                    let items = result.get(field).cloned().unwrap_or(Value::Null);
                    all.push(serde_json::json!({
                        "server": name,
                        (field): items,
                        "nextCursor": result.get("nextCursor"),
                    }));
                }
                Ok(Value::Array(all))
            }
        }
    }

    pub(crate) async fn resources_read(
        &self,
        server: Option<&str>,
        uri: &str,
    ) -> anyhow::Result<Value> {
        let uri = uri.trim();
        if uri.is_empty() {
            return Err(anyhow!("mcp_resources_read: uri is empty"));
        }

        match server.map(|s| s.trim()).filter(|s| !s.is_empty()) {
            Some(name) => {
                let server = self
                    .servers
                    .get(name)
                    .ok_or_else(|| anyhow!("unknown mcp server: {name}"))?;
                let result = server
                    .stdio
                    .client
                    .request("resources/read", Some(serde_json::json!({ "uri": uri })))
                    .await?;
                Ok(serde_json::json!({ "server": name, "result": result }))
            }
            None => {
                for name in self.servers.keys() {
                    let server = self
                        .servers
                        .get(name)
                        .ok_or_else(|| anyhow!("unknown mcp server: {name}"))?;
                    let res = server
                        .stdio
                        .client
                        .request("resources/read", Some(serde_json::json!({ "uri": uri })))
                        .await;
                    if let Ok(result) = res {
                        return Ok(serde_json::json!({ "server": name, "result": result }));
                    }
                }
                Err(anyhow!("mcp_resources_read: no server handled uri: {uri}"))
            }
        }
    }

    pub(crate) async fn prompts_get(
        &self,
        server: Option<&str>,
        name: &str,
        arguments: Option<Value>,
    ) -> anyhow::Result<Value> {
        let name = name.trim();
        if name.is_empty() {
            return Err(anyhow!("mcp_prompts_get: name is empty"));
        }

        let mut params = serde_json::Map::new();
        params.insert("name".to_string(), Value::String(name.to_string()));
        if let Some(arguments) = arguments {
            params.insert("arguments".to_string(), arguments);
        }
        let params = Value::Object(params);

        match server.map(|s| s.trim()).filter(|s| !s.is_empty()) {
            Some(server_name) => {
                let server = self
                    .servers
                    .get(server_name)
                    .ok_or_else(|| anyhow!("unknown mcp server: {server_name}"))?;
                let result = server
                    .stdio
                    .client
                    .request("prompts/get", Some(params))
                    .await?;
                Ok(serde_json::json!({ "server": server_name, "result": result }))
            }
            None => {
                for server_name in self.servers.keys() {
                    let server = self
                        .servers
                        .get(server_name)
                        .ok_or_else(|| anyhow!("unknown mcp server: {server_name}"))?;
                    let res = server
                        .stdio
                        .client
                        .request("prompts/get", Some(params.clone()))
                        .await;
                    if let Ok(result) = res {
                        return Ok(serde_json::json!({ "server": server_name, "result": result }));
                    }
                }
                Err(anyhow!("mcp_prompts_get: no server handled prompt: {name}"))
            }
        }
    }
}
