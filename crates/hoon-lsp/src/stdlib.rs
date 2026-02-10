use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use tower_lsp::lsp_types::Url;

use crate::symbol_index::WorkspaceIndex;

const ZUSE_SRC: &str = include_str!("../embedded-stdlib/zuse.hoon");
const LULL_SRC: &str = include_str!("../embedded-stdlib/lull.hoon");
const ARVO_SRC: &str = include_str!("../embedded-stdlib/arvo.hoon");
const HOON_SRC: &str = include_str!("../embedded-stdlib/hoon.hoon");

pub fn bootstrap_embedded_stdlib(index: &Arc<WorkspaceIndex>) -> std::io::Result<usize> {
    let root = std::env::temp_dir().join("hoon-lsp-stdlib").join("sys");
    fs::create_dir_all(&root)?;

    let files = [
        ("zuse.hoon", ZUSE_SRC),
        ("lull.hoon", LULL_SRC),
        ("arvo.hoon", ARVO_SRC),
        ("hoon.hoon", HOON_SRC),
    ];

    let mut count = 0usize;
    for (name, text) in files {
        let path = root.join(name);
        write_if_changed(&path, text)?;
        if let Ok(uri) = Url::from_file_path(&path) {
            index.upsert_document(&uri, text);
            count += 1;
        }
    }

    Ok(count)
}

pub fn embedded_stdlib_sources() -> [(&'static str, &'static str); 4] {
    [
        ("hoon.hoon", HOON_SRC),
        ("arvo.hoon", ARVO_SRC),
        ("lull.hoon", LULL_SRC),
        ("zuse.hoon", ZUSE_SRC),
    ]
}

fn write_if_changed(path: &PathBuf, text: &str) -> std::io::Result<()> {
    match fs::read_to_string(path) {
        Ok(existing) if existing == text => Ok(()),
        _ => fs::write(path, text),
    }
}
