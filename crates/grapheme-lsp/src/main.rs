use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct Backend {
    client: Client,
    documents: Mutex<HashMap<Url, String>>,
    workspace_roots: Mutex<Vec<Url>>,
}

#[derive(Clone, Copy)]
struct TransformOpHint {
    module: &'static str,
    op: &'static str,
    arg_name: &'static str,
    return_shape: &'static str,
    summary: &'static str,
}

const TRANSFORM_HINTS: &[TransformOpHint] = &[
    TransformOpHint {
        module: "html",
        op: "to_md",
        arg_name: "html",
        return_shape: "{ text: string, markdown: string }",
        summary: "Converts HTML input to Markdown text.",
    },
    TransformOpHint {
        module: "json",
        op: "parse",
        arg_name: "text",
        return_shape: "JsonValue (array | object | string | number | bool | null)",
        summary: "Parses JSON text into a structured JSON value.",
    },
    TransformOpHint {
        module: "csv",
        op: "to_list",
        arg_name: "text",
        return_shape: "Array<Object<string, string>>",
        summary: "Parses CSV text with headers into row objects.",
    },
    TransformOpHint {
        module: "yaml",
        op: "to_json",
        arg_name: "text",
        return_shape: "JsonValue (converted from YAML)",
        summary: "Parses YAML text and converts it to JSON.",
    },
];

#[derive(Clone, Copy)]
enum DefinitionKind {
    Query,
    Mutation,
    Iterator,
    Subscription,
}

