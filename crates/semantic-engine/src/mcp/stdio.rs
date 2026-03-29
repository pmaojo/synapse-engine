use crate::store::SynapseStore;
use serde_json::Value;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use super::protocol::{JsonRpcRequest, JsonRpcResponse};
use super::tools::handle_tool_call;

pub async fn run_stdio_mcp_server(store: Arc<SynapseStore>) {
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();

    let mut reader = BufReader::new(stdin).lines();

    while let Ok(Some(line)) = reader.next_line().await {
            if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(&line) {
                let id = req.id.unwrap_or(Value::Null);

                let response = match handle_tool_call(req.method.as_str(), req.params, store.clone()).await {
                    Ok(res) => JsonRpcResponse::success(id, res),
                    Err(e) => JsonRpcResponse::error(id, -32000, &e),
                };

                let out = serde_json::to_string(&response).unwrap() + "\n";
                stdout.write_all(out.as_bytes()).await.unwrap();
                stdout.flush().await.unwrap();
            }
    }
}
