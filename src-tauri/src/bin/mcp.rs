use std::path::{Path, PathBuf};
use std::process::ExitCode;

use liteskill_vr_lib::db::Database;
use liteskill_vr_lib::mcp::server::McpServer;
use liteskill_vr_lib::MCP_PORT;

#[tokio::main]
async fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(msg) => {
            eprintln!("{msg}");
            print_usage();
            return ExitCode::from(2);
        }
    };

    let db = match open_db(&args.path, args.init) {
        Ok(db) => db,
        Err(msg) => {
            eprintln!("{msg}");
            return ExitCode::FAILURE;
        }
    };

    let server = McpServer::new(db);

    match args.transport {
        Transport::Stdio => run_stdio(server, &args.path).await,
        Transport::Http { port } => run_http(server, port, &args.path).await,
    }
}

async fn run_http(server: McpServer, port: u16, path: &Path) -> ExitCode {
    let addr = match server.start(port).await {
        Ok(addr) => addr,
        Err(e) => {
            eprintln!("Failed to start MCP server on port {port}: {e}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!(
        "liteskillvr-mcp listening on http://{addr}/mcp (project: {})",
        path.display()
    );
    eprintln!("Ctrl-C to stop.");
    if let Err(e) = tokio::signal::ctrl_c().await {
        eprintln!("Signal handler error: {e}");
        return ExitCode::FAILURE;
    }
    eprintln!("\nShutting down.");
    ExitCode::SUCCESS
}

async fn run_stdio(server: McpServer, path: &Path) -> ExitCode {
    eprintln!(
        "liteskillvr-mcp serving MCP over stdio (project: {})",
        path.display()
    );
    if let Err(e) = server.serve_stdio().await {
        eprintln!("stdio transport error: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

enum Transport {
    Http { port: u16 },
    Stdio,
}

struct Args {
    path: PathBuf,
    transport: Transport,
    init: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut path: Option<PathBuf> = None;
    let mut port: u16 = MCP_PORT;
    let mut port_set = false;
    let mut stdio = false;
    let mut init = false;
    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--port" => {
                let v = iter.next().ok_or("--port requires a value")?;
                port = v
                    .parse()
                    .map_err(|_| format!("--port must be a number, got '{v}'"))?;
                port_set = true;
            }
            "--stdio" => stdio = true,
            "--init" => init = true,
            "-h" | "--help" => {
                print_usage();
                std::process::exit(0);
            }
            s if !s.starts_with('-') => {
                if path.is_some() {
                    return Err(format!("Unexpected positional argument: '{s}'"));
                }
                path = Some(PathBuf::from(s));
            }
            other => return Err(format!("Unknown argument: '{other}'")),
        }
    }
    if stdio && port_set {
        return Err("--stdio and --port are mutually exclusive".to_string());
    }
    let path = path.ok_or("Missing required argument: <PROJECT_PATH>")?;
    let transport = if stdio {
        Transport::Stdio
    } else {
        Transport::Http { port }
    };
    Ok(Args {
        path,
        transport,
        init,
    })
}

fn open_db(path: &Path, init: bool) -> Result<Database, String> {
    if path.exists() {
        Database::open(path).map_err(|e| format!("Failed to open '{}': {e}", path.display()))
    } else if init {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Project");
        Database::open_and_seed(path, name)
            .map_err(|e| format!("Failed to create '{}': {e}", path.display()))
    } else {
        Err(format!(
            "Project file not found: {}. Pass --init to create a new one.",
            path.display()
        ))
    }
}

fn print_usage() {
    eprintln!("liteskillvr-mcp — headless MCP server for a LiteSkill VR project");
    eprintln!();
    eprintln!("USAGE: liteskillvr-mcp [OPTIONS] <PROJECT_PATH>");
    eprintln!();
    eprintln!("OPTIONS:");
    eprintln!("  --port <PORT>   HTTP MCP server port (default {MCP_PORT})");
    eprintln!("  --stdio         Serve MCP over stdin/stdout instead of HTTP");
    eprintln!("  --init          Create the project file if it doesn't exist");
    eprintln!("  -h, --help      Show this help");
}