#[derive(Clone)]
struct DefinitionIndex {
    name: String,
    kind: DefinitionKind,
    line: u32,
    start_char: u32,
    end_char: u32,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: Mutex::new(HashMap::new()),
            workspace_roots: Mutex::new(Vec::new()),
        }
    }

    async fn validate_and_publish(&self, uri: Url, text: String) {
        let diagnostics = match grapheme_compiler::parse(&text) {
            Ok(_) => Vec::new(),
            Err(err) => vec![Diagnostic {
                range: full_document_range(&text),
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("grapheme-lsp".to_string()),
                message: err.to_string(),
                related_information: None,
                tags: None,
                data: None,
            }],
        };

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    async fn all_documents_for_index(&self) -> HashMap<Url, String> {
        let roots = self.workspace_roots.lock().await.clone();
        let mut docs = load_workspace_documents(&roots);

        // Open buffers are source of truth over on-disk snapshots.
        let open_docs = self.documents.lock().await.clone();
        for (uri, text) in open_docs {
            docs.insert(uri, text);
        }

        docs
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let roots = workspace_roots_from_initialize(&params);
        *self.workspace_roots.lock().await = roots;

        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "grapheme-lsp".to_string(),
                version: Some("0.1.0".to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                document_formatting_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string()]),
                    ..CompletionOptions::default()
                }),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Left(true)),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                    retrigger_characters: Some(vec![",".to_string()]),
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: None,
                    },
                }),
                definition_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Grapheme LSP initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;

        self.documents.lock().await.insert(uri.clone(), text.clone());
        self.validate_and_publish(uri, text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;

        if let Some(change) = params.content_changes.into_iter().next() {
            let text = change.text;
            self.documents.lock().await.insert(uri.clone(), text.clone());
            self.validate_and_publish(uri, text).await;
        }
    }

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let text = {
            let docs = self.documents.lock().await;
            match docs.get(&uri) {
                Some(content) => content.clone(),
                None => return Ok(None),
            }
        };

        let formatted = simple_format(&text);
        if formatted == text {
            return Ok(None);
        }

        Ok(Some(vec![TextEdit {
            range: full_document_range(&text),
            new_text: formatted,
        }]))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri;
        let position = params.text_document_position_params.position;

        let text = {
            let docs = self.documents.lock().await;
            match docs.get(&uri) {
                Some(content) => content.clone(),
                None => return Ok(None),
            }
        };

        let Some(line) = line_at(&text, position.line as usize) else {
            return Ok(None);
        };

        let Some((module, op)) = op_at_position(line, position.character as usize) else {
            return Ok(None);
        };

        let Some(hint) = transform_hint(module, op) else {
            return Ok(None);
        };

        let markdown = format!(
            "**{}.{}**\n\n{}\n\n- arg: `{}` (string; defaults to pipeline input when omitted)\n- returns: `{}`",
            hint.module, hint.op, hint.summary, hint.arg_name, hint.return_shape
        );

        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: markdown,
            }),
            range: None,
        }))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let text = {
            let docs = self.documents.lock().await;
            match docs.get(&uri) {
                Some(content) => content.clone(),
                None => return Ok(None),
            }
        };

        let Some(line) = line_at(&text, position.line as usize) else {
            return Ok(None);
        };

        let prefix = module_and_op_prefix_at_position(line, position.character as usize);
        let mut items = Vec::new();

        for hint in TRANSFORM_HINTS {
            let include = match prefix {
                Some((module, op_prefix)) => {
                    hint.module == module && hint.op.starts_with(op_prefix)
                }
                None => true,
            };

            if !include {
                continue;
            }

            let (insert_text, insert_format) = match prefix {
                Some(_) => (
                    format!("{}({}: ${{1:\"\"}})", hint.op, hint.arg_name),
                    InsertTextFormat::SNIPPET,
                ),
                None => (
                    format!("{}.{}({}: ${{1:\"\"}})", hint.module, hint.op, hint.arg_name),
                    InsertTextFormat::SNIPPET,
                ),
            };

            items.push(CompletionItem {
                label: format!("{}.{}", hint.module, hint.op),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(format!("returns {}", hint.return_shape)),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!(
                        "{}\n\n- arg: `{}` (string)\n- returns: `{}`",
                        hint.summary, hint.arg_name, hint.return_shape
                    ),
                })),
                insert_text: Some(insert_text),
                insert_text_format: Some(insert_format),
                ..CompletionItem::default()
            });
        }

        if prefix.is_none() {
            items.extend(keyword_completion_items());
        }

        if items.is_empty() {
            return Ok(None);
        }

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let docs_snapshot = self.all_documents_for_index().await;
        let Some(text) = docs_snapshot.get(&uri).cloned() else {
            return Ok(None);
        };

        let Some(line) = line_at(&text, position.line as usize) else {
            return Ok(None);
        };

        let Some(symbol) = symbol_at_position(line, position.character as usize) else {
            return Ok(None);
        };

        // Prefer current document definitions first for local ergonomics.
        let mut ordered_uris = Vec::with_capacity(docs_snapshot.len());
        ordered_uris.push(uri.clone());
        ordered_uris.extend(docs_snapshot.keys().filter(|u| *u != &uri).cloned());

        for doc_uri in ordered_uris {
            let Some(doc_text) = docs_snapshot.get(&doc_uri) else {
                continue;
            };

            if let Some(def) = index_definitions(doc_text).into_iter().find(|d| d.name == symbol) {
                return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                    uri: doc_uri,
                    range: definition_range(&def),
                })));
            }
        }

        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let include_declaration = params.context.include_declaration;

        let docs_snapshot = self.all_documents_for_index().await;
        let Some(current_text) = docs_snapshot.get(&uri).cloned() else {
            return Ok(None);
        };

        let Some(line) = line_at(&current_text, position.line as usize) else {
            return Ok(None);
        };
        let Some(symbol) = symbol_at_position(line, position.character as usize) else {
            return Ok(None);
        };

        let declaration_range = index_definitions(&current_text)
            .into_iter()
            .find(|d| d.name == symbol)
            .map(|d| definition_range(&d));

        let mut locations = Vec::new();
        for (doc_uri, doc_text) in docs_snapshot {
            for range in symbol_occurrences(&doc_text, &symbol) {
                if !include_declaration
                    && doc_uri == uri
                    && declaration_range.as_ref().is_some_and(|decl| ranges_equal(decl, &range))
                {
                    continue;
                }

                locations.push(Location {
                    uri: doc_uri.clone(),
                    range,
                });
            }
        }

        if locations.is_empty() {
            return Ok(None);
        }

        Ok(Some(locations))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let new_name = params.new_name;

        if !is_ident(&new_name) {
            return Ok(None);
        }

        let docs_snapshot = self.all_documents_for_index().await;
        let Some(current_text) = docs_snapshot.get(&uri).cloned() else {
            return Ok(None);
        };

        let Some(line) = line_at(&current_text, position.line as usize) else {
            return Ok(None);
        };
        let Some(symbol) = symbol_at_position(line, position.character as usize) else {
            return Ok(None);
        };

        let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();
        for (doc_uri, doc_text) in docs_snapshot {
            let edits = symbol_occurrences(&doc_text, &symbol)
                .into_iter()
                .map(|range| TextEdit {
                    range,
                    new_text: new_name.clone(),
                })
                .collect::<Vec<_>>();

            if !edits.is_empty() {
                changes.insert(doc_uri, edits);
            }
        }

        if changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }))
    }

    #[allow(deprecated)]
    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        let text = {
            let docs = self.documents.lock().await;
            match docs.get(&uri) {
                Some(content) => content.clone(),
                None => return Ok(None),
            }
        };

        let symbols = index_definitions(&text)
            .into_iter()
            .map(|def| {
                let line_text = text_line(&text, def.line as usize).unwrap_or("");
                let line_end = line_len(line_text);
                let range = Range {
                    start: Position {
                        line: def.line,
                        character: def.start_char,
                    },
                    end: Position {
                        line: def.line,
                        character: def.end_char.max(line_end),
                    },
                };

                let detail = Some(match def.kind {
                    DefinitionKind::Query => "query".to_string(),
                    DefinitionKind::Mutation => "mutation".to_string(),
                    DefinitionKind::Iterator => "iterator".to_string(),
                    DefinitionKind::Subscription => "subscription".to_string(),
                });

                SymbolInformation {
                    name: def.name,
                    kind: SymbolKind::FUNCTION,
                    tags: None,
                    deprecated: None,
                    location: Location {
                        uri: uri.clone(),
                        range,
                    },
                    container_name: detail,
                }
            })
            .collect::<Vec<_>>();

        Ok(Some(DocumentSymbolResponse::Flat(symbols)))
    }

    async fn signature_help(
        &self,
        params: SignatureHelpParams,
    ) -> Result<Option<SignatureHelp>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let text = {
            let docs = self.documents.lock().await;
            match docs.get(&uri) {
                Some(content) => content.clone(),
                None => return Ok(None),
            }
        };

        let Some(line) = line_at(&text, position.line as usize) else {
            return Ok(None);
        };

        let Some((hint, active_parameter)) = active_transform_call_at(line, position.character as usize) else {
            return Ok(None);
        };

        let signature = SignatureInformation {
            label: format!(
                "{}.{}({}: string) -> {}",
                hint.module, hint.op, hint.arg_name, hint.return_shape
            ),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: hint.summary.to_string(),
            })),
            parameters: Some(vec![ParameterInformation {
                label: ParameterLabel::Simple(format!("{}: string", hint.arg_name)),
                documentation: None,
            }]),
            active_parameter: Some(0),
        };

        Ok(Some(SignatureHelp {
            signatures: vec![signature],
            active_signature: Some(0),
            active_parameter: Some(active_parameter),
        }))
    }
}

