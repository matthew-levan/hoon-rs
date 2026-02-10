mod backend;
mod config;
mod document_store;
mod parser_adapter;
mod position_mapper;
mod stdlib;
mod symbol_index;

use backend::Backend;
use config::HoonLspConfig;
use tower_lsp::{LspService, Server};
use tracing::info;
use tracing_subscriber::EnvFilter;

const WORKER_STACK_BYTES: usize = 64 * 1024 * 1024;

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("hoon-lsp-worker")
        .thread_stack_size(WORKER_STACK_BYTES)
        .build()
        .expect("build tokio runtime");
    runtime.block_on(async_main());
}

async fn async_main() {
    // Surface panics to Helix stderr with backtrace context while debugging stability.
    std::panic::set_hook(Box::new(|panic_info| {
        let bt = std::backtrace::Backtrace::force_capture();
        eprintln!("hoon-lsp panic: {panic_info}\nbacktrace:\n{bt}");
    }));

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("hoon_lsp::backend=debug,hoon_lsp::symbol_index=debug,hoon_lsp=info,parser=warn,tower_lsp=info")
    });
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_ansi(false)
        .with_target(true)
        .with_file(true)
        .with_line_number(true)
        .with_writer(std::io::stderr)
        .init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let config = HoonLspConfig::default();
    info!(
        debounce_ms = config.debounce_ms,
        max_workspace_files = config.max_workspace_files,
        follow_symlinks = config.follow_symlinks,
        workspace_refresh_ms = config.workspace_refresh_ms,
        "hoon-lsp startup config"
    );
    let (service, socket) = LspService::new(|client| Backend::new(client, config.clone()));
    Server::new(stdin, stdout, socket).serve(service).await;
}
