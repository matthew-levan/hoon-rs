use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

mod hover_helpers;
mod navigation;
mod requests;
mod server;
mod status;
mod tasks;
mod tokens;

use dashmap::DashMap;
use parser::ast::hoon::{BaseType, Hoon, Limb, NounExpr, ParsedAtom, Skin, Spec, Woof};
use parser::{local_bindings_with_metadata, parse_with_metadata};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, DocumentSymbolParams, DocumentSymbolResponse,
    DidChangeConfigurationParams,
    GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents, HoverParams,
    HoverProviderCapability, InitializeParams, InitializeResult, MessageType, OneOf,
    Location, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind, Url,
};
use tower_lsp::{Client, LanguageServer};
use tokio::task::JoinHandle;
use tracing::{debug, info, trace, warn};

use crate::config::HoonLspConfig;
use crate::document_store::{DocumentStore, SharedDocumentStore};
use crate::parser_adapter::parse_diagnostics;
use crate::position_mapper::range_from_byte_offsets;
use crate::stdlib::{bootstrap_embedded_stdlib, embedded_stdlib_sources};
use crate::symbol_index::{
    byte_offset_at_position, document_symbols,
    word_at_position, workspace_definitions_for_symbol,
    workspace_root_for, WorkspaceIndex,
};
pub(crate) use hover_helpers::parser_render_value;
#[cfg(test)]
pub(crate) use hover_helpers::parser_render_value_with_scope;
use hover_helpers::{
    field_type_from_location, field_type_from_type_in_text, is_tilde_literal_at,
    literal_hover_markdown, local_binding_definition_location, local_binding_hover_markdown,
    local_binding_type_symbol_at, location_is_member_of_module, symbol_hover_markdown,
    unresolved_hover_markdown,
};
use tokens::{
    is_simple_symbol_name, namespace_member_symbol_at_position, namespace_rhs_symbol_at_position,
    token_at_position,
};

pub struct Backend {
    client: Client,
    docs: SharedDocumentStore,
    workspace_index: Arc<WorkspaceIndex>,
    config: Arc<RwLock<HoonLspConfig>>,
    parse_tasks: Arc<DashMap<Url, JoinHandle<()>>>,
    index_tasks: Arc<DashMap<PathBuf, JoinHandle<()>>>,
    status_throttle: Arc<DashMap<String, Instant>>,
}

impl Backend {
    pub fn new(client: Client, config: HoonLspConfig) -> Self {
        let workspace_index = Arc::new(WorkspaceIndex::new());

        Self {
            client,
            docs: Arc::new(DocumentStore::new()),
            workspace_index,
            config: Arc::new(RwLock::new(config)),
            parse_tasks: Arc::new(DashMap::new()),
            index_tasks: Arc::new(DashMap::new()),
            status_throttle: Arc::new(DashMap::new()),
        }
    }

    pub(super) fn config_snapshot(&self) -> HoonLspConfig {
        self.config
            .read()
            .expect("config lock")
            .clone()
    }
}

#[cfg(test)]
mod tests;