fn transform_hint(module: &str, op: &str) -> Option<&'static TransformOpHint> {
    TRANSFORM_HINTS
        .iter()
        .find(|hint| hint.module == module && hint.op == op)
}

fn line_at(text: &str, line_number: usize) -> Option<&str> {
    text.lines().nth(line_number)
}

fn module_and_op_prefix_at_position(line: &str, character: usize) -> Option<(&str, &str)> {
    let bytes = line.as_bytes();
    if bytes.is_empty() {
        return None;
    }

    let mut cursor = character.min(bytes.len());
    if cursor == bytes.len() && cursor > 0 {
        cursor -= 1;
    }

    let mut start = cursor;
    while start > 0 && is_ident_or_dot(bytes[start - 1]) {
        start -= 1;
    }

    let mut end = cursor;
    while end < bytes.len() && is_ident_or_dot(bytes[end]) {
        end += 1;
    }

    if start >= end {
        return None;
    }

    let token = &line[start..end];
    let dot = token.find('.')?;
    let module = &token[..dot];
    let op_prefix = &token[dot + 1..];

    if module.is_empty() || !is_ident(module) {
        return None;
    }

    if !op_prefix.is_empty() && !is_ident(op_prefix) {
        return None;
    }

    Some((module, op_prefix))
}

