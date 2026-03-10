use super::*;

impl Backend {
    pub(super) fn resolve_definition_matches(
        &self,
        uri: &Url,
        symbol: &str,
        allow_workspace_scan: bool,
    ) -> Vec<Location> {
        trace!(uri = %uri, symbol, allow_workspace_scan, "resolve_definition_matches start");
        let mut matches = self
            .workspace_index
            .local_definitions_for_symbol(uri, symbol);
        debug!(uri = %uri, symbol, matches = matches.len(), "local definition matches");
        if matches.is_empty() {
            matches = self
                .workspace_index
                .imported_definitions_for_symbol(uri, symbol);
            debug!(uri = %uri, symbol, matches = matches.len(), "imported definition matches");
        }
        if matches.is_empty() {
            matches = self.workspace_index.definitions_for_symbol(symbol);
            matches.retain(|loc| &loc.uri != uri);
            // If any results are from user files (non-stdlib), drop the stdlib ones
            // so we don't show both e.g. the repo definition and the /var/folders copy.
            if matches
                .iter()
                .any(|loc| !self.workspace_index.is_stdlib_location(loc))
            {
                matches.retain(|loc| !self.workspace_index.is_stdlib_location(loc));
            }
            debug!(uri = %uri, symbol, matches = matches.len(), "workspace-index symbol matches");
        }
        if allow_workspace_scan && matches.is_empty() {
            let cfg = self.config_snapshot();
            matches = workspace_definitions_for_symbol(
                uri, symbol, cfg.max_workspace_files, cfg.follow_symlinks,
            );
            debug!(uri = %uri, symbol, matches = matches.len(), "workspace scan matches");
        }
        matches
    }

    pub(super) fn resolve_definition_matches_without_local(
        &self,
        uri: &Url,
        symbol: &str,
        allow_workspace_scan: bool,
    ) -> Vec<Location> {
        trace!(
            uri = %uri,
            symbol,
            allow_workspace_scan,
            "resolve_definition_matches_without_local start"
        );
        let mut matches = self
            .workspace_index
            .imported_definitions_for_symbol(uri, symbol);
        debug!(uri = %uri, symbol, matches = matches.len(), "imported definition matches");
        if matches.is_empty() {
            matches = self.workspace_index.definitions_for_symbol(symbol);
            matches.retain(|loc| &loc.uri != uri);
            // If any results are from user files (non-stdlib), drop the stdlib ones.
            if matches
                .iter()
                .any(|loc| !self.workspace_index.is_stdlib_location(loc))
            {
                matches.retain(|loc| !self.workspace_index.is_stdlib_location(loc));
            }
            debug!(uri = %uri, symbol, matches = matches.len(), "workspace-index symbol matches");
        }
        if allow_workspace_scan && matches.is_empty() {
            let cfg = self.config_snapshot();
            matches = workspace_definitions_for_symbol(
                uri, symbol, cfg.max_workspace_files, cfg.follow_symlinks,
            );
            debug!(uri = %uri, symbol, matches = matches.len(), "workspace scan matches");
        }
        matches
    }

    pub(super) fn resolve_qualified_member_matches(
        &self,
        current_uri: &Url,
        current_text: &str,
        member: &str,
        module: &str,
    ) -> Vec<Location> {
        let mut matches = self.resolve_definition_matches_without_local(current_uri, member, true);
        matches.retain(|loc| location_is_member_of_module(current_uri, current_text, loc, module));
        matches
    }

    pub(super) fn projected_field_hover_markdown(
        &self,
        current_uri: &Url,
        current_text: &str,
        cursor_byte: usize,
        token: &str,
    ) -> Option<String> {
        let (field, base) = token.split_once('.')?;
        if field.is_empty() || base.is_empty() {
            return None;
        }
        if !is_simple_symbol_name(field) || !is_simple_symbol_name(base) {
            return None;
        }

        let mut base_type = None;
        for binding in local_bindings_with_metadata(current_text) {
            if binding.start_byte > cursor_byte {
                continue;
            }
            if binding.name == base {
                base_type = binding.declared_type;
            }
        }
        let base_type = base_type?;
        let (type_name, module_name) = if let Some((ty, module)) = base_type.split_once(':') {
            (ty.trim(), Some(module.trim()))
        } else {
            (base_type.trim(), None)
        };
        if type_name.is_empty() {
            return None;
        }

        let locations = if let Some(module) = module_name {
            self.resolve_qualified_member_matches(current_uri, current_text, type_name, module)
        } else {
            self.resolve_definition_matches_without_local(current_uri, type_name, true)
        };
        for loc in locations {
            if let Some(ty) =
                field_type_from_location(current_uri, current_text, &loc, type_name, field)
            {
                return Some(format!("**`{token}`**\n\n- type: `{ty}`"));
            }
        }
        if let Some(module) = module_name {
            for (_name, text) in embedded_stdlib_sources() {
                if let Some(ty) = field_type_from_type_in_text(text, type_name, module, field) {
                    return Some(format!("**`{token}`**\n\n- type: `{ty}`"));
                }
            }
        }
        None
    }
}
