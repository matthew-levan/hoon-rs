use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use parser::{definitions_with_metadata, parse_with_metadata, DefinitionKind, ImportKind};
use tower_lsp::lsp_types::{DocumentSymbol, Location, Position, SymbolKind, Url};
use tracing::{debug, trace, warn};
use walkdir::WalkDir;

use crate::position_mapper::range_from_byte_offsets;

pub struct WorkspaceIndex {
    inner: RwLock<WorkspaceIndexData>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IndexRunStats {
    pub scanned_files: usize,
    pub indexed_files: usize,
}

#[derive(Default)]
struct WorkspaceIndexData {
    by_symbol: HashMap<String, Vec<Location>>,
    by_file: HashMap<Url, IndexedFile>,
    by_path: HashMap<PathBuf, Url>,
}

#[derive(Clone, Default)]
struct IndexedFile {
    defs: Vec<(String, Location)>,
    imports: Vec<ImportBinding>,
    path: Option<PathBuf>,
    stdlib_rank: u8,
}

#[derive(Clone)]
struct ImportBinding {
    visible_symbol: Option<String>,
    target: String,
    kind: ImportKind,
}

impl WorkspaceIndex {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(WorkspaceIndexData::default()),
        }
    }

    pub fn upsert_document(&self, uri: &Url, text: &str) {
        let (defs, imports, path) = file_metadata(uri, text);
        let stdlib_rank = path.as_deref().and_then(stdlib_rank_for_path).unwrap_or(0);
        let new_file = IndexedFile {
            defs: defs.clone(),
            imports,
            path,
            stdlib_rank,
        };

        let mut inner = self.inner.write().expect("workspace index lock");
        if let Some(old_file) = inner.by_file.insert(uri.clone(), new_file.clone()) {
            remove_defs(&mut inner.by_symbol, old_file.defs);
            remove_file_path(&mut inner.by_path, &old_file.path);
        }

        for (name, loc) in defs {
            inner.by_symbol.entry(name).or_default().push(loc);
        }
        if let Some(path) = new_file.path {
            inner.by_path.insert(path.clone(), uri.clone());
            if let Ok(canon) = path.canonicalize() {
                inner.by_path.insert(canon, uri.clone());
            }
        }
    }

    pub fn definitions_for_symbol(&self, symbol: &str) -> Vec<Location> {
        let inner = self.inner.read().expect("workspace index lock");
        let mut defs = inner.by_symbol.get(symbol).cloned().unwrap_or_default();
        defs.sort_by(|a, b| {
            stdlib_rank_for_location(&inner, b).cmp(&stdlib_rank_for_location(&inner, a))
        });
        defs
    }

    /// Returns true if the given location points to a bundled stdlib file.
    pub fn is_stdlib_location(&self, loc: &Location) -> bool {
        let inner = self.inner.read().expect("workspace index lock");
        stdlib_rank_for_location(&inner, loc) > 0
    }

    pub fn imported_definitions_for_symbol(
        &self,
        current_uri: &Url,
        symbol: &str,
    ) -> Vec<Location> {
        let inner = self.inner.read().expect("workspace index lock");
        let Some(current_file_uri) = canonicalize_current_uri(&inner, current_uri) else {
            debug!(uri = %current_uri, symbol, "import lookup: current uri not indexed");
            return Vec::new();
        };
        let Some(current_file) = inner.by_file.get(&current_file_uri) else {
            debug!(uri = %current_uri, symbol, canonical_uri = %current_file_uri, "import lookup: canonical uri missing file entry");
            return Vec::new();
        };
        let Some(current_path) = current_file.path.as_ref() else {
            debug!(uri = %current_uri, symbol, "import lookup: file has no path");
            return Vec::new();
        };
        let workspace_root = workspace_root_for(current_path);
        trace!(
            uri = %current_uri,
            canonical_uri = %current_file_uri,
            symbol,
            current_path = %current_path.display(),
            workspace_root = %workspace_root.display(),
            imports = current_file.imports.len(),
            "import lookup start"
        );

        let mut out = Vec::new();
        let mut explicit_imports = Vec::new();
        let mut wildcard_imports = Vec::new();
        for import in &current_file.imports {
            if import.visible_symbol.is_some() {
                explicit_imports.push(import);
            } else {
                wildcard_imports.push(import);
            }
        }
        debug!(
            uri = %current_uri,
            symbol,
            explicit_imports = explicit_imports.len(),
            wildcard_imports = wildcard_imports.len(),
            "import buckets"
        );

        for import in explicit_imports
            .into_iter()
            .chain(wildcard_imports.into_iter())
        {
            if let Some(visible) = &import.visible_symbol {
                if visible != symbol {
                    continue;
                }
            }
            debug!(
                uri = %current_uri,
                symbol,
                import_visible = ?import.visible_symbol,
                import_target = %import.target,
                import_kind = ?import.kind,
                "considering import binding"
            );

            if import.target.is_empty() {
                continue;
            }

            let candidates = resolve_import_candidates_from_index(
                current_path, &workspace_root, import, &inner.by_path,
            );
            debug!(
                uri = %current_uri,
                symbol,
                import_target = %import.target,
                candidates = ?candidates,
                "resolved import candidates"
            );
            trace!(
                uri = %current_uri,
                symbol,
                import_symbol = ?import.visible_symbol,
                import_target = %import.target,
                candidates = candidates.len(),
                "import candidate resolution"
            );
            for cand_uri in candidates {
                let defs = if let Some(file) = inner.by_file.get(&cand_uri) {
                    file.defs.clone()
                } else {
                    trace!(candidate = %cand_uri, "import candidate uri missing from by_file");
                    definitions_for_uri_from_disk(&cand_uri)
                };
                let mut matched_in_file = false;
                for (name, loc) in &defs {
                    if name == symbol {
                        matched_in_file = true;
                        out.push(loc.clone());
                    }
                }
                if !matched_in_file && import.visible_symbol.as_deref() == Some(symbol) {
                    if let Some(module_loc) = module_entry_location(&defs, &cand_uri) {
                        out.push(module_loc);
                    }
                }
            }

            if !out.is_empty() {
                break;
            }
        }

        out.sort_by(|a, b| {
            a.uri
                .as_str()
                .cmp(b.uri.as_str())
                .then(a.range.start.line.cmp(&b.range.start.line))
                .then(a.range.start.character.cmp(&b.range.start.character))
        });
        out.dedup();
        debug!(uri = %current_uri, symbol, matches = out.len(), "import lookup complete");
        out
    }

    pub fn local_definitions_for_symbol(&self, current_uri: &Url, symbol: &str) -> Vec<Location> {
        let inner = self.inner.read().expect("workspace index lock");
        let Some(current_file_uri) = canonicalize_current_uri(&inner, current_uri) else {
            return Vec::new();
        };
        let Some(current_file) = inner.by_file.get(&current_file_uri) else {
            return Vec::new();
        };

        current_file
            .defs
            .iter()
            .filter_map(|(name, loc)| (name == symbol).then_some(loc.clone()))
            .collect()
    }

    pub fn imported_member_definitions_for_symbol(
        &self,
        current_uri: &Url,
        module_symbol: &str,
        member_symbol: &str,
    ) -> Vec<Location> {
        let inner = self.inner.read().expect("workspace index lock");
        let Some(current_file_uri) = canonicalize_current_uri(&inner, current_uri) else {
            debug!(uri = %current_uri, module_symbol, member_symbol, "member lookup: current uri not indexed");
            return Vec::new();
        };
        let Some(current_file) = inner.by_file.get(&current_file_uri) else {
            return Vec::new();
        };
        let Some(current_path) = current_file.path.as_ref() else {
            return Vec::new();
        };
        let workspace_root = workspace_root_for(current_path);
        trace!(
            uri = %current_uri,
            canonical_uri = %current_file_uri,
            module_symbol,
            member_symbol,
            current_path = %current_path.display(),
            "member lookup start"
        );

        let mut out = Vec::new();
        for import in &current_file.imports {
            if import.visible_symbol.as_deref() != Some(module_symbol) {
                continue;
            }

            let candidates = resolve_import_candidates_from_index(
                current_path, &workspace_root, import, &inner.by_path,
            );
            trace!(
                uri = %current_uri,
                module_symbol,
                member_symbol,
                import_target = %import.target,
                candidates = candidates.len(),
                "member import candidates"
            );
            for cand_uri in candidates {
                let defs = if let Some(file) = inner.by_file.get(&cand_uri) {
                    file.defs.clone()
                } else {
                    definitions_for_uri_from_disk(&cand_uri)
                };
                for (name, loc) in &defs {
                    if name == member_symbol {
                        out.push(loc.clone());
                    }
                }
            }
            if !out.is_empty() {
                break;
            }
        }

        out.sort_by(|a, b| {
            a.uri
                .as_str()
                .cmp(b.uri.as_str())
                .then(a.range.start.line.cmp(&b.range.start.line))
                .then(a.range.start.character.cmp(&b.range.start.character))
        });
        out.dedup();
        debug!(
            uri = %current_uri,
            module_symbol,
            member_symbol,
            matches = out.len(),
            "member lookup complete"
        );
        out
    }

    pub fn index_workspace(
        &self,
        root: &Path,
        max_workspace_files: usize,
        follow_symlinks: bool,
    ) -> IndexRunStats {
        let mut scanned = 0usize;
        let mut indexed = 0usize;
        let mut seen_uris = HashSet::new();
        let root_canon = root.canonicalize().ok();
        let files = collect_workspace_hoon_files(root, max_workspace_files, follow_symlinks);
        for path in files {
            scanned += 1;
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            let Ok(uri) = Url::from_file_path(&path) else {
                continue;
            };
            seen_uris.insert(uri.clone());
            self.upsert_document(&uri, &text);
            indexed += 1;
        }

        self.remove_stale_root_entries(root, root_canon.as_deref(), &seen_uris);

        IndexRunStats {
            scanned_files: scanned,
            indexed_files: indexed,
        }
    }

    fn remove_stale_root_entries(
        &self,
        root: &Path,
        root_canon: Option<&Path>,
        seen_uris: &HashSet<Url>,
    ) {
        let mut inner = self.inner.write().expect("workspace index lock");
        let stale_uris = inner
            .by_file
            .iter()
            .filter_map(|(uri, file)| {
                let path = file.path.as_ref()?;
                if !path_is_under_root(path, root, root_canon) {
                    return None;
                }
                if seen_uris.contains(uri) {
                    return None;
                }
                Some(uri.clone())
            })
            .collect::<Vec<_>>();

        for uri in stale_uris {
            if let Some(old_file) = inner.by_file.remove(&uri) {
                remove_defs(&mut inner.by_symbol, old_file.defs);
                remove_file_path(&mut inner.by_path, &old_file.path);
            }
        }
    }
}

