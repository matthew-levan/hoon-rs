use super::*;

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        if let Some(options) = params.initialization_options {
            self.config
                .write()
                .expect("config lock")
                .merge_json(&options);
        }
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
            ..InitializeResult::default()
        })
    }

    async fn initialized(&self, _: tower_lsp::lsp_types::InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "hoon-lsp initialized")
            .await;
        self.client
            .show_message(
                MessageType::INFO,
                "hoon-lsp initialized: loading embedded stdlib in background",
            )
            .await;
        let client = self.client.clone();
        let index = Arc::clone(&self.workspace_index);
        tokio::spawn(async move {
            let start = Instant::now();
            match tokio::task::spawn_blocking(move || bootstrap_embedded_stdlib(&index)).await {
                Ok(Ok(files)) => {
                    let elapsed_ms = start.elapsed().as_millis();
                    info!(files, elapsed_ms, "embedded stdlib indexed");
                    client
                        .show_message(
                            MessageType::INFO,
                            format!(
                                "hoon-lsp ready: embedded stdlib loaded ({} files, {}ms)",
                                files, elapsed_ms
                            ),
                        )
                        .await;
                }
                Ok(Err(err)) => {
                    warn!(error = %err, "failed to bootstrap embedded stdlib");
                    client
                        .show_message(
                            MessageType::WARNING,
                            format!("hoon-lsp warning: failed to load embedded stdlib ({err})"),
                        )
                        .await;
                }
                Err(err) => {
                    warn!(error = %err, "stdlib bootstrap task failed");
                    client
                        .show_message(
                            MessageType::WARNING,
                            format!("hoon-lsp warning: stdlib bootstrap task failed ({err})"),
                        )
                        .await;
                }
            }
        });
    }

    async fn shutdown(&self) -> Result<()> {
        for entry in self.parse_tasks.iter() {
            entry.value().abort();
        }
        for entry in self.index_tasks.iter() {
            entry.value().abort();
        }
        Ok(())
    }

    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        self.config
            .write()
            .expect("config lock")
            .merge_json(&params.settings);
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let doc = params.text_document;
        let uri = doc.uri.clone();
        self.workspace_index.upsert_document(&uri, &doc.text);
        self.docs.open(uri.clone(), doc.version, doc.text);
        self.schedule_parse(uri.clone(), doc.version, false);
        self.schedule_workspace_index(&uri);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let doc = params.text_document;
        if let Some(snapshot) = self
            .docs
            .apply_changes(&doc.uri, doc.version, params.content_changes)
        {
            self.workspace_index
                .upsert_document(&doc.uri, &snapshot.text);
            self.schedule_parse(doc.uri.clone(), doc.version, false);
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(snapshot) = self.docs.get(&uri) {
            self.workspace_index.upsert_document(&uri, &snapshot.text);
            self.schedule_parse(uri, snapshot.version, true);
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.cancel_parse_task(&uri);
        self.docs.close(&uri);
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        self.handle_hover(params).await
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        self.handle_goto_definition(params).await
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        let Some(snapshot) = self.docs.get(&uri) else {
            return Ok(None);
        };

        let symbols = document_symbols(&snapshot.text);
        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }
}