fn op_at_position(line: &str, character: usize) -> Option<(&str, &str)> {
    let (module, op_prefix) = module_and_op_prefix_at_position(line, character)?;
    if op_prefix.is_empty() {
        return None;
    }
    Some((module, op_prefix))
}

fn is_ident_or_dot(byte: u8) -> bool {
    is_ident_char(byte) || byte == b'.'
}

fn is_ident_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn is_ident(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(is_ident_char)
}

fn keyword_completion_items() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "query".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            insert_text: Some("query ${1:Name} {\n  ${2}\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            documentation: Some(Documentation::String(
                "Create a query entrypoint.".to_string(),
            )),
            ..CompletionItem::default()
        },
        CompletionItem {
            label: "iterator".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            insert_text: Some("iterator ${1:Name} on ${2:Any} {\n  ${3}\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            documentation: Some(Documentation::String(
                "Create a reusable iterator pipeline.".to_string(),
            )),
            ..CompletionItem::default()
        },
        CompletionItem {
            label: "flow.branch".to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            insert_text: Some(
                "flow.branch(\n  when: { field: \"${1:status}\", eq: ${2:\"done\"} },\n  then: ${3:return},\n  else: ${4:NextStep}\n)"
                    .to_string(),
            ),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            documentation: Some(Documentation::String(
                "Conditional control transfer to iterator targets.".to_string(),
            )),
            ..CompletionItem::default()
        },
    ]
}

fn symbol_at_position(line: &str, character: usize) -> Option<String> {
    let bytes = line.as_bytes();
    if bytes.is_empty() {
        return None;
    }

    let mut cursor = character.min(bytes.len());
    if cursor == bytes.len() && cursor > 0 {
        cursor -= 1;
    }

    if !is_ident_char(*bytes.get(cursor)?) {
        if cursor == 0 || !is_ident_char(*bytes.get(cursor.saturating_sub(1))?) {
            return None;
        }
        cursor = cursor.saturating_sub(1);
    }

    let mut start = cursor;
    while start > 0 && is_ident_char(bytes[start - 1]) {
        start -= 1;
    }

    let mut end = cursor + 1;
    while end < bytes.len() && is_ident_char(bytes[end]) {
        end += 1;
    }

    let token = &line[start..end];
    if token.is_empty() || !is_ident(token) {
        return None;
    }

    Some(token.to_string())
}

fn symbol_occurrences(text: &str, symbol: &str) -> Vec<Range> {
    let mut out = Vec::new();
    for (line_idx, line) in text.lines().enumerate() {
        for (start, matched) in line.match_indices(symbol) {
            if matched != symbol {
                continue;
            }

            let end = start + symbol.len();
            if !is_symbol_boundary(line, start, end) {
                continue;
            }

            out.push(Range {
                start: Position {
                    line: line_idx as u32,
                    character: start as u32,
                },
                end: Position {
                    line: line_idx as u32,
                    character: end as u32,
                },
            });
        }
    }

    out
}

fn is_symbol_boundary(line: &str, start: usize, end: usize) -> bool {
    let before_ok = if start == 0 {
        true
    } else {
        !is_ident_char(line.as_bytes()[start - 1])
    };

    let after_ok = if end >= line.len() {
        true
    } else {
        !is_ident_char(line.as_bytes()[end])
    };

    before_ok && after_ok
}

fn ranges_equal(a: &Range, b: &Range) -> bool {
    a.start == b.start && a.end == b.end
}

fn index_definitions(text: &str) -> Vec<DefinitionIndex> {
    let mut out = Vec::new();

    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();

        if let Some((name, start_char, end_char)) = parse_definition_name(trimmed, "query", line) {
            out.push(DefinitionIndex {
                name,
                kind: DefinitionKind::Query,
                line: idx as u32,
                start_char,
                end_char,
            });
            continue;
        }

        if let Some((name, start_char, end_char)) = parse_definition_name(trimmed, "mutation", line) {
            out.push(DefinitionIndex {
                name,
                kind: DefinitionKind::Mutation,
                line: idx as u32,
                start_char,
                end_char,
            });
            continue;
        }

        if let Some((name, start_char, end_char)) = parse_definition_name(trimmed, "iterator", line) {
            out.push(DefinitionIndex {
                name,
                kind: DefinitionKind::Iterator,
                line: idx as u32,
                start_char,
                end_char,
            });
            continue;
        }

        if let Some((name, start_char, end_char)) = parse_definition_name(trimmed, "subscription", line) {
            out.push(DefinitionIndex {
                name,
                kind: DefinitionKind::Subscription,
                line: idx as u32,
                start_char,
                end_char,
            });
        }
    }

    out
}