impl Default for WorkspaceIndex {
    fn default() -> Self {
        Self::new()
    }
}

pub fn word_at_position(text: &str, byte_offset: usize) -> Option<String> {
    if text.is_empty() {
        return None;
    }

    let bytes = text.as_bytes();
    let mut idx = byte_offset.min(text.len());
    if idx == text.len() && idx > 0 {
        idx -= 1;
    }

    if !is_ident_byte(bytes.get(idx).copied()) {
        return None;
    }

    let mut start = idx;
    while start > 0 && is_ident_byte(bytes.get(start - 1).copied()) {
        start -= 1;
    }

    let mut end = idx + 1;
    while end < text.len() && is_ident_byte(bytes.get(end).copied()) {
        end += 1;
    }

    text.get(start..end).map(ToOwned::to_owned)
}

pub fn extract_defs(text: &str) -> Vec<(String, tower_lsp::lsp_types::Range)> {
    definitions_with_metadata(text)
        .into_iter()
        .map(|def| {
            (
                def.name,
                range_from_byte_offsets(text, def.start_byte as usize, def.end_byte as usize),
            )
        })
        .collect()
}

fn extract_defs_with_kind(
    text: &str,
) -> Vec<(String, tower_lsp::lsp_types::Range, DefinitionKind)> {
    definitions_with_metadata(text)
        .into_iter()
        .map(|def| {
            (
                def.name,
                range_from_byte_offsets(text, def.start_byte as usize, def.end_byte as usize),
                def.kind,
            )
        })
        .collect()
}

