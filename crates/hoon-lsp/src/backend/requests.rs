use super::*;

impl Backend {
    pub(super) async fn handle_hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let pos = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;

        let Some(snapshot) = self.docs.get(&uri) else {
            return Ok(None);
        };

        let byte = byte_offset_at_position(&snapshot.text, pos);
        if let Some(token) = token_at_position(&snapshot.text, byte) {
            if let Some(md) = self.projected_field_hover_markdown(&uri, &snapshot.text, byte, &token)
            {
                return Ok(Some(Hover {
                    contents: HoverContents::Scalar(tower_lsp::lsp_types::MarkedString::String(md)),
                    range: None,
                }));
            }
        }
        if is_tilde_literal_at(&snapshot.text, byte) {
            return Ok(Some(Hover {
                contents: HoverContents::Scalar(tower_lsp::lsp_types::MarkedString::String(
                    "**`~`**\n\nNull noun in Hoon (`unit` none), similar to `null`/`None`."
                        .to_string(),
                )),
                range: None,
            }));
        }

        if let Some(symbol) = word_at_position(&snapshot.text, byte) {
            if let Some(local_md) = local_binding_hover_markdown(&snapshot.text, byte, &symbol) {
                return Ok(Some(Hover {
                    contents: HoverContents::Scalar(tower_lsp::lsp_types::MarkedString::String(
                        local_md,
                    )),
                    range: None,
                }));
            }

            let matches = self.resolve_definition_matches(&uri, &symbol, false);
            if !matches.is_empty() {
                let md = symbol_hover_markdown(&uri, &snapshot.text, &symbol, &matches);
                return Ok(Some(Hover {
                    contents: HoverContents::Scalar(tower_lsp::lsp_types::MarkedString::String(md)),
                    range: None,
                }));
            }
        }

        if let Some(token) = token_at_position(&snapshot.text, byte) {
            if let Some(render) = parser_render_value(&token) {
                if render.ty.as_deref() == Some("reference") {
                    let reference = render
                        .value
                        .clone()
                        .unwrap_or_else(|| token.trim().to_string());
                    let md = unresolved_hover_markdown(&token, &reference);
                    return Ok(Some(Hover {
                        contents: HoverContents::Scalar(tower_lsp::lsp_types::MarkedString::String(md)),
                        range: None,
                    }));
                }
                if render.ty.is_some() || render.value.is_some() {
                    let md = literal_hover_markdown(&token, &render);
                    return Ok(Some(Hover {
                        contents: HoverContents::Scalar(tower_lsp::lsp_types::MarkedString::String(md)),
                        range: None,
                    }));
                }
            }
        }

        Ok(None)
    }

    pub(super) async fn handle_goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let request_start = Instant::now();
        let tdpp = params.text_document_position_params;
        let uri = tdpp.text_document.uri;
        let Some(snapshot) = self.docs.get(&uri) else {
            return Ok(None);
        };

        let byte = byte_offset_at_position(&snapshot.text, tdpp.position);
        debug!(
            uri = %uri,
            line = tdpp.position.line,
            character = tdpp.position.character,
            byte,
            "goto_definition request"
        );
        if let Some((member, module)) = namespace_member_symbol_at_position(&snapshot.text, byte) {
            debug!(uri = %uri, member, module, "namespace member context");
            let mut matches = self
                .workspace_index
                .imported_member_definitions_for_symbol(&uri, &module, &member);
            if matches.is_empty() {
                matches = self.resolve_qualified_member_matches(&uri, &snapshot.text, &member, &module);
            }
            debug!(uri = %uri, member, module, matches = matches.len(), "namespace member matches");
            if matches.is_empty() {
                return Ok(None);
            }
            if matches.len() == 1 {
                return Ok(Some(GotoDefinitionResponse::Scalar(matches.remove(0))));
            }
            return Ok(Some(GotoDefinitionResponse::Array(matches)));
        }

        let namespace_rhs_symbol = namespace_rhs_symbol_at_position(&snapshot.text, byte);
        let symbol = namespace_rhs_symbol
            .clone()
            .or_else(|| word_at_position(&snapshot.text, byte));
        let Some(symbol) = symbol else {
            debug!(uri = %uri, "no symbol at cursor");
            return Ok(None);
        };
        debug!(
            uri = %uri,
            symbol,
            namespace_rhs = namespace_rhs_symbol.is_some(),
            "resolved cursor symbol"
        );
        let in_namespace_rhs_context = namespace_rhs_symbol.is_some();
        let local_type_symbol = local_binding_type_symbol_at(&snapshot.text, byte);
        let in_local_type_context = local_type_symbol.as_deref() == Some(symbol.as_str());

        if !in_local_type_context && !in_namespace_rhs_context {
            if let Some(local) = local_binding_definition_location(&uri, &snapshot.text, byte, &symbol) {
                return Ok(Some(GotoDefinitionResponse::Scalar(local)));
            }
        }

        if in_local_type_context {
            let mut matches = self.resolve_definition_matches(&uri, &symbol, true);
            if matches.is_empty() {
                return Ok(None);
            }
            if matches.len() == 1 {
                return Ok(Some(GotoDefinitionResponse::Scalar(matches.remove(0))));
            }
            return Ok(Some(GotoDefinitionResponse::Array(matches)));
        }

        let mut matches = if in_namespace_rhs_context {
            self.resolve_definition_matches_without_local(&uri, &symbol, true)
        } else {
            self.resolve_definition_matches(&uri, &symbol, true)
        };

        if matches.is_empty() {
            return Ok(None);
        }

        if matches.len() == 1 {
            if request_start.elapsed() > Duration::from_millis(700) {
                self.notify_status_throttled(
                    "goto-slow",
                    MessageType::WARNING,
                    format!(
                        "hoon-lsp under load: definition lookup took {}ms",
                        request_start.elapsed().as_millis()
                    ),
                    Duration::from_secs(20),
                );
            }
            return Ok(Some(GotoDefinitionResponse::Scalar(matches.remove(0))));
        }

        if request_start.elapsed() > Duration::from_millis(700) {
            self.notify_status_throttled(
                "goto-slow",
                MessageType::WARNING,
                format!(
                    "hoon-lsp under load: definition lookup took {}ms",
                    request_start.elapsed().as_millis()
                ),
                Duration::from_secs(20),
            );
        }
        Ok(Some(GotoDefinitionResponse::Array(matches)))
    }
}
