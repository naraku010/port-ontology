// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // One executable, two entry points:
    //   port-ontology.exe        -> desktop app
    //   port-ontology.exe --mcp  -> MCP server on stdio for an LLM CLI
    if std::env::args().any(|a| a == "--mcp") {
        port_ontology_lib::mcp::run();
    } else {
        port_ontology_lib::run();
    }
}