#[allow(deprecated)]
pub fn document_symbols(text: &str) -> Vec<DocumentSymbol> {
    extract_defs_with_kind(text)
        .into_iter()
        .map(|(name, range, kind)| DocumentSymbol {
            name,
            detail: Some(
                match kind {
                    DefinitionKind::Arm => "++ arm",
                    DefinitionKind::Type => "+$ type",
                }
                .to_string(),
            ),
            kind: match kind {
                DefinitionKind::Arm => SymbolKind::FUNCTION,
                DefinitionKind::Type => SymbolKind::STRUCT,
            },
            tags: None,
            deprecated: None,
            range,
            selection_range: range,
            children: None,
        })
        .collect()
}

#[cfg(test)]
pub fn local_definitions_for_symbol(uri: &Url, text: &str, symbol: &str) -> Vec<Location> {
    extract_defs(text)
        .into_iter()
        .filter(|(name, _)| name == symbol)
        .map(|(_, range)| Location {
            uri: uri.clone(),
            range,
        })
        .collect()
}

pub fn workspace_definitions_for_symbol(
    current_uri: &Url,
    symbol: &str,
    max_workspace_files: usize,
    follow_symlinks: bool,
) -> Vec<Location> {
    let Ok(current_path) = current_uri.to_file_path() else {
        return Vec::new();
    };

    let root = workspace_root_for(&current_path);

    let mut out = Vec::new();
    let files = collect_workspace_hoon_files(&root, max_workspace_files, follow_symlinks);
    for path in files {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(uri) = Url::from_file_path(&path) else {
            continue;
        };

        for (name, range) in extract_defs(&text) {
            if name == symbol {
                out.push(Location {
                    uri: uri.clone(),
                    range,
                });
            }
        }
    }

    out.sort_by(|a, b| {
        a.uri
            .as_str()
            .cmp(b.uri.as_str())
            .then(a.range.start.line.cmp(&b.range.start.line))
            .then(a.range.start.character.cmp(&b.range.start.character))
    });
    out.dedup();
    out
}

pub fn byte_offset_at_position(text: &str, position: Position) -> usize {
    let target_line = usize::try_from(position.line).unwrap_or(usize::MAX);
    let mut byte_start = 0usize;

    for (line_idx, line) in text.split_inclusive('\n').enumerate() {
        if line_idx == target_line {
            let target_utf16 = usize::try_from(position.character).unwrap_or(usize::MAX);
            let mut utf16 = 0usize;
            let mut bytes = 0usize;
            for ch in line.chars() {
                if utf16 >= target_utf16 {
                    break;
                }
                let w = ch.len_utf16();
                if utf16 + w > target_utf16 {
                    break;
                }
                utf16 += w;
                bytes += ch.len_utf8();
            }
            return byte_start + bytes;
        }
        byte_start += line.len();
    }

    text.len()
}

pub fn workspace_root_for(path: &Path) -> PathBuf {
    if let Some(root) = desk_root_for(path) {
        return root;
    }
    if let Ok(canon) = path.canonicalize() {
        if let Some(root) = desk_root_for(&canon) {
            return root;
        }
    }

    let mut current = path.parent().map(Path::to_path_buf);
    while let Some(dir) = current.clone() {
        if dir.join(".git").exists() {
            return dir;
        }
        current = dir.parent().map(Path::to_path_buf);
    }

    path.parent().unwrap_or(path).to_path_buf()
}

fn path_to_wer(path: &Path) -> Vec<String> {
    path.iter()
        .map(|part| part.to_string_lossy().into_owned())
        .collect()
}

