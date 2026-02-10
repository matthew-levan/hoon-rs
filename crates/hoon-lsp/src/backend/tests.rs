    use futures::StreamExt;
    use serde_json::{json, Value};
    use std::fs;
    use tokio::time::{timeout, Duration, Instant};
    use tower::{Service, ServiceExt};
    use tower_lsp::jsonrpc::Request;
    use tower_lsp::lsp_types::{Location, PublishDiagnosticsParams};
    use tower_lsp::LspService;
    use tempfile::tempdir;

    use super::Backend;
    use super::parser_render_value;
    use super::parser_render_value_with_scope;
    use crate::config::HoonLspConfig;

    fn initialize_request(id: i64) -> Request {
        Request::build("initialize")
            .params(json!({
                "capabilities": {}
            }))
            .id(id)
            .finish()
    }

    fn initialize_request_with_options(id: i64, options: Value) -> Request {
        Request::build("initialize")
            .params(json!({
                "capabilities": {},
                "initializationOptions": options
            }))
            .id(id)
            .finish()
    }

    async fn next_publish_diagnostics(
        socket: &mut tower_lsp::ClientSocket,
    ) -> PublishDiagnosticsParams {
        let maybe_req = timeout(Duration::from_secs(2), socket.next())
            .await
            .expect("timed out waiting for server notification");
        let req = maybe_req.expect("server socket closed");
        assert_eq!(req.method(), "textDocument/publishDiagnostics");
        let params = req.params().cloned().expect("publishDiagnostics params");
        serde_json::from_value(params).expect("valid PublishDiagnosticsParams")
    }

    async fn initialize_service(service: &mut LspService<Backend>) {
        let init = initialize_request(1);
        let init_response = service
            .ready()
            .await
            .expect("service ready")
            .call(init)
            .await
            .expect("initialize call ok")
            .expect("initialize response exists");
        assert!(init_response.is_ok());
    }

    async fn open_document(
        service: &mut LspService<Backend>,
        uri: &str,
        version: i32,
        text: &str,
    ) {
        let did_open = Request::build("textDocument/didOpen")
            .params(json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": "hoon",
                    "version": version,
                    "text": text
                }
            }))
            .finish();
        let open_response = service
            .ready()
            .await
            .expect("service ready")
            .call(did_open)
            .await
            .expect("didOpen call ok");
        assert_eq!(open_response, None);
    }

    async fn change_document(service: &mut LspService<Backend>, uri: &str, version: i32, text: &str) {
        let did_change = Request::build("textDocument/didChange")
            .params(json!({
                "textDocument": {
                    "uri": uri,
                    "version": version
                },
                "contentChanges": [
                    {
                        "text": text
                    }
                ]
            }))
            .finish();
        let change_response = service
            .ready()
            .await
            .expect("service ready")
            .call(did_change)
            .await
            .expect("didChange call ok");
        assert_eq!(change_response, None);
    }

    fn definition_locations_from_result(result: &Value) -> Vec<Location> {
        if let Ok(loc) = serde_json::from_value::<Location>(result.clone()) {
            return vec![loc];
        }
        serde_json::from_value::<Vec<Location>>(result.clone())
            .expect("definition must return Location or Location[]")
    }

    #[tokio::test(flavor = "current_thread")]
    async fn initialize_advertises_core_capabilities() {
        let (mut service, _socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));
        let req = initialize_request(1);
        let response = service
            .ready()
            .await
            .expect("service ready")
            .call(req)
            .await
            .expect("initialize call ok")
            .expect("initialize response exists");

        let result = response.result().expect("initialize success result");
        let capabilities: &Value = &result["capabilities"];
        assert_eq!(capabilities["textDocumentSync"], json!(2));
        assert_eq!(capabilities["hoverProvider"], json!(true));
        assert_eq!(capabilities["definitionProvider"], json!(true));
        assert_eq!(capabilities["documentSymbolProvider"], json!(true));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn did_open_publishes_diagnostics() {
        let (mut service, mut socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));

        initialize_service(&mut service).await;

        let uri = "file:///tmp/protocol-diag.hoon";
        open_document(&mut service, uri, 1, "this is not hoon").await;

        let published = next_publish_diagnostics(&mut socket).await;
        assert_eq!(published.uri.as_str(), uri);
        assert!(!published.diagnostics.is_empty());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn initialize_options_override_debounce() {
        let (mut service, mut socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));

        let init = initialize_request_with_options(
            1,
            json!({
                "hoonLsp": {
                    "debounceMs": 450
                }
            }),
        );
        let init_response = service
            .ready()
            .await
            .expect("service ready")
            .call(init)
            .await
            .expect("initialize call ok")
            .expect("initialize response exists");
        assert!(init_response.is_ok());

        let start = Instant::now();
        open_document(&mut service, "file:///tmp/protocol-init-config.hoon", 1, "bad hoon").await;
        let _ = next_publish_diagnostics(&mut socket).await;
        assert!(
            start.elapsed() >= Duration::from_millis(350),
            "initializeOptions debounce should be applied"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn did_change_cancels_stale_debounced_parse() {
        let cfg = HoonLspConfig {
            debounce_ms: 250,
            ..HoonLspConfig::default()
        };
        let (mut service, mut socket) = LspService::new(|client| Backend::new(client, cfg.clone()));
        initialize_service(&mut service).await;

        let uri = "file:///tmp/protocol-stale.hoon";
        open_document(&mut service, uri, 1, "first invalid payload").await;

        let did_change = Request::build("textDocument/didChange")
            .params(json!({
                "textDocument": {
                    "uri": uri,
                    "version": 2
                },
                "contentChanges": [
                    {
                        "text": "second invalid payload"
                    }
                ]
            }))
            .finish();
        let change_response = service
            .ready()
            .await
            .expect("service ready")
            .call(did_change)
            .await
            .expect("didChange call ok");
        assert_eq!(change_response, None);

        let published = next_publish_diagnostics(&mut socket).await;
        assert_eq!(published.uri.as_str(), uri);
        assert_eq!(published.version, Some(2));
        assert!(!published.diagnostics.is_empty());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn did_change_configuration_updates_debounce() {
        let (mut service, mut socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));
        initialize_service(&mut service).await;

        let uri = "file:///tmp/protocol-config-change.hoon";
        open_document(&mut service, uri, 1, "first invalid payload").await;
        let _ = next_publish_diagnostics(&mut socket).await;

        let did_change_cfg = Request::build("workspace/didChangeConfiguration")
            .params(json!({
                "settings": {
                    "hoonLsp": {
                        "debounceMs": 450
                    }
                }
            }))
            .finish();
        let cfg_response = service
            .ready()
            .await
            .expect("service ready")
            .call(did_change_cfg)
            .await
            .expect("didChangeConfiguration call ok");
        assert_eq!(cfg_response, None);

        let start = Instant::now();
        change_document(&mut service, uri, 2, "second invalid payload").await;
        let published = next_publish_diagnostics(&mut socket).await;
        assert_eq!(published.version, Some(2));
        assert!(
            start.elapsed() >= Duration::from_millis(350),
            "didChangeConfiguration debounce should be applied"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn did_save_forces_immediate_parse() {
        let cfg = HoonLspConfig {
            debounce_ms: 1_000,
            ..HoonLspConfig::default()
        };
        let (mut service, mut socket) = LspService::new(|client| Backend::new(client, cfg.clone()));
        initialize_service(&mut service).await;

        let uri = "file:///tmp/protocol-save.hoon";
        open_document(&mut service, uri, 1, "save invalid payload").await;

        let did_save = Request::build("textDocument/didSave")
            .params(json!({
                "textDocument": {
                    "uri": uri
                }
            }))
            .finish();
        let start = Instant::now();
        let save_response = service
            .ready()
            .await
            .expect("service ready")
            .call(did_save)
            .await
            .expect("didSave call ok");
        assert_eq!(save_response, None);

        let published = next_publish_diagnostics(&mut socket).await;
        assert_eq!(published.uri.as_str(), uri);
        assert_eq!(published.version, Some(1));
        assert!(
            start.elapsed() < Duration::from_millis(cfg.debounce_ms / 2),
            "didSave parse should not wait for debounce window"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn hover_returns_semantic_literal_details() {
        let (mut service, _socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));
        initialize_service(&mut service).await;

        let uri = "file:///tmp/protocol-hover-literal.hoon";
        let text = "++  foo  ~\n";
        open_document(&mut service, uri, 1, text).await;

        let hover = Request::build("textDocument/hover")
            .params(json!({
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 9 }
            }))
            .id(2)
            .finish();
        let response = service
            .ready()
            .await
            .expect("service ready")
            .call(hover)
            .await
            .expect("hover call ok")
            .expect("hover response exists");
        let result = response.result().expect("hover must return an LSP result");
        assert_ne!(result, &json!(null), "hover should return non-null contents");
        let contents = result
            .get("contents")
            .and_then(|v| v.as_str().or_else(|| v.get("value").and_then(|vv| vv.as_str())))
            .expect("hover contents string");
        assert!(contents.contains("`~`"), "hover should mention tilde literal");
        assert!(
            contents.contains("null") || contents.contains("None"),
            "hover should describe null/unit-none semantics"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn definition_resolves_imported_symbol_cross_file() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).expect("git marker");

        let main_path = root.join("main.hoon");
        let foo_path = root.join("foo.hoon");
        let main_text = "/+ foo\n++  main\n  foo\n";
        let foo_text = "++  foo\n  ~\n";
        fs::write(&main_path, main_text).expect("write main");
        fs::write(&foo_path, foo_text).expect("write foo");

        let main_uri = format!(
            "file://{}",
            main_path.to_str().expect("main path str")
        );
        let foo_uri = format!(
            "file://{}",
            foo_path.to_str().expect("foo path str")
        );

        let (mut service, _socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));
        initialize_service(&mut service).await;
        open_document(&mut service, &foo_uri, 1, foo_text).await;
        open_document(&mut service, &main_uri, 1, main_text).await;

        let definition = Request::build("textDocument/definition")
            .params(json!({
                "textDocument": { "uri": main_uri },
                "position": { "line": 2, "character": 2 }
            }))
            .id(3)
            .finish();
        let response = service
            .ready()
            .await
            .expect("service ready")
            .call(definition)
            .await
            .expect("definition call ok")
            .expect("definition response exists");
        let result = response.result().expect("definition result");
        let locations = definition_locations_from_result(result);
        assert!(!locations.is_empty());
        assert!(locations.iter().any(|loc| loc.uri.as_str() == foo_uri));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn definition_prefers_import_over_local_collision() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).expect("git marker");

        let main_path = root.join("main.hoon");
        let foo_path = root.join("foo.hoon");
        let main_text = "/+ foo\n++  foo\n  ~\n++  main\n  foo\n";
        let foo_text = "++  foo\n  ~\n";
        fs::write(&main_path, main_text).expect("write main");
        fs::write(&foo_path, foo_text).expect("write foo");

        let main_uri = format!(
            "file://{}",
            main_path.to_str().expect("main path str")
        );
        let foo_uri = format!(
            "file://{}",
            foo_path.to_str().expect("foo path str")
        );

        let (mut service, _socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));
        initialize_service(&mut service).await;
        open_document(&mut service, &foo_uri, 1, foo_text).await;
        open_document(&mut service, &main_uri, 1, main_text).await;

        let definition = Request::build("textDocument/definition")
            .params(json!({
                "textDocument": { "uri": main_uri },
                "position": { "line": 4, "character": 3 }
            }))
            .id(5)
            .finish();
        let response = service
            .ready()
            .await
            .expect("service ready")
            .call(definition)
            .await
            .expect("definition call ok")
            .expect("definition response exists");
        let result = response.result().expect("definition result");
        let locations = definition_locations_from_result(result);
        assert!(!locations.is_empty());
        assert!(locations.iter().any(|loc| loc.uri.as_str() == foo_uri));
        assert!(!locations.iter().any(|loc| loc.uri.as_str() == main_uri));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn definition_returns_scalar_for_unique_match() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).expect("git marker");

        let main_path = root.join("main.hoon");
        let foo_path = root.join("foo.hoon");
        let main_text = "/+ foo\n++  main\n  foo\n";
        let foo_text = "++  foo\n  ~\n";
        fs::write(&main_path, main_text).expect("write main");
        fs::write(&foo_path, foo_text).expect("write foo");

        let main_uri = format!("file://{}", main_path.to_str().expect("main path str"));
        let foo_uri = format!("file://{}", foo_path.to_str().expect("foo path str"));

        let (mut service, _socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));
        initialize_service(&mut service).await;
        open_document(&mut service, &foo_uri, 1, foo_text).await;
        open_document(&mut service, &main_uri, 1, main_text).await;

        let definition = Request::build("textDocument/definition")
            .params(json!({
                "textDocument": { "uri": main_uri },
                "position": { "line": 2, "character": 2 }
            }))
            .id(10)
            .finish();
        let response = service
            .ready()
            .await
            .expect("service ready")
            .call(definition)
            .await
            .expect("definition call ok")
            .expect("definition response exists");
        let result = response.result().expect("definition result");
        assert!(result.is_object(), "expected scalar Location response");
        assert_eq!(result["uri"], foo_uri);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn definition_returns_array_for_multiple_matches() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).expect("git marker");

        let main_path = root.join("main.hoon");
        let a_path = root.join("a.hoon");
        let b_path = root.join("b.hoon");
        let main_text = "++  main\n  foo\n";
        let a_text = "++  foo\n  ~\n";
        let b_text = "++  foo\n  ~\n";
        fs::write(&main_path, main_text).expect("write main");
        fs::write(&a_path, a_text).expect("write a");
        fs::write(&b_path, b_text).expect("write b");

        let main_uri = format!("file://{}", main_path.to_str().expect("main path str"));
        let a_uri = format!("file://{}", a_path.to_str().expect("a path str"));
        let b_uri = format!("file://{}", b_path.to_str().expect("b path str"));

        let (mut service, _socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));
        initialize_service(&mut service).await;
        open_document(&mut service, &a_uri, 1, a_text).await;
        open_document(&mut service, &b_uri, 1, b_text).await;
        open_document(&mut service, &main_uri, 1, main_text).await;

        let definition = Request::build("textDocument/definition")
            .params(json!({
                "textDocument": { "uri": main_uri },
                "position": { "line": 1, "character": 2 }
            }))
            .id(11)
            .finish();
        let response = service
            .ready()
            .await
            .expect("service ready")
            .call(definition)
            .await
            .expect("definition call ok")
            .expect("definition response exists");
        let result = response.result().expect("definition result");
        let arr = result.as_array().expect("expected Location[] response");
        assert!(arr.iter().any(|loc| loc["uri"] == a_uri));
        assert!(arr.iter().any(|loc| loc["uri"] == b_uri));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn definition_resolves_local_equals_binding_inside_arm() {
        let (mut service, _socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));
        initialize_service(&mut service).await;

        let uri = "file:///tmp/protocol-local-binding-def.hoon";
        let text = "|%\n++  main\n  =/  local  ~\n  local\n--\n";
        open_document(&mut service, uri, 1, text).await;

        let definition = Request::build("textDocument/definition")
            .params(json!({
                "textDocument": { "uri": uri },
                "position": { "line": 3, "character": 4 }
            }))
            .id(12)
            .finish();
        let response = service
            .ready()
            .await
            .expect("service ready")
            .call(definition)
            .await
            .expect("definition call ok")
            .expect("definition response exists");
        let result = response.result().expect("definition result");
        assert!(result.is_object(), "expected scalar Location response");
        assert_eq!(result["uri"], uri);
        assert_eq!(result["range"]["start"]["line"], 2);
        assert_eq!(result["range"]["start"]["character"], 6);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn definition_resolves_local_door_sample_binding() {
        let (mut service, _socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));
        initialize_service(&mut service).await;

        let uri = "file:///tmp/protocol-local-door-sample-def.hoon";
        let text = "|_  =bowl:gall\n++  main\n  bowl\n";
        open_document(&mut service, uri, 1, text).await;

        let definition = Request::build("textDocument/definition")
            .params(json!({
                "textDocument": { "uri": uri },
                "position": { "line": 2, "character": 3 }
            }))
            .id(60)
            .finish();
        let response = service
            .ready()
            .await
            .expect("service ready")
            .call(definition)
            .await
            .expect("definition call ok")
            .expect("definition response exists");
        let result = response.result().expect("definition result");
        assert!(result.is_object(), "expected scalar Location response");
        assert_eq!(result["uri"], uri);
        assert_eq!(result["range"]["start"]["line"], 0);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn definition_resolves_namespace_member_without_import_alias() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).expect("git marker");

        let main_path = root.join("main.hoon");
        let lib_path = root.join("stdlib.hoon");
        let main_text = "++  main\n  bowl:gall\n";
        let lib_text = "++  gall\n  ++  bowl\n    ~\n++  rand\n  ++  bowl\n    ~\n";
        fs::write(&main_path, main_text).expect("write main");
        fs::write(&lib_path, lib_text).expect("write lib");

        let main_uri = format!("file://{}", main_path.to_str().expect("main path str"));
        let lib_uri = format!("file://{}", lib_path.to_str().expect("lib path str"));

        let (mut service, _socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));
        initialize_service(&mut service).await;
        open_document(&mut service, &lib_uri, 1, lib_text).await;
        open_document(&mut service, &main_uri, 1, main_text).await;

        let definition = Request::build("textDocument/definition")
            .params(json!({
                "textDocument": { "uri": main_uri },
                "position": { "line": 1, "character": 3 }
            }))
            .id(62)
            .finish();
        let response = service
            .ready()
            .await
            .expect("service ready")
            .call(definition)
            .await
            .expect("definition call ok")
            .expect("definition response exists");
        let result = response.result().expect("definition result");
        let locations = definition_locations_from_result(result);
        assert!(!locations.is_empty());
        assert!(locations.iter().any(|loc| {
            loc.uri.as_str() == lib_uri && loc.range.start.line == 1
        }));
        assert!(!locations.iter().any(|loc| {
            loc.uri.as_str() == lib_uri && loc.range.start.line == 4
        }));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn definition_prefers_type_symbol_in_local_annotation_context() {
        let (mut service, _socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));
        initialize_service(&mut service).await;

        let uri = "file:///tmp/protocol-local-typed-binding-def.hoon";
        let text = "|%\n+$  kind  @t\n++  main\n  =/  kind  ~\n  =/  val=kind  'x'\n  val\n--\n";
        open_document(&mut service, uri, 1, text).await;

        let definition = Request::build("textDocument/definition")
            .params(json!({
                "textDocument": { "uri": uri },
                "position": { "line": 4, "character": 11 }
            }))
            .id(13)
            .finish();
        let response = service
            .ready()
            .await
            .expect("service ready")
            .call(definition)
            .await
            .expect("definition call ok")
            .expect("definition response exists");
        let result = response.result().expect("definition result");
        assert!(result.is_object(), "expected scalar Location response");
        assert_eq!(result["uri"], uri);
        assert_eq!(result["range"]["start"]["line"], 1);
        assert_eq!(result["range"]["start"]["character"], 4);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn definition_resolves_equals_alias_import() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).expect("git marker");
        fs::create_dir_all(root.join("lib")).expect("lib dir");

        let main_path = root.join("main.hoon");
        let helper_path = root.join("lib").join("helper.hoon");
        let main_text = "/= h  /lib/helper\n++  main\n  h\n";
        let helper_text = "++  h\n  ~\n";
        fs::write(&main_path, main_text).expect("write main");
        fs::write(&helper_path, helper_text).expect("write helper");

        let main_uri = format!(
            "file://{}",
            main_path.to_str().expect("main path str")
        );
        let helper_uri = format!(
            "file://{}",
            helper_path.to_str().expect("helper path str")
        );

        let (mut service, _socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));
        initialize_service(&mut service).await;
        open_document(&mut service, &helper_uri, 1, helper_text).await;
        open_document(&mut service, &main_uri, 1, main_text).await;

        let definition = Request::build("textDocument/definition")
            .params(json!({
                "textDocument": { "uri": main_uri },
                "position": { "line": 2, "character": 2 }
            }))
            .id(6)
            .finish();
        let response = service
            .ready()
            .await
            .expect("service ready")
            .call(definition)
            .await
            .expect("definition call ok")
            .expect("definition response exists");
        let result = response.result().expect("definition result");
        if let Some(arr) = result.as_array() {
            assert!(!arr.is_empty());
            assert!(arr.iter().any(|loc| loc["uri"] == helper_uri));
        } else {
            assert_eq!(result["uri"], helper_uri);
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn definition_resolves_equals_star_import() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).expect("git marker");
        fs::create_dir_all(root.join("lib")).expect("lib dir");

        let main_path = root.join("main.hoon");
        let helper_path = root.join("lib").join("helper.hoon");
        let main_text = "/= *  /lib/helper\n++  main\n  calc\n";
        let helper_text = "++  calc\n  ~\n";
        fs::write(&main_path, main_text).expect("write main");
        fs::write(&helper_path, helper_text).expect("write helper");

        let main_uri = format!(
            "file://{}",
            main_path.to_str().expect("main path str")
        );
        let helper_uri = format!(
            "file://{}",
            helper_path.to_str().expect("helper path str")
        );

        let (mut service, _socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));
        initialize_service(&mut service).await;
        open_document(&mut service, &helper_uri, 1, helper_text).await;
        open_document(&mut service, &main_uri, 1, main_text).await;

        let definition = Request::build("textDocument/definition")
            .params(json!({
                "textDocument": { "uri": main_uri },
                "position": { "line": 2, "character": 2 }
            }))
            .id(7)
            .finish();
        let response = service
            .ready()
            .await
            .expect("service ready")
            .call(definition)
            .await
            .expect("definition call ok")
            .expect("definition response exists");
        let result = response.result().expect("definition result");
        if let Some(arr) = result.as_array() {
            assert!(!arr.is_empty());
            assert!(arr.iter().any(|loc| loc["uri"] == helper_uri));
        } else {
            assert_eq!(result["uri"], helper_uri);
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn definition_prefers_explicit_alias_over_wildcard_import() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).expect("git marker");
        fs::create_dir_all(root.join("lib")).expect("lib dir");

        let main_path = root.join("main.hoon");
        let helper_path = root.join("lib").join("helper.hoon");
        let alt_path = root.join("alt.hoon");
        let main_text = "/= *  /alt\n/= helper  /lib/helper\n++  main\n  helper\n";
        let helper_text = "++  helper\n  ~\n";
        let alt_text = "++  helper\n  ~\n";
        fs::write(&main_path, main_text).expect("write main");
        fs::write(&helper_path, helper_text).expect("write helper");
        fs::write(&alt_path, alt_text).expect("write alt");

        let main_uri = format!("file://{}", main_path.to_str().expect("main path str"));
        let helper_uri = format!(
            "file://{}",
            helper_path.to_str().expect("helper path str")
        );
        let alt_uri = format!("file://{}", alt_path.to_str().expect("alt path str"));

        let (mut service, _socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));
        initialize_service(&mut service).await;
        open_document(&mut service, &helper_uri, 1, helper_text).await;
        open_document(&mut service, &alt_uri, 1, alt_text).await;
        open_document(&mut service, &main_uri, 1, main_text).await;

        let definition = Request::build("textDocument/definition")
            .params(json!({
                "textDocument": { "uri": main_uri },
                "position": { "line": 3, "character": 3 }
            }))
            .id(8)
            .finish();
        let response = service
            .ready()
            .await
            .expect("service ready")
            .call(definition)
            .await
            .expect("definition call ok")
            .expect("definition response exists");
        let result = response.result().expect("definition result");
        let locations = definition_locations_from_result(result);
        assert!(!locations.is_empty());
        assert!(locations.iter().any(|loc| loc.uri.as_str() == helper_uri));
        assert!(!locations.iter().any(|loc| loc.uri.as_str() == alt_uri));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn definition_does_not_treat_pathlike_missing_import_as_bare_import() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).expect("git marker");

        let main_path = root.join("main.hoon");
        let stray_path = root.join("helper.hoon");
        let main_text = "/= helper  /lib/helper\n++  helper\n  ~\n++  main\n  helper\n";
        let stray_text = "++  helper\n  ~\n";
        fs::write(&main_path, main_text).expect("write main");
        fs::write(&stray_path, stray_text).expect("write stray helper");

        let main_uri = format!("file://{}", main_path.to_str().expect("main path str"));
        let stray_uri = format!("file://{}", stray_path.to_str().expect("stray path str"));

        let (mut service, _socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));
        initialize_service(&mut service).await;
        open_document(&mut service, &stray_uri, 1, stray_text).await;
        open_document(&mut service, &main_uri, 1, main_text).await;

        let definition = Request::build("textDocument/definition")
            .params(json!({
                "textDocument": { "uri": main_uri },
                "position": { "line": 4, "character": 3 }
            }))
            .id(9)
            .finish();
        let response = service
            .ready()
            .await
            .expect("service ready")
            .call(definition)
            .await
            .expect("definition call ok")
            .expect("definition response exists");
        let result = response.result().expect("definition result");
        let locations = definition_locations_from_result(result);
        assert!(!locations.is_empty());
        assert!(locations.iter().any(|loc| loc.uri.as_str() == main_uri));
        assert!(!locations.iter().any(|loc| loc.uri.as_str() == stray_uri));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn hover_resolves_symbol_to_definition_value() {
        let (mut service, _socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));
        initialize_service(&mut service).await;

        let uri = "file:///tmp/protocol-hover-symbol.hoon";
        let text = "|%\n++  foo\n  ~\n++  main\n  foo\n--\n";
        open_document(&mut service, uri, 1, text).await;

        let hover = Request::build("textDocument/hover")
            .params(json!({
                "textDocument": { "uri": uri },
                "position": { "line": 4, "character": 2 }
            }))
            .id(4)
            .finish();
        let response = service
            .ready()
            .await
            .expect("service ready")
            .call(hover)
            .await
            .expect("hover call ok")
            .expect("hover response exists");
        let result = response.result().expect("hover must return an LSP result");
        assert_ne!(result, &json!(null), "hover should return non-null contents");
        let contents = result
            .get("contents")
            .and_then(|v| v.as_str().or_else(|| v.get("value").and_then(|vv| vv.as_str())))
            .expect("hover contents string");
        assert!(
            contents.contains("`foo`"),
            "hover should include the symbol name"
        );
        assert!(
            contents.contains("type:"),
            "hover should include resolved type details"
        );
        assert!(
            contents.contains("~"),
            "hover should include the definition value for foo"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn hover_resolves_local_bindings_with_type_and_value() {
        let (mut service, _socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));
        initialize_service(&mut service).await;

        let uri = "file:///tmp/protocol-hover-local-bindings.hoon";
        let text = "=/  one  \"this should be indented\"\n=/  five  one\n=/  seis  'yoyo'\n=/  siete  seis\n";
        open_document(&mut service, uri, 1, text).await;

        let hover_one = Request::build("textDocument/hover")
            .params(json!({
                "textDocument": { "uri": uri },
                "position": { "line": 1, "character": 11 }
            }))
            .id(30)
            .finish();
        let response_one = service
            .ready()
            .await
            .expect("service ready")
            .call(hover_one)
            .await
            .expect("hover call ok")
            .expect("hover response exists");
        let result_one = response_one.result().expect("hover result");
        let contents_one = result_one
            .get("contents")
            .and_then(|v| v.as_str().or_else(|| v.get("value").and_then(|vv| vv.as_str())))
            .expect("hover contents string");
        assert!(contents_one.contains("type: `tape`"));
        assert!(contents_one.contains("`\"this should be indented\"`"));

        let hover_seis = Request::build("textDocument/hover")
            .params(json!({
                "textDocument": { "uri": uri },
                "position": { "line": 3, "character": 12 }
            }))
            .id(31)
            .finish();
        let response_seis = service
            .ready()
            .await
            .expect("service ready")
            .call(hover_seis)
            .await
            .expect("hover call ok")
            .expect("hover response exists");
        let result_seis = response_seis.result().expect("hover result");
        let contents_seis = result_seis
            .get("contents")
            .and_then(|v| v.as_str().or_else(|| v.get("value").and_then(|vv| vv.as_str())))
            .expect("hover contents string");
        assert!(contents_seis.contains("type: `cord (@t)`"));
        assert!(contents_seis.contains("yoyo"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn hover_resolves_gate_sample_argument_type() {
        let (mut service, _socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));
        initialize_service(&mut service).await;

        let uri = "file:///tmp/protocol-hover-gate-arg.hoon";
        let text = "++  handle-del\n  |=  [is-ted=? uidt=@t]\n  uidt\n";
        open_document(&mut service, uri, 1, text).await;

        let hover = Request::build("textDocument/hover")
            .params(json!({
                "textDocument": { "uri": uri },
                "position": { "line": 2, "character": 3 }
            }))
            .id(61)
            .finish();
        let response = service
            .ready()
            .await
            .expect("service ready")
            .call(hover)
            .await
            .expect("hover call ok")
            .expect("hover response exists");
        let result = response.result().expect("hover result");
        let contents = result
            .get("contents")
            .and_then(|v| v.as_str().or_else(|| v.get("value").and_then(|vv| vv.as_str())))
            .expect("hover contents string");
        assert!(contents.contains("**`uidt`**"));
        assert!(contents.contains("type: `@t`"));
        assert!(!contents.contains("unresolved"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn hover_displays_full_colon_sample_type() {
        let (mut service, _socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));
        initialize_service(&mut service).await;

        let uri = "file:///tmp/protocol-hover-colon-sample.hoon";
        let text = "|_  =bowl:gall\n++  main\n  bowl\n";
        open_document(&mut service, uri, 1, text).await;

        let hover = Request::build("textDocument/hover")
            .params(json!({
                "textDocument": { "uri": uri },
                "position": { "line": 2, "character": 3 }
            }))
            .id(63)
            .finish();
        let response = service
            .ready()
            .await
            .expect("service ready")
            .call(hover)
            .await
            .expect("hover call ok")
            .expect("hover response exists");
        let result = response.result().expect("hover result");
        let contents = result
            .get("contents")
            .and_then(|v| v.as_str().or_else(|| v.get("value").and_then(|vv| vv.as_str())))
            .expect("hover contents string");
        assert!(contents.contains("type: `bowl:gall`"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn hover_resolves_struct_field_projection_type() {
        let (mut service, _socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));
        initialize_service(&mut service).await;

        let uri = "file:///tmp/protocol-hover-struct-field.hoon";
        let text = "++  gall\n  +$  bowl\n    $:  src=@p\n        now=@da\n    ==\n++  main\n  |_  =bowl:gall\n  src.bowl\n";
        open_document(&mut service, uri, 1, text).await;

        let hover = Request::build("textDocument/hover")
            .params(json!({
                "textDocument": { "uri": uri },
                "position": { "line": 7, "character": 3 }
            }))
            .id(64)
            .finish();
        let response = service
            .ready()
            .await
            .expect("service ready")
            .call(hover)
            .await
            .expect("hover call ok")
            .expect("hover response exists");
        let result = response.result().expect("hover result");
        let contents = result
            .get("contents")
            .and_then(|v| v.as_str().or_else(|| v.get("value").and_then(|vv| vv.as_str())))
            .expect("hover contents string");
        assert!(contents.contains("**`src.bowl`**"));
        assert!(contents.contains("type: `@p`"));
        assert!(!contents.contains("unresolved"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn hover_resolves_nested_list_field_projection_type() {
        let (mut service, _socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));
        initialize_service(&mut service).await;

        let uri = "file:///tmp/protocol-hover-list-field.hoon";
        let text = "+$  struct\n  $:  two=(list @)\n  ==\n++  main\n  =/  one=struct  [~]\n  two.one\n";
        open_document(&mut service, uri, 1, text).await;

        let hover = Request::build("textDocument/hover")
            .params(json!({
                "textDocument": { "uri": uri },
                "position": { "line": 5, "character": 3 }
            }))
            .id(65)
            .finish();
        let response = service
            .ready()
            .await
            .expect("service ready")
            .call(hover)
            .await
            .expect("hover call ok")
            .expect("hover response exists");
        let result = response.result().expect("hover result");
        let contents = result
            .get("contents")
            .and_then(|v| v.as_str().or_else(|| v.get("value").and_then(|vv| vv.as_str())))
            .expect("hover contents string");
        assert!(contents.contains("**`two.one`**"));
        assert!(contents.contains("type: `(list @)`"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn hover_resolves_multiline_local_binding_values() {
        let (mut service, _socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));
        initialize_service(&mut service).await;

        let uri = "file:///tmp/protocol-hover-local-bindings-multiline.hoon";
        let text = "=/  one\n  \"this should be indented\"\n=/  two  one\n";
        open_document(&mut service, uri, 1, text).await;

        let hover_two = Request::build("textDocument/hover")
            .params(json!({
                "textDocument": { "uri": uri },
                "position": { "line": 2, "character": 10 }
            }))
            .id(58)
            .finish();
        let response_two = service
            .ready()
            .await
            .expect("service ready")
            .call(hover_two)
            .await
            .expect("hover call ok")
            .expect("hover response exists");
        let result_two = response_two.result().expect("hover result");
        let contents_two = result_two
            .get("contents")
            .and_then(|v| v.as_str().or_else(|| v.get("value").and_then(|vv| vv.as_str())))
            .expect("hover contents string");
        assert!(contents_two.contains("type: `tape`"));
        assert!(contents_two.contains("\"this should be indented\""));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn hover_shows_parser_type_for_arm_values() {
        let (mut service, _socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));
        initialize_service(&mut service).await;

        let uri = "file:///tmp/protocol-hover-arm-types.hoon";
        let text = "|%\n++  atom-t  1\n++  null-t  ~\n++  main\n  atom-t\n--\n";
        open_document(&mut service, uri, 1, text).await;

        let hover = Request::build("textDocument/hover")
            .params(json!({
                "textDocument": { "uri": uri },
                "position": { "line": 4, "character": 4 }
            }))
            .id(32)
            .finish();
        let response = service
            .ready()
            .await
            .expect("service ready")
            .call(hover)
            .await
            .expect("hover call ok")
            .expect("hover response exists");
        let result = response.result().expect("hover result");
        let contents = result
            .get("contents")
            .and_then(|v| v.as_str().or_else(|| v.get("value").and_then(|vv| vv.as_str())))
            .expect("hover contents string");
        assert!(contents.contains("type: `unsigned decimal (@ud)`"));
        assert!(contents.contains("value: `1`"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn hover_renders_lists_with_element_type() {
        let (mut service, _socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));
        initialize_service(&mut service).await;

        let uri = "file:///tmp/protocol-hover-list-type.hoon";
        let text = "|%\n++  list-t  ~[1 2 3]\n++  main\n  list-t\n--\n";
        open_document(&mut service, uri, 1, text).await;

        let hover = Request::build("textDocument/hover")
            .params(json!({
                "textDocument": { "uri": uri },
                "position": { "line": 3, "character": 5 }
            }))
            .id(33)
            .finish();
        let response = service
            .ready()
            .await
            .expect("service ready")
            .call(hover)
            .await
            .expect("hover call ok")
            .expect("hover response exists");
        let result = response.result().expect("hover result");
        let contents = result
            .get("contents")
            .and_then(|v| v.as_str().or_else(|| v.get("value").and_then(|vv| vv.as_str())))
            .expect("hover contents string");
        assert!(contents.contains("type: `(list @ud)`"));
        assert!(contents.contains("value: `~[1 2 3]`"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn hover_renders_da_and_dr_literal_types() {
        let (mut service, _socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));
        initialize_service(&mut service).await;

        let uri = "file:///tmp/protocol-hover-date-types.hoon";
        let text = "|%\n++  da-t  ~2020.1.1\n++  dr-t  ~h1.m1\n--\n";
        open_document(&mut service, uri, 1, text).await;

        let hover_da = Request::build("textDocument/hover")
            .params(json!({
                "textDocument": { "uri": uri },
                "position": { "line": 1, "character": 12 }
            }))
            .id(34)
            .finish();
        let response_da = service
            .ready()
            .await
            .expect("service ready")
            .call(hover_da)
            .await
            .expect("hover call ok")
            .expect("hover response exists");
        let result_da = response_da.result().expect("hover result");
        let contents_da = result_da
            .get("contents")
            .and_then(|v| v.as_str().or_else(|| v.get("value").and_then(|vv| vv.as_str())))
            .expect("hover contents string");
        assert!(contents_da.contains("type: `absolute date (@da)`"));
        assert!(contents_da.contains("value: `~2020.1.1`"));

        let hover_dr = Request::build("textDocument/hover")
            .params(json!({
                "textDocument": { "uri": uri },
                "position": { "line": 2, "character": 12 }
            }))
            .id(35)
            .finish();
        let response_dr = service
            .ready()
            .await
            .expect("service ready")
            .call(hover_dr)
            .await
            .expect("hover call ok")
            .expect("hover response exists");
        let result_dr = response_dr.result().expect("hover result");
        let contents_dr = result_dr
            .get("contents")
            .and_then(|v| v.as_str().or_else(|| v.get("value").and_then(|vv| vv.as_str())))
            .expect("hover contents string");
        assert!(contents_dr.contains("type: `relative date/timespan (@dr)`"));
        assert!(contents_dr.contains("value: `~h1.m1`"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn hover_resolves_ketcol_type_definition_verbatim() {
        let (mut service, _socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));
        initialize_service(&mut service).await;

        let uri = "file:///tmp/protocol-hover-type-def.hoon";
        let text = "|%\n+$  spec-t\n  $:  a=@\n      b=@tas\n  ==\n++  main\n  ^-  spec-t\n  [1 %foo]\n--\n";
        open_document(&mut service, uri, 1, text).await;

        let hover = Request::build("textDocument/hover")
            .params(json!({
                "textDocument": { "uri": uri },
                "position": { "line": 6, "character": 8 }
            }))
            .id(36)
            .finish();
        let response = service
            .ready()
            .await
            .expect("service ready")
            .call(hover)
            .await
            .expect("hover call ok")
            .expect("hover response exists");
        let result = response.result().expect("hover result");
        let contents = result
            .get("contents")
            .and_then(|v| v.as_str().or_else(|| v.get("value").and_then(|vv| vv.as_str())))
            .expect("hover contents string");
        assert!(contents.contains("type: `type`"));
        assert!(contents.contains("b=@tas"));
        assert!(!contents.contains("+$  spec-t"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn hover_resolves_inline_type_definitions() {
        let (mut service, _socket) =
            LspService::new(|client| Backend::new(client, HoonLspConfig::default()));
        initialize_service(&mut service).await;

        let uri = "file:///tmp/protocol-hover-inline-type-def.hoon";
        let text = "|%\n+$  spec-4  ?(%lol %lal %rofl)\n+$  spec-5  %type\n+$  spec-6  @ux\n++  main\n  [spec-4 spec-5 spec-6]\n--\n";
        open_document(&mut service, uri, 1, text).await;

        for (idx, (symbol, character, expected)) in [
            ("spec-4", 3, "?(%lol %lal %rofl)"),
            ("spec-5", 10, "%type"),
            ("spec-6", 17, "@ux"),
        ]
        .into_iter()
        .enumerate()
        {
            let hover = Request::build("textDocument/hover")
                .params(json!({
                    "textDocument": { "uri": uri },
                    "position": { "line": 5, "character": character }
                }))
                .id(40 + idx as i64)
                .finish();
            let response = service
                .ready()
                .await
                .expect("service ready")
                .call(hover)
                .await
                .expect("hover call ok")
                .expect("hover response exists");
            let result = response.result().expect("hover result");
            let contents = result
                .get("contents")
                .and_then(|v| v.as_str().or_else(|| v.get("value").and_then(|vv| vv.as_str())))
                .expect("hover contents string");
            assert!(contents.contains(&format!("**`{symbol}`**")));
            assert!(contents.contains("type: `type`"));
            assert!(contents.contains(expected));
        }
    }

    #[test]
    fn parser_render_value_reports_expected_literal_types() {
        let one = parser_render_value("1").expect("render one");
        assert_eq!(one.ty.as_deref(), Some("unsigned decimal (@ud)"));
        assert_eq!(one.value.as_deref(), Some("1"));

        let tape = parser_render_value("\"this should be indented\"").expect("render tape");
        assert_eq!(tape.ty.as_deref(), Some("tape"));

        let da = parser_render_value("~2020.1.1").expect("render da");
        assert_eq!(da.ty.as_deref(), Some("absolute date (@da)"));
        assert_eq!(da.value.as_deref(), Some("~2020.1.1"));

        let list = parser_render_value("~[1 2 3]").expect("render list");
        assert_eq!(list.ty.as_deref(), Some("(list @ud)"));
        assert_eq!(list.value.as_deref(), Some("~[1 2 3]"));
    }

    #[test]
    fn parser_render_value_reads_multiline_gate_return_hint() {
        let gate = "|=  a=@\n^-  tape\n(scow %ud a)";
        let render = parser_render_value(gate).expect("render gate");
        assert_eq!(render.ty.as_deref(), Some("gate"));
        let signature = render.value.expect("gate signature");
        assert!(signature.contains("|= a=@ -> tape"));
        assert!(!signature.contains("inferred"));
    }

    #[test]
    fn parser_render_value_infers_ketlus_return_from_gate_arg() {
        let gate = "|=  a=@\n^+  a\n(scow %ud a)";
        let render = parser_render_value(gate).expect("render gate");
        assert_eq!(render.ty.as_deref(), Some("gate"));
        let signature = render.value.expect("gate signature");
        assert!(signature.contains("|= a=@ -> @"));
        assert!(!signature.contains("inferred"));
    }

    #[test]
    fn parser_render_value_infers_ketlus_return_from_sample_symbol_hint() {
        let gate = "|=  a=@\n^+  tape-t\n(scow %ud a)";
        let scope = "++  tape-t  \"foobar\"\n";
        let render = parser_render_value_with_scope(gate, Some(scope)).expect("render gate");
        assert_eq!(render.ty.as_deref(), Some("gate"));
        let signature = render.value.expect("gate signature");
        assert!(signature.contains("|= a=@ -> tape"));
        assert!(!signature.contains("inferred"));
    }
