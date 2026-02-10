use super::*;

impl Backend {
    pub(super) fn cancel_parse_task(&self, uri: &Url) {
        if let Some((_, handle)) = self.parse_tasks.remove(uri) {
            handle.abort();
        }
    }

    pub(super) fn schedule_parse(&self, uri: Url, expected_version: i32, immediate: bool) {
        self.cancel_parse_task(&uri);

        let docs = Arc::clone(&self.docs);
        let client = self.client.clone();
        let parse_tasks = Arc::clone(&self.parse_tasks);
        let debounce_ms = self.config_snapshot().debounce_ms;
        let uri_for_task = uri.clone();

        let handle = tokio::spawn(async move {
            if !immediate {
                tokio::time::sleep(Duration::from_millis(debounce_ms)).await;
            }

            let Some(snapshot) = docs.get(&uri_for_task) else {
                parse_tasks.remove(&uri_for_task);
                return;
            };

            if snapshot.version != expected_version {
                parse_tasks.remove(&uri_for_task);
                return;
            }

            let uri_for_parse = uri_for_task.clone();
            let text_for_parse = snapshot.text.clone();
            let diags = tokio::task::spawn_blocking(move || parse_diagnostics(&uri_for_parse, &text_for_parse))
                .await
                .unwrap_or_default();
            client
                .publish_diagnostics(uri_for_task.clone(), diags, Some(snapshot.version))
                .await;

            parse_tasks.remove(&uri_for_task);
        });

        self.parse_tasks.insert(uri, handle);
    }

    pub(super) fn schedule_workspace_index(&self, uri: &Url) {
        let Ok(path) = uri.to_file_path() else {
            return;
        };
        let root = workspace_root_for(&path);
        if self.index_tasks.contains_key(&root) {
            return;
        }

        let index = Arc::clone(&self.workspace_index);
        let cfg = self.config_snapshot();
        let max_workspace_files = cfg.max_workspace_files;
        let follow_symlinks = cfg.follow_symlinks;
        let refresh_ms = cfg.workspace_refresh_ms;
        let root_for_task = root.clone();
        let client = self.client.clone();
        self.notify_status_throttled(
            "workspace-index-start",
            MessageType::INFO,
            format!("hoon-lsp indexing workspace: {}", root_for_task.display()),
            Duration::from_secs(20),
        );
        self.notify_user(
            MessageType::INFO,
            format!("hoon-lsp indexing workspace: {}", root_for_task.display()),
        );
        let handle = tokio::spawn(async move {
            let start = std::time::Instant::now();
            let stats = tokio::task::spawn_blocking({
                let index = Arc::clone(&index);
                let root_for_index = root_for_task.clone();
                move || index.index_workspace(&root_for_index, max_workspace_files, follow_symlinks)
            })
            .await
            .unwrap_or_default();
            info!(
                root = %root_for_task.display(),
                scanned_files = stats.scanned_files,
                indexed_files = stats.indexed_files,
                elapsed_ms = start.elapsed().as_millis(),
                "workspace index refresh"
            );
            let elapsed_ms = start.elapsed().as_millis();
            if elapsed_ms > 3_000 {
                client
                    .log_message(
                        MessageType::WARNING,
                        format!(
                            "hoon-lsp under load: indexed {} files in {}ms",
                            stats.indexed_files, elapsed_ms
                        ),
                    )
                    .await;
                client
                    .show_message(
                        MessageType::WARNING,
                        format!(
                            "hoon-lsp under load: indexed {} files in {}ms",
                            stats.indexed_files, elapsed_ms
                        ),
                    )
                    .await;
            } else {
                client
                    .log_message(
                        MessageType::INFO,
                        format!(
                            "hoon-lsp ready: indexed {} files in {}ms",
                            stats.indexed_files, elapsed_ms
                        ),
                    )
                    .await;
                client
                    .show_message(
                        MessageType::INFO,
                        format!(
                            "hoon-lsp ready: indexed {} files in {}ms",
                            stats.indexed_files, elapsed_ms
                        ),
                    )
                    .await;
            }
            let mut interval = tokio::time::interval(Duration::from_millis(refresh_ms));
            loop {
                interval.tick().await;
                let start = std::time::Instant::now();
                let stats = tokio::task::spawn_blocking({
                    let index = Arc::clone(&index);
                    let root_for_index = root_for_task.clone();
                    move || index.index_workspace(&root_for_index, max_workspace_files, follow_symlinks)
                })
                .await
                .unwrap_or_default();
                info!(
                    root = %root_for_task.display(),
                    scanned_files = stats.scanned_files,
                    indexed_files = stats.indexed_files,
                    elapsed_ms = start.elapsed().as_millis(),
                    "workspace index refresh"
                );
            }
        });
        self.index_tasks.insert(root, handle);
    }
}