fn is_ident_byte(b: Option<u8>) -> bool {
    matches!(
        b,
        Some(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_')
    )
}

fn remove_defs(by_symbol: &mut HashMap<String, Vec<Location>>, defs: Vec<(String, Location)>) {
    for (name, loc) in defs {
        let mut should_remove = false;
        if let Some(existing) = by_symbol.get_mut(&name) {
            existing.retain(|entry| entry.uri != loc.uri || entry.range != loc.range);
            should_remove = existing.is_empty();
        }
        if should_remove {
            by_symbol.remove(&name);
        }
    }
}

fn remove_file_path(by_path: &mut HashMap<PathBuf, Url>, path: &Option<PathBuf>) {
    if let Some(path) = path {
        by_path.remove(path);
        if let Ok(canon) = path.canonicalize() {
            by_path.remove(&canon);
        }
    }
}

fn stdlib_rank_for_location(inner: &WorkspaceIndexData, loc: &Location) -> u8 {
    inner
        .by_file
        .get(&loc.uri)
        .map(|file| file.stdlib_rank)
        .unwrap_or(0)
}

fn stdlib_rank_for_path(path: &Path) -> Option<u8> {
    let mut has_embedded_root = false;
    for part in path.components() {
        if part.as_os_str() == "hoon-lsp-stdlib" {
            has_embedded_root = true;
            break;
        }
    }
    if !has_embedded_root {
        return None;
    }

    let file = path.file_name()?.to_string_lossy();
    match file.as_ref() {
        "hoon.hoon" => Some(4),
        "arvo.hoon" => Some(3),
        "lull.hoon" => Some(2),
        "zuse.hoon" => Some(1),
        _ => None,
    }
}

fn file_metadata(
    uri: &Url,
    text: &str,
) -> (Vec<(String, Location)>, Vec<ImportBinding>, Option<PathBuf>) {
    let Ok(path) = uri.to_file_path() else {
        let defs = extract_defs(text)
            .into_iter()
            .map(|(name, range)| {
                (
                    name,
                    Location {
                        uri: uri.clone(),
                        range,
                    },
                )
            })
            .collect();
        return (defs, Vec::new(), None);
    };

    let wer = path_to_wer(&path);
    let parsed = parse_with_metadata(text, wer, false);
    let defs = parsed
        .definitions
        .iter()
        .map(|def| {
            (
                def.name.clone(),
                Location {
                    uri: uri.clone(),
                    range: range_from_byte_offsets(
                        text, def.start_byte as usize, def.end_byte as usize,
                    ),
                },
            )
        })
        .collect();
    let definitions_count = parsed.definitions.len();
    let import_decl_count = parsed.imports.len();
    let imports: Vec<ImportBinding> = parsed
        .imports
        .into_iter()
        .flat_map(|decl| {
            let kind = decl.kind.clone();
            decl.bindings
                .into_iter()
                .map(move |binding| (binding, kind.clone()))
        })
        .map(|(binding, kind)| ImportBinding {
            visible_symbol: binding.visible_symbol,
            target: binding.target,
            kind,
        })
        .collect();
    trace!(
        uri = %uri,
        path = %path.display(),
        definitions = definitions_count,
        import_decls = import_decl_count,
        import_bindings = imports.len(),
        "file metadata parsed"
    );
    (defs, imports, Some(path))
}

fn resolve_import_candidates_from_index(
    current_path: &Path,
    workspace_root: &Path,
    import: &ImportBinding,
    by_path: &HashMap<PathBuf, Url>,
) -> Vec<Url> {
    let current_dir = current_path.parent().unwrap_or(current_path);
    let desk_root = desk_root_for(current_path);
    let target = import.target.trim();
    if target.is_empty() {
        return Vec::new();
    }

    let mut candidate_rel = target.trim_start_matches('/').to_string();
    if !candidate_rel.ends_with(".hoon") {
        candidate_rel.push_str(".hoon");
    }

    let target_has_path = candidate_rel.contains('/');
    let mut candidates = Vec::new();
    if target_has_path {
        candidates.push(current_dir.join(&candidate_rel));
        if let Some(root) = &desk_root {
            candidates.push(root.join(&candidate_rel));
        }
        candidates.push(workspace_root.join(&candidate_rel));
    } else {
        match import.kind {
            ImportKind::Plus => {
                if let Some(root) = &desk_root {
                    candidates.push(root.join("lib").join(&candidate_rel));
                }
                candidates.push(workspace_root.join("lib").join(&candidate_rel));
            }
            ImportKind::Minus => {
                if let Some(root) = &desk_root {
                    candidates.push(root.join("sur").join(&candidate_rel));
                }
                candidates.push(workspace_root.join("sur").join(&candidate_rel));
            }
            _ => {
                if let Some(root) = &desk_root {
                    candidates.push(root.join(&candidate_rel));
                }
                candidates.push(workspace_root.join(&candidate_rel));
            }
        }

        // Compatibility fallback for non-desk layouts.
        candidates.push(current_dir.join(&candidate_rel));
        if let Some(root) = &desk_root {
            candidates.push(root.join(&candidate_rel));
        }
        candidates.push(workspace_root.join(&candidate_rel));
    }

    let mut out = Vec::new();
    let mut seen_uris = HashSet::new();
    let mut seen = HashSet::new();
    for candidate in candidates {
        if !seen.insert(candidate.clone()) {
            continue;
        }
        if let Some(uri) = lookup_uri_for_path(by_path, &candidate) {
            if seen_uris.insert(uri.as_str().to_string()) {
                out.push(uri.clone());
            }
            continue;
        }
        if candidate.exists() {
            if let Ok(uri) = Url::from_file_path(&candidate) {
                if seen_uris.insert(uri.as_str().to_string()) {
                    out.push(uri);
                }
            }
        }
    }
    out
}

fn lookup_uri_for_path<'a>(by_path: &'a HashMap<PathBuf, Url>, path: &Path) -> Option<&'a Url> {
    if let Some(uri) = by_path.get(path) {
        return Some(uri);
    }
    let canon = path.canonicalize().ok()?;
    by_path.get(&canon)
}

