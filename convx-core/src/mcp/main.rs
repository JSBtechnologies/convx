fn main() -> anyhow::Result<()> {
    // TODO: Re-enable after license server is live
    // License gate — MCP server requires a valid license
    // if let Err(msg) = convx::license::require_license() {
    //     let err = serde_json::json!({
    //         "jsonrpc": "2.0",
    //         "error": {
    //             "code": -32001,
    //             "message": "License required",
    //             "data": msg,
    //         },
    //         "id": null
    //     });
    //     eprintln!("{}", serde_json::to_string(&err)?);
    //     std::process::exit(1);
    // }

    convx::mcp_server::run_stdio_server()
}