fn parse_definition_name(trimmed_line: &str, keyword: &str, full_line: &str) -> Option<(String, u32, u32)> {
    let rest = trimmed_line.strip_prefix(keyword)?.trim_start();
    let mut end = 0usize;
    for (idx, ch) in rest.char_indices() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            end = idx + ch.len_utf8();
        } else {
            break;
        }
    }

    if end == 0 {
        return None;
    }

    let name = rest[..end].to_string();
    let start_char = full_line.find(&name)? as u32;
    let end_char = start_char + name.len() as u32;

    Some((name, start_char, end_char))
}

fn definition_range(def: &DefinitionIndex) -> Range {
    Range {
        start: Position {
            line: def.line,
            character: def.start_char,
        },
        end: Position {
            line: def.line,
            character: def.end_char,
        },
    }
}

fn active_transform_call_at(line: &str, cursor: usize) -> Option<(&'static TransformOpHint, u32)> {
    if line.is_empty() {
        return None;
    }

    let clamped = cursor.min(line.len());
    let before = &line[..clamped];
    let open_idx = before.rfind('(')?;

    let call_head = before[..open_idx].trim_end();
    let mut end = call_head.len();
    while end > 0 && !is_ident_or_dot(call_head.as_bytes()[end - 1]) {
        end -= 1;
    }
    if end == 0 {
        return None;
    }

    let mut start = end;
    while start > 0 && is_ident_or_dot(call_head.as_bytes()[start - 1]) {
        start -= 1;
    }

    let token = &call_head[start..end];
    let (module, op) = token.split_once('.')?;
    let hint = transform_hint(module, op)?;

    let args_so_far = &before[open_idx + 1..];
    let active_parameter = args_so_far.chars().filter(|c| *c == ',').count() as u32;
    Some((hint, active_parameter))
}

fn text_line(text: &str, line_number: usize) -> Option<&str> {
    text.lines().nth(line_number)
}

fn line_len(line: &str) -> u32 {
    line.chars().count() as u32
}

fn workspace_roots_from_initialize(params: &InitializeParams) -> Vec<Url> {
    let mut roots = Vec::new();

    if let Some(workspace_folders) = &params.workspace_folders {
        for folder in workspace_folders {
            if folder.uri.scheme() == "file" {
                roots.push(folder.uri.clone());
            }
        }
    }

    if roots.is_empty() {
        if let Some(root_uri) = &params.root_uri {
            if root_uri.scheme() == "file" {
                roots.push(root_uri.clone());
            }
        }
    }

    roots
}

fn load_workspace_documents(roots: &[Url]) -> HashMap<Url, String> {
    let mut docs = HashMap::new();

    for root in roots {
        let Ok(path) = root.to_file_path() else {
            continue;
        };

        collect_aql_files(&path, &mut docs);
    }

    docs
}

fn collect_aql_files(dir: &Path, out: &mut HashMap<Url, String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if should_skip_dir(&path) {
                continue;
            }
            collect_aql_files(&path, out);
            continue;
        }

        if !is_aql_file(&path) {
            continue;
        }

        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };

        let Ok(uri) = Url::from_file_path(&path) else {
            continue;
        };

        out.insert(uri, text);
    }
}

fn is_aql_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("aql"))
}

fn should_skip_dir(path: &PathBuf) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };

    matches!(name, ".git" | "target" | "node_modules" | ".vscode")
}

fn simple_format(input: &str) -> String {
    let lines = input
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>();
    let mut out = lines.join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn full_document_range(text: &str) -> Range {
    let mut line_count: u32 = 0;
    let mut last_line_len: u32 = 0;

    for line in text.split('\n') {
        last_line_len = line.chars().count() as u32;
        line_count += 1;
    }

    if line_count == 0 {
        line_count = 1;
    }

    Range {
        start: Position {
            line: 0,
            character: 0,
        },
        end: Position {
            line: line_count.saturating_sub(1),
            character: last_line_len,
        },
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