fn canonicalize_current_uri(inner: &WorkspaceIndexData, current_uri: &Url) -> Option<Url> {
    if inner.by_file.contains_key(current_uri) {
        return Some(current_uri.clone());
    }
    let path = current_uri.to_file_path().ok()?;
    lookup_uri_for_path(&inner.by_path, &path).cloned()
}

fn module_entry_location(defs: &[(String, Location)], uri: &Url) -> Option<Location> {
    if let Some((_, loc)) = defs.first() {
        return Some(loc.clone());
    }
    Some(Location {
        uri: uri.clone(),
        range: range_from_byte_offsets("", 0, 0),
    })
}

fn definitions_for_uri_from_disk(uri: &Url) -> Vec<(String, Location)> {
    let Ok(path) = uri.to_file_path() else {
        return Vec::new();
    };
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    extract_defs(&text)
        .into_iter()
        .map(|(name, range)| {
            (
                name,
                Location {
                    uri: uri.clone(),
                    range,
                },
            )
        })
        .collect()
}

fn desk_root_for(path: &Path) -> Option<PathBuf> {
    let mut dir = path.parent();
    while let Some(current) = dir {
        let has_app = current.join("app").is_dir();
        let has_lib = current.join("lib").is_dir();
        let has_sur = current.join("sur").is_dir();
        let has_web = current.join("web").is_dir();
        if has_app || has_lib || has_sur || has_web {
            return Some(current.to_path_buf());
        }
        dir = current.parent();
    }
    None
}

const MAX_TRAVERSAL_DEPTH: usize = 64;
const MAX_VISITED_DIRS: usize = 100_000;

fn collect_workspace_hoon_files(
    root: &Path,
    max_workspace_files: usize,
    follow_symlinks: bool,
) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut seen_file_paths = HashSet::new();
    let mut seen_dirs = HashSet::new();
    let mut iter = WalkDir::new(root).follow_links(follow_symlinks).into_iter();

    while let Some(next) = iter.next() {
        let entry = match next {
            Ok(entry) => entry,
            Err(err) => {
                warn!(error = %err, "workspace index traversal entry error");
                continue;
            }
        };

        if entry.depth() > MAX_TRAVERSAL_DEPTH {
            if entry.file_type().is_dir() {
                warn!(
                    path = %entry.path().display(),
                    depth = entry.depth(),
                    max_depth = MAX_TRAVERSAL_DEPTH,
                    "workspace index traversal depth guard"
                );
                iter.skip_current_dir();
            }
            continue;
        }

        if entry.file_type().is_dir() {
            if follow_symlinks {
                if seen_dirs.len() >= MAX_VISITED_DIRS {
                    warn!(
                        path = %entry.path().display(),
                        max_dirs = MAX_VISITED_DIRS,
                        "workspace index traversal visited-dir guard"
                    );
                    iter.skip_current_dir();
                    continue;
                }

                if let Ok(canon) = entry.path().canonicalize() {
                    if !seen_dirs.insert(canon) {
                        warn!(
                            path = %entry.path().display(),
                            "workspace index symlink/cycle guard"
                        );
                        iter.skip_current_dir();
                        continue;
                    }
                }
            }
            continue;
        }

        if !entry.file_type().is_file() {
            continue;
        }
        if entry.path().extension().and_then(|s| s.to_str()) != Some("hoon") {
            continue;
        }

        let path = entry.path();
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if !seen_file_paths.insert(canonical) {
            continue;
        }
        out.push(path.to_path_buf());
        if out.len() >= max_workspace_files {
            warn!(
                root = %root.display(),
                cap = max_workspace_files,
                "workspace index file-cap guard"
            );
            break;
        }
    }

    out
}

