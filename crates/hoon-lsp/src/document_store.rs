use dashmap::DashMap;
use ropey::Rope;
use std::sync::Arc;
use tower_lsp::lsp_types::{TextDocumentContentChangeEvent, Url};

use crate::position_mapper::position_utf16_to_char_idx;

#[derive(Clone)]
pub struct DocumentSnapshot {
    pub version: i32,
    pub text: String,
    pub rope: Rope,
}

pub struct DocumentStore {
    docs: DashMap<Url, DocumentSnapshot>,
}

impl DocumentStore {
    pub fn new() -> Self {
        Self {
            docs: DashMap::new(),
        }
    }

    pub fn open(&self, uri: Url, version: i32, text: String) {
        let rope = Rope::from_str(&text);
        self.docs.insert(
            uri.clone(),
            DocumentSnapshot {
                version,
                text,
                rope,
            },
        );
    }

    pub fn close(&self, uri: &Url) {
        self.docs.remove(uri);
    }

    pub fn get(&self, uri: &Url) -> Option<DocumentSnapshot> {
        self.docs.get(uri).map(|entry| entry.value().clone())
    }

    pub fn apply_changes(
        &self,
        uri: &Url,
        version: i32,
        changes: Vec<TextDocumentContentChangeEvent>,
    ) -> Option<DocumentSnapshot> {
        let mut snap = self.get(uri)?;

        for change in changes {
            if let Some(range) = change.range {
                let start = position_utf16_to_char_idx(&snap.rope, range.start);
                let end = position_utf16_to_char_idx(&snap.rope, range.end);
                if start <= end && end <= snap.rope.len_chars() {
                    snap.rope.remove(start..end);
                    snap.rope.insert(start, &change.text);
                }
            } else {
                snap.rope = Rope::from_str(&change.text);
            }
        }

        snap.version = version;
        snap.text = snap.rope.to_string();
        self.docs.insert(uri.clone(), snap.clone());
        Some(snap)
    }
}

impl Default for DocumentStore {
    fn default() -> Self {
        Self::new()
    }
}

pub type SharedDocumentStore = Arc<DocumentStore>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser_adapter::parse_diagnostics;
    use tower_lsp::lsp_types::{Position, Range, Url};

    #[test]
    fn applies_incremental_change() {
        let store = DocumentStore::new();
        let uri = Url::parse("file:///tmp/test.hoon").expect("valid url");
        store.open(uri.clone(), 1, "abc\n".to_string());

        store.apply_changes(
            &uri,
            2,
            vec![TextDocumentContentChangeEvent {
                range: Some(Range {
                    start: Position {
                        line: 0,
                        character: 1,
                    },
                    end: Position {
                        line: 0,
                        character: 2,
                    },
                }),
                range_length: None,
                text: "Z".to_string(),
            }],
        );

        let updated = store.get(&uri).expect("updated snapshot exists");
        assert_eq!(updated.text, "aZc\n");
        assert_eq!(updated.version, 2);
    }

    #[test]
    fn lifecycle_parse_diagnostics_updates_after_change() {
        let store = DocumentStore::new();
        let uri = Url::parse("file:///tmp/diag.hoon").expect("valid url");
        store.open(uri.clone(), 1, "this is not hoon".to_string());

        let snap1 = store.get(&uri).expect("snapshot exists");
        let d1 = parse_diagnostics(&uri, &snap1.text);
        assert!(!d1.is_empty());

        store.apply_changes(
            &uri,
            2,
            vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "++  foo\n  ~\n".to_string(),
            }],
        );
        let snap2 = store.get(&uri).expect("updated snapshot exists");
        let d2 = parse_diagnostics(&uri, &snap2.text);
        assert_eq!(snap2.version, 2);
        assert_eq!(snap2.text, "++  foo\n  ~\n");
        assert!(d2.iter().all(|d| d.message.len() > 0));
    }
}