fn path_is_under_root(path: &Path, root: &Path, root_canon: Option<&Path>) -> bool {
    if path.starts_with(root) {
        return true;
    }
    let Some(root_canon) = root_canon else {
        return false;
    };
    match path.canonicalize() {
        Ok(canon) => canon.starts_with(root_canon),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    use std::path::PathBuf;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn imported_definition_resolves_across_files() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).expect("git marker");

        let main_path = root.join("main.hoon");
        let foo_path = root.join("foo.hoon");
        let main_text = "/+ foo\n++  main\n  foo\n";
        let foo_text = "++  foo\n  ~\n";
        fs::write(&main_path, main_text).expect("write main");
        fs::write(&foo_path, foo_text).expect("write foo");

        let main_uri = Url::from_file_path(&main_path).expect("file uri");
        let index = WorkspaceIndex::new();
        index.index_workspace(root, 100, true);
        let defs = index.imported_definitions_for_symbol(&main_uri, "foo");

        assert!(!defs.is_empty());
        assert!(defs
            .iter()
            .any(|loc| loc.uri == Url::from_file_path(&foo_path).expect("foo uri")));
    }

    #[test]
    fn local_definitions_resolve_symbol() {
        let uri = Url::parse("file:///tmp/test.hoon").expect("valid url");
        let text = "++  foo\n  ~\n";
        let defs = local_definitions_for_symbol(&uri, text, "foo");
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].uri, uri);
    }

    #[test]
    fn document_symbol_extracts_top_level_defs() {
        let text = "++  alpha\n  ~\n++  beta\n  ~\n";
        let syms = document_symbols(text);
        assert_eq!(syms.len(), 2);
        assert_eq!(syms[0].name, "alpha");
        assert_eq!(syms[1].name, "beta");
    }

    #[test]
    fn workspace_index_collects_defs_across_files() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).expect("git marker");
        fs::write(root.join("a.hoon"), "++  foo\n  ~\n").expect("write a");
        fs::write(root.join("b.hoon"), "++  foo\n  ~\n").expect("write b");

        let index = WorkspaceIndex::new();
        let stats = index.index_workspace(root, 100, true);
        let defs = index.definitions_for_symbol("foo");
        assert_eq!(stats.scanned_files, 2);
        assert_eq!(stats.indexed_files, 2);
        assert_eq!(defs.len(), 2);
    }

    #[test]
    fn workspace_index_resolves_imported_defs_without_scan() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).expect("git marker");

        let main_path = root.join("main.hoon");
        let foo_path = root.join("foo.hoon");
        fs::write(&main_path, "/+ foo\n++  main\n  foo\n").expect("write main");
        fs::write(&foo_path, "++  foo\n  ~\n").expect("write foo");

        let main_uri = Url::from_file_path(&main_path).expect("main uri");
        let foo_uri = Url::from_file_path(&foo_path).expect("foo uri");

        let index = WorkspaceIndex::new();
        let stats = index.index_workspace(root, 100, true);
        assert_eq!(stats.scanned_files, 2);
        assert_eq!(stats.indexed_files, 2);
        let defs = index.imported_definitions_for_symbol(&main_uri, "foo");

        assert!(!defs.is_empty());
        assert!(defs.iter().any(|loc| loc.uri == foo_uri));
    }

    #[test]
    fn imported_definitions_use_first_matching_import() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).expect("git marker");
        fs::create_dir_all(root.join("lib")).expect("lib dir");
        fs::create_dir_all(root.join("other")).expect("other dir");

        let main_path = root.join("main.hoon");
        let lib_foo_path = root.join("lib").join("foo.hoon");
        let other_foo_path = root.join("other").join("foo.hoon");
        fs::write(&main_path, "/+ lib/foo\n/+ other/foo\n++  main\n  foo\n").expect("write main");
        fs::write(&lib_foo_path, "++  foo\n  ~\n").expect("write lib foo");
        fs::write(&other_foo_path, "++  foo\n  ~\n").expect("write other foo");

        let main_uri = Url::from_file_path(&main_path).expect("main uri");
        let lib_foo_uri = Url::from_file_path(&lib_foo_path).expect("lib foo uri");
        let other_foo_uri = Url::from_file_path(&other_foo_path).expect("other foo uri");

        let index = WorkspaceIndex::new();
        let stats = index.index_workspace(root, 100, true);
        assert_eq!(stats.scanned_files, 3);
        assert_eq!(stats.indexed_files, 3);
        let defs = index.imported_definitions_for_symbol(&main_uri, "foo");

        assert!(!defs.is_empty());
        assert!(defs.iter().any(|loc| loc.uri == lib_foo_uri));
        assert!(!defs.iter().any(|loc| loc.uri == other_foo_uri));
    }

    #[test]
    fn imported_definitions_prefer_explicit_binding_before_wildcard() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).expect("git marker");
        fs::create_dir_all(root.join("lib")).expect("lib dir");
        fs::create_dir_all(root.join("alt")).expect("alt dir");

        let main_path = root.join("main.hoon");
        let lib_helper_path = root.join("lib").join("helper.hoon");
        let alt_path = root.join("alt.hoon");
        fs::write(
            &main_path, "/= *  /alt\n/= helper  /lib/helper\n++  main\n  helper\n",
        )
        .expect("write main");
        fs::write(&lib_helper_path, "++  helper\n  ~\n").expect("write helper");
        fs::write(&alt_path, "++  helper\n  ~\n").expect("write alt");

        let main_uri = Url::from_file_path(&main_path).expect("main uri");
        let helper_uri = Url::from_file_path(&lib_helper_path).expect("helper uri");
        let alt_uri = Url::from_file_path(&alt_path).expect("alt uri");

        let index = WorkspaceIndex::new();
        let stats = index.index_workspace(root, 100, true);
        assert_eq!(stats.scanned_files, 3);
        assert_eq!(stats.indexed_files, 3);

        let defs = index.imported_definitions_for_symbol(&main_uri, "helper");
        assert!(!defs.is_empty());
        assert!(defs.iter().any(|loc| loc.uri == helper_uri));
        assert!(!defs.iter().any(|loc| loc.uri == alt_uri));
    }

    #[test]
    fn pathlike_import_does_not_fallback_to_basename_match() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).expect("git marker");
        fs::create_dir_all(root.join("lib")).expect("lib dir");

        let main_path = root.join("main.hoon");
        let stray_helper_path = root.join("helper.hoon");
        fs::write(&main_path, "/= helper  /lib/helper\n++  main\n  helper\n").expect("write main");
        fs::write(&stray_helper_path, "++  helper\n  ~\n").expect("write helper");

        let main_uri = Url::from_file_path(&main_path).expect("main uri");

        let index = WorkspaceIndex::new();
        let stats = index.index_workspace(root, 100, true);
        assert_eq!(stats.scanned_files, 2);
        assert_eq!(stats.indexed_files, 2);

        let defs = index.imported_definitions_for_symbol(&main_uri, "helper");
        assert!(defs.is_empty());
    }

    #[test]
    fn equals_import_resolves_by_alias() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).expect("git marker");
        fs::create_dir_all(root.join("lib")).expect("lib dir");

        let main_path = root.join("main.hoon");
        let helper_path = root.join("lib").join("helper.hoon");
        fs::write(&main_path, "/= h  /lib/helper\n++  main\n  h\n").expect("write main");
        fs::write(&helper_path, "++  h\n  ~\n").expect("write helper");

        let main_uri = Url::from_file_path(&main_path).expect("main uri");
        let helper_uri = Url::from_file_path(&helper_path).expect("helper uri");

        let index = WorkspaceIndex::new();
        let stats = index.index_workspace(root, 100, true);
        assert_eq!(stats.scanned_files, 2);
        assert_eq!(stats.indexed_files, 2);

        let defs = index.imported_definitions_for_symbol(&main_uri, "h");
        assert!(!defs.is_empty());
        assert!(defs.iter().any(|loc| loc.uri == helper_uri));
    }

    #[test]
    fn equals_star_import_resolves_symbols_from_target_file() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).expect("git marker");
        fs::create_dir_all(root.join("lib")).expect("lib dir");

        let main_path = root.join("main.hoon");
        let helper_path = root.join("lib").join("helper.hoon");
        fs::write(&main_path, "/= *  /lib/helper\n++  main\n  calc\n").expect("write main");
        fs::write(&helper_path, "++  calc\n  ~\n").expect("write helper");

        let main_uri = Url::from_file_path(&main_path).expect("main uri");
        let helper_uri = Url::from_file_path(&helper_path).expect("helper uri");

        let index = WorkspaceIndex::new();
        let stats = index.index_workspace(root, 100, true);
        assert_eq!(stats.scanned_files, 2);
        assert_eq!(stats.indexed_files, 2);

        let defs = index.imported_definitions_for_symbol(&main_uri, "calc");
        assert!(!defs.is_empty());
        assert!(defs.iter().any(|loc| loc.uri == helper_uri));
    }

    #[test]
    fn plus_import_supports_whitespace_separated_symbols() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).expect("git marker");

        let main_path = root.join("main.hoon");
        let a_path = root.join("alpha.hoon");
        let b_path = root.join("beta.hoon");
        fs::write(&main_path, "/+ alpha beta\n++  main\n  beta\n").expect("write main");
        fs::write(&a_path, "++  alpha\n  ~\n").expect("write alpha");
        fs::write(&b_path, "++  beta\n  ~\n").expect("write beta");

        let main_uri = Url::from_file_path(&main_path).expect("main uri");
        let beta_uri = Url::from_file_path(&b_path).expect("beta uri");

        let index = WorkspaceIndex::new();
        let stats = index.index_workspace(root, 100, true);
        assert_eq!(stats.scanned_files, 3);
        assert_eq!(stats.indexed_files, 3);

        let defs = index.imported_definitions_for_symbol(&main_uri, "beta");
        assert!(!defs.is_empty());
        assert!(defs.iter().any(|loc| loc.uri == beta_uri));
    }

    #[test]
    fn plus_import_ignores_inline_comments() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).expect("git marker");

        let main_path = root.join("main.hoon");
        let a_path = root.join("alpha.hoon");
        fs::write(
            &main_path, "/+ alpha  :: inline comment\n++  main\n  alpha\n",
        )
        .expect("write main");
        fs::write(&a_path, "++  alpha\n  ~\n").expect("write alpha");

        let main_uri = Url::from_file_path(&main_path).expect("main uri");
        let alpha_uri = Url::from_file_path(&a_path).expect("alpha uri");

        let index = WorkspaceIndex::new();
        let stats = index.index_workspace(root, 100, true);
        assert_eq!(stats.scanned_files, 2);
        assert_eq!(stats.indexed_files, 2);

        let defs = index.imported_definitions_for_symbol(&main_uri, "alpha");
        assert!(!defs.is_empty());
        assert!(defs.iter().any(|loc| loc.uri == alpha_uri));
    }

    #[test]
    fn multiline_plus_alias_resolves_to_lib_module() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).expect("git marker");
        fs::create_dir_all(root.join("app")).expect("app dir");
        fs::create_dir_all(root.join("lib")).expect("lib dir");

        let main_path = root.join("app").join("main.hoon");
        let forum_path = root.join("lib").join("forum.hoon");
        fs::write(&main_path, "/+  lib=forum,\n    seeds\n++  main\n  lib\n").expect("write main");
        fs::write(&forum_path, "++  forum-state\n  ~\n").expect("write forum");

        let main_uri = Url::from_file_path(&main_path).expect("main uri");
        let forum_uri = Url::from_file_path(&forum_path).expect("forum uri");

        let index = WorkspaceIndex::new();
        let stats = index.index_workspace(root, 100, true);
        assert_eq!(stats.scanned_files, 2);
        assert_eq!(stats.indexed_files, 2);

        let defs = index.imported_definitions_for_symbol(&main_uri, "lib");
        assert!(!defs.is_empty());
        assert!(defs.iter().any(|loc| loc.uri == forum_uri));
    }

    #[test]
    fn equals_import_resolves_desk_root_absolute_path() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).expect("git marker");
        fs::create_dir_all(root.join("app")).expect("app dir");
        fs::create_dir_all(root.join("web")).expect("web dir");

        let main_path = root.join("app").join("main.hoon");
        let router_path = root.join("web").join("router.hoon");
        fs::write(&main_path, "/=  router  /web/router\n++  main\n  router\n").expect("write main");
        fs::write(&router_path, "++  route\n  ~\n").expect("write router");

        let main_uri = Url::from_file_path(&main_path).expect("main uri");
        let router_uri = Url::from_file_path(&router_path).expect("router uri");

        let index = WorkspaceIndex::new();
        let stats = index.index_workspace(root, 100, true);
        assert_eq!(stats.scanned_files, 2);
        assert_eq!(stats.indexed_files, 2);

        let defs = index.imported_definitions_for_symbol(&main_uri, "router");
        assert!(!defs.is_empty());
        assert!(defs.iter().any(|loc| loc.uri == router_uri));
    }

    #[test]
    fn workspace_index_corpus_smoke_example_hoon() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../parser/example-hoon");
        let index = WorkspaceIndex::new();
        let stats = index.index_workspace(&root, 5_000, false);
        assert!(stats.scanned_files > 0);
        assert!(stats.indexed_files > 0);
    }

    #[test]
    #[ignore = "expensive corpus smoke; run manually when needed"]
    fn workspace_index_corpus_smoke_groups() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../parser/groups");
        let index = WorkspaceIndex::new();
        let stats = index.index_workspace(&root, 250, false);
        assert!(stats.scanned_files > 0);
        assert!(stats.indexed_files > 0);
    }

    #[test]
    fn workspace_index_corpus_smoke_landscape() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../parser/landscape");
        let index = WorkspaceIndex::new();
        let stats = index.index_workspace(&root, 10_000, false);
        assert!(stats.scanned_files > 0);
        assert!(stats.indexed_files > 0);
    }

    #[test]
    fn workspace_index_prunes_deleted_files() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).expect("git marker");

        let a_path = root.join("a.hoon");
        let b_path = root.join("b.hoon");
        fs::write(&a_path, "++  foo\n  ~\n").expect("write a");
        fs::write(&b_path, "++  foo\n  ~\n").expect("write b");

        let index = WorkspaceIndex::new();
        let stats = index.index_workspace(root, 100, true);
        assert_eq!(stats.scanned_files, 2);
        assert_eq!(stats.indexed_files, 2);
        assert_eq!(index.definitions_for_symbol("foo").len(), 2);

        fs::remove_file(&b_path).expect("remove b");

        let stats = index.index_workspace(root, 100, true);
        assert_eq!(stats.scanned_files, 1);
        assert_eq!(stats.indexed_files, 1);
        let defs = index.definitions_for_symbol("foo");
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].uri, Url::from_file_path(&a_path).expect("a uri"));
    }

    #[test]
    fn workspace_index_prunes_renamed_files() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).expect("git marker");

        let old_path = root.join("old.hoon");
        let new_path = root.join("new.hoon");
        fs::write(&old_path, "++  foo\n  ~\n").expect("write old");

        let index = WorkspaceIndex::new();
        let stats = index.index_workspace(root, 100, true);
        assert_eq!(stats.scanned_files, 1);
        assert_eq!(stats.indexed_files, 1);
        assert_eq!(index.definitions_for_symbol("foo").len(), 1);

        fs::rename(&old_path, &new_path).expect("rename file");

        let stats = index.index_workspace(root, 100, true);
        assert_eq!(stats.scanned_files, 1);
        assert_eq!(stats.indexed_files, 1);
        let defs = index.definitions_for_symbol("foo");
        assert_eq!(defs.len(), 1);
        assert_eq!(
            defs[0].uri,
            Url::from_file_path(&new_path).expect("new uri")
        );
    }

    #[cfg(unix)]
    #[test]
    fn workspace_index_deduplicates_symlinked_files() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).expect("git marker");

        let real_path = root.join("real.hoon");
        let alias_path = root.join("alias.hoon");
        fs::write(&real_path, "++  foo\n  ~\n").expect("write real");
        symlink(&real_path, &alias_path).expect("create symlink");

        let index = WorkspaceIndex::new();
        let stats = index.index_workspace(root, 100, true);
        let defs = index.definitions_for_symbol("foo");

        assert_eq!(stats.scanned_files, 1);
        assert_eq!(stats.indexed_files, 1);
        assert_eq!(defs.len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn workspace_index_handles_symlink_dir_cycle() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).expect("git marker");

        let a = root.join("a");
        let b = a.join("b");
        fs::create_dir_all(&b).expect("nested dirs");
        fs::write(a.join("real.hoon"), "++  foo\n  ~\n").expect("write real");
        symlink(&a, b.join("loop")).expect("create dir symlink cycle");

        let index = WorkspaceIndex::new();
        let stats = index.index_workspace(root, 100, true);
        let defs = index.definitions_for_symbol("foo");

        assert_eq!(stats.scanned_files, 1);
        assert_eq!(stats.indexed_files, 1);
        assert_eq!(defs.len(), 1);
    }

    #[test]
    fn workspace_index_respects_max_workspace_files_cap() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).expect("git marker");
        fs::write(root.join("a.hoon"), "++  a\n  ~\n").expect("write a");
        fs::write(root.join("b.hoon"), "++  b\n  ~\n").expect("write b");
        fs::write(root.join("c.hoon"), "++  c\n  ~\n").expect("write c");

        let index = WorkspaceIndex::new();
        let stats = index.index_workspace(root, 2, true);

        assert_eq!(stats.scanned_files, 2);
        assert_eq!(stats.indexed_files, 2);
    }

    #[test]
    fn workspace_scan_definitions_are_deterministically_sorted() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).expect("git marker");

        let z_path = root.join("z.hoon");
        let a_path = root.join("a.hoon");
        fs::write(&z_path, "++  foo\n  ~\n").expect("write z");
        fs::write(&a_path, "++  foo\n  ~\n").expect("write a");

        let current_uri = Url::from_file_path(root.join("main.hoon")).expect("main uri");
        let defs = workspace_definitions_for_symbol(&current_uri, "foo", 100, true);

        assert_eq!(defs.len(), 2);
        assert_eq!(defs[0].uri, Url::from_file_path(&a_path).expect("a uri"));
        assert_eq!(defs[1].uri, Url::from_file_path(&z_path).expect("z uri"));
    }
}
