//! Grapheme language server binary.
//!
//! Implements diagnostics, completion, formatting, and symbol/navigation features
//! for Grapheme source files over LSP.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use grapheme_signatures::{find_op_spec, op_specs, ArgType, OpSpec};
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
enum DefinitionKind {
    Glyph,
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

#[derive(Clone)]
struct ExecutableSignatureIndex {
    name: String,
    input_type: String,
    output_type: Option<String>,
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
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            work_done_progress_options: WorkDoneProgressOptions {
                                work_done_progress: None,
                            },
                            legend: semantic_tokens_legend(),
                            range: Some(false.into()),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                        },
                    ),
                ),
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
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
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

        if let Some((module, op)) = op_at_position(line, position.character as usize) {
            if let Some(spec) = find_op_spec(module, op) {
                let arg_docs = signature_args_label(spec);
                let return_shape = signature_return_shape(spec);
                let markdown = format!(
                    "**{}.{}**\n\n{}\n\n- args: `{}`\n- returns: `{}`\n- effect: `{}`",
                    spec.module,
                    spec.op,
                    signature_summary(spec),
                    arg_docs,
                    return_shape,
                    signature_effect_label(spec)
                );

                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: markdown,
                    }),
                    range: None,
                }));
            }
        }

        if is_current_context(line, position.character as usize) {
            if let Some((input_type, output_type)) = enclosing_executable_signature(&text, position.line as usize) {
                let fields = resolve_fields_for_type_ref(&text, &uri, &input_type);

                let mut markdown = format!("**$current**\n\n- input type: `{}`", input_type);
                if let Some(output_type) = output_type {
                    markdown.push_str(&format!("\n- output type: `{}`", output_type));
                }

                if !fields.is_empty() {
                    markdown.push_str("\n\nKnown fields:\n");
                    for field in fields {
                        markdown.push_str(&format!("- `{}`\n", field));
                    }
                }

                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: markdown,
                    }),
                    range: None,
                }));
            }
        }

        if let Some((namespace, type_name)) = namespaced_type_at_position(line, position.character as usize) {
            let imported = resolve_imported_type_fields(&text, &uri, &namespace, &type_name);
            if !imported.is_empty() {
                let mut markdown = format!("**{}::{}**\n\nImported type fields:\n", namespace, type_name);
                for field in imported {
                    markdown.push_str(&format!("- `{}`\n", field));
                }

                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: markdown,
                    }),
                    range: None,
                }));
            }
        }

        Ok(None)
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

        for spec in op_specs() {
            let include = match prefix {
                Some((module, op_prefix)) => {
                    spec.module == module && spec.op.starts_with(op_prefix)
                }
                None => true,
            };

            if !include {
                continue;
            }

            let (insert_text, insert_format) = match prefix {
                Some(_) => (
                    completion_insert_text(spec, true),
                    InsertTextFormat::SNIPPET,
                ),
                None => (
                    completion_insert_text(spec, false),
                    InsertTextFormat::SNIPPET,
                ),
            };

            items.push(CompletionItem {
                label: format!("{}.{}", spec.module, spec.op),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(format!(
                    "returns {}",
                    signature_return_shape(spec)
                )),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!(
                        "{}\n\n- args: `{}`\n- returns: `{}`\n- effect: `{}`",
                        signature_summary(spec),
                        signature_args_label(spec),
                        signature_return_shape(spec),
                        signature_effect_label(spec)
                    ),
                })),
                insert_text: Some(insert_text),
                insert_text_format: Some(insert_format),
                ..CompletionItem::default()
            });
        }

        if let Some(field_prefix) = current_field_prefix_at_position(line, position.character as usize) {
            for field in typed_current_fields_at_line(&text, &uri, position.line as usize) {
                if !field.starts_with(field_prefix) {
                    continue;
                }

                items.push(CompletionItem {
                    label: field.clone(),
                    kind: Some(CompletionItemKind::FIELD),
                    detail: Some("field on $current".to_string()),
                    insert_text: Some(field),
                    ..CompletionItem::default()
                });
            }
        }

        if let Some((namespace, type_prefix)) = namespace_type_prefix_at_position(line, position.character as usize) {
            for type_name in resolve_imported_type_names(&text, &uri, namespace) {
                if !type_name.starts_with(type_prefix) {
                    continue;
                }

                items.push(CompletionItem {
                    label: type_name.clone(),
                    kind: Some(CompletionItemKind::CLASS),
                    detail: Some(format!("type from {}", namespace)),
                    insert_text: Some(type_name),
                    ..CompletionItem::default()
                });
            }
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
                    DefinitionKind::Glyph => "glyph".to_string(),
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

        let Some((module, op, active_parameter)) = active_transform_call_at(line, position.character as usize) else {
            if let Some((target, active_parameter)) = active_user_call_at(line, position.character as usize) {
                let signatures = parse_executable_signatures(&text);
                if let Some(sig) = signatures.get(&target) {
                    let output = sig
                        .output_type
                        .as_deref()
                        .unwrap_or(&sig.input_type);

                    let signature = SignatureInformation {
                        label: format!("{}(input: {}) -> {}", sig.name, sig.input_type, output),
                        documentation: Some(Documentation::MarkupContent(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: "User-defined executable signature".to_string(),
                        })),
                        parameters: Some(vec![ParameterInformation {
                            label: ParameterLabel::Simple(format!("input: {}", sig.input_type)),
                            documentation: None,
                        }]),
                        active_parameter: Some(0),
                    };

                    return Ok(Some(SignatureHelp {
                        signatures: vec![signature],
                        active_signature: Some(0),
                        active_parameter: Some(active_parameter),
                    }));
                }
            }

            return Ok(None);
        };

        let Some(spec) = find_op_spec(&module, &op) else {
            return Ok(None);
        };

        let signature = SignatureInformation {
            label: format!("{}.{}({}) -> {}", spec.module, spec.op, signature_args_label(spec), signature_return_shape(spec)),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("{}\n\nEffect: `{}`", signature_summary(spec), signature_effect_label(spec)),
            })),
            parameters: Some(
                spec.args
                    .iter()
                    .map(|arg| ParameterInformation {
                        label: ParameterLabel::Simple(format!(
                            "{}: {}{}",
                            arg.name,
                            signature_arg_type_label(arg.ty),
                            if arg.required { "" } else { "?" }
                        )),
                        documentation: None,
                    })
                    .collect(),
            ),
            active_parameter: Some(0),
        };

        Ok(Some(SignatureHelp {
            signatures: vec![signature],
            active_signature: Some(0),
            active_parameter: Some(active_parameter),
        }))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let line_number = params.range.start.line as usize;

        let text = {
            let docs = self.documents.lock().await;
            match docs.get(&uri) {
                Some(content) => content.clone(),
                None => return Ok(None),
            }
        };

        let existing_defs = index_definitions(&text)
            .into_iter()
            .map(|d| d.name)
            .collect::<std::collections::HashSet<_>>();

        let missing_targets = missing_if_branch_targets_at_line(&text, line_number, &existing_defs);
        if missing_targets.is_empty() {
            return Ok(None);
        }

        let (input_type, output_type) = enclosing_executable_signature(&text, line_number)
            .unwrap_or(("Any".to_string(), None));

        let insert_at = full_document_range(&text).end;
        let mut actions = Vec::new();

        for target in missing_targets {
            let new_text = build_iterator_skeleton(&target, &input_type, output_type.as_deref());

            let mut changes = HashMap::new();
            changes.insert(
                uri.clone(),
                vec![TextEdit {
                    range: Range {
                        start: insert_at,
                        end: insert_at,
                    },
                    new_text,
                }],
            );

            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                title: format!("Generate iterator '{}'", target),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: None,
                edit: Some(WorkspaceEdit {
                    changes: Some(changes),
                    document_changes: None,
                    change_annotations: None,
                }),
                command: None,
                is_preferred: Some(false),
                disabled: None,
                data: None,
            }));
        }

        Ok(Some(actions))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        let text = {
            let docs = self.documents.lock().await;
            match docs.get(&uri) {
                Some(content) => content.clone(),
                None => return Ok(None),
            }
        };

        let data = build_semantic_tokens(&text);
        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data,
        })))
    }
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
            label: "glyph".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            insert_text: Some("glyph ${1:Main} {\n  ${2}\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            documentation: Some(Documentation::String(
                "Create an explicit composition entrypoint root.".to_string(),
            )),
            ..CompletionItem::default()
        },
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

        if let Some((name, start_char, end_char)) = parse_definition_name(trimmed, "glyph", line) {
            out.push(DefinitionIndex {
                name,
                kind: DefinitionKind::Glyph,
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

fn active_transform_call_at(line: &str, cursor: usize) -> Option<(String, String, u32)> {
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
    let _ = find_op_spec(module, op)?;

    let args_so_far = &before[open_idx + 1..];
    let active_parameter = args_so_far.chars().filter(|c| *c == ',').count() as u32;
    Some((module.to_string(), op.to_string(), active_parameter))
}

fn completion_insert_text(spec: &OpSpec, op_only: bool) -> String {
    let mut parts = Vec::new();
    for (idx, arg) in spec.args.iter().enumerate() {
        let value = match arg.ty {
            ArgType::String => format!("\"${{{}:}}\"", idx + 1),
            ArgType::Number => format!("${{{}:0}}", idx + 1),
            ArgType::Boolean => format!("${{{}:false}}", idx + 1),
            ArgType::Object => format!("${{{}:{{}}}}", idx + 1),
            ArgType::Array => format!("${{{}:[]}}", idx + 1),
            ArgType::Any => format!("${{{}:null}}", idx + 1),
        };
        parts.push(format!("{}: {}", arg.name, value));
    }

    let args = parts.join(", ");
    if op_only {
        format!("{}({})", spec.op, args)
    } else {
        format!("{}.{}({})", spec.module, spec.op, args)
    }
}

fn signature_summary(spec: &OpSpec) -> &'static str {
    match (spec.module, spec.op) {
        ("html", "to_md") => "Converts HTML input to Markdown text.",
        ("json", "parse") => "Parses JSON text into a structured JSON value.",
        ("csv", "to_list") => "Parses CSV text with headers into row objects.",
        ("yaml", "to_json") => "Parses YAML text and converts it to JSON.",
        _ => "Operation from shared signature registry.",
    }
}

fn signature_return_shape(spec: &OpSpec) -> String {
    if let Some(shape) = spec.output_schema_ref {
        return shape.to_string();
    }

    match (spec.module, spec.op) {
        ("html", "to_md") => "{ text: string, markdown: string, ... }".to_string(),
        ("json", "parse") => "JsonValue".to_string(),
        ("csv", "to_list") => "Array<Object<string, string>>".to_string(),
        ("yaml", "to_json") => "JsonValue".to_string(),
        _ => "JsonValue".to_string(),
    }
}

fn signature_args_label(spec: &OpSpec) -> String {
    if spec.args.is_empty() {
        return "".to_string();
    }

    spec.args
        .iter()
        .map(|arg| {
            format!(
                "{}: {}{}",
                arg.name,
                signature_arg_type_label(arg.ty),
                if arg.required { "" } else { "?" }
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn signature_arg_type_label(ty: ArgType) -> &'static str {
    match ty {
        ArgType::String => "string",
        ArgType::Number => "number",
        ArgType::Boolean => "boolean",
        ArgType::Object => "object",
        ArgType::Array => "array",
        ArgType::Any => "any",
    }
}

fn signature_effect_label(spec: &OpSpec) -> &'static str {
    match spec.effect {
        grapheme_signatures::SignatureEffect::Pure => "pure",
        grapheme_signatures::SignatureEffect::Network => "network",
        grapheme_signatures::SignatureEffect::Io => "io",
        grapheme_signatures::SignatureEffect::State => "state",
        grapheme_signatures::SignatureEffect::Secrets => "secrets",
        grapheme_signatures::SignatureEffect::Control => "control",
    }
}

fn text_line(text: &str, line_number: usize) -> Option<&str> {
    text.lines().nth(line_number)
}

fn line_len(line: &str) -> u32 {
    line.chars().count() as u32
}

fn current_field_prefix_at_position(line: &str, character: usize) -> Option<&str> {
    if line.is_empty() {
        return None;
    }

    let bytes = line.as_bytes();
    let mut cursor = character.min(bytes.len());
    if cursor > 0 && cursor == bytes.len() {
        cursor -= 1;
    }

    let mut start = cursor;
    while start > 0 && is_ident_char(bytes[start.saturating_sub(1)]) {
        start -= 1;
    }

    let mut end = cursor;
    while end < bytes.len() && is_ident_char(bytes[end]) {
        end += 1;
    }

    let prefix = &line[start..end];
    let before = &line[..start];
    if !before.ends_with("$current.") {
        return None;
    }

    Some(prefix)
}

fn namespace_type_prefix_at_position<'a>(line: &'a str, character: usize) -> Option<(&'a str, &'a str)> {
    if line.is_empty() {
        return None;
    }

    let bytes = line.as_bytes();
    let mut cursor = character.min(bytes.len());
    if cursor == bytes.len() && cursor > 0 {
        cursor -= 1;
    }

    let mut start = cursor;
    while start > 0 && (is_ident_char(bytes[start - 1]) || bytes[start - 1] == b':') {
        start -= 1;
    }

    let mut end = cursor;
    while end < bytes.len() && (is_ident_char(bytes[end]) || bytes[end] == b':') {
        end += 1;
    }

    let token = &line[start..end];
    let (namespace, type_prefix) = token.split_once("::")?;
    if !is_ident(namespace) {
        return None;
    }
    if !type_prefix.is_empty() && !is_ident(type_prefix) {
        return None;
    }
    Some((namespace, type_prefix))
}

fn namespaced_type_at_position(line: &str, character: usize) -> Option<(String, String)> {
    let (namespace, type_name) = namespace_type_prefix_at_position(line, character)?;
    if type_name.is_empty() {
        return None;
    }
    Some((namespace.to_string(), type_name.to_string()))
}

fn resolve_imported_type_names(text: &str, uri: &Url, namespace: &str) -> Vec<String> {
    let imports = parse_type_imports(text);
    let Some(path) = imports.get(namespace) else {
        return Vec::new();
    };

    let Some(imported_text) = read_imported_file_text(uri, path) else {
        return Vec::new();
    };

    parse_struct_names(&imported_text)
}

fn resolve_imported_type_fields(text: &str, uri: &Url, namespace: &str, type_name: &str) -> Vec<String> {
    let imports = parse_type_imports(text);
    let Some(path) = imports.get(namespace) else {
        return Vec::new();
    };

    let Some(imported_text) = read_imported_file_text(uri, path) else {
        return Vec::new();
    };

    parse_struct_fields(&imported_text)
        .remove(type_name)
        .unwrap_or_default()
}

fn parse_type_imports(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for line in text.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("import types ") else {
            continue;
        };

        let Some((alias_part, path_part)) = rest.split_once(" from ") else {
            continue;
        };

        let alias = alias_part.trim();
        let path = path_part.trim().trim_matches('"');
        if alias.is_empty() || path.is_empty() || !is_ident(alias) {
            continue;
        }

        out.insert(alias.to_string(), path.to_string());
    }
    out
}

fn read_imported_file_text(current_uri: &Url, import_path: &str) -> Option<String> {
    let current_file = current_uri.to_file_path().ok()?;
    let parent = current_file.parent()?;
    let candidate = parent.join(import_path);
    fs::read_to_string(candidate).ok()
}

fn parse_struct_names(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let rest = trimmed.strip_prefix("struct ")?;
            let name = rest
                .split(|c: char| c.is_ascii_whitespace() || c == '{')
                .next()
                .unwrap_or("")
                .trim();
            if is_ident(name) {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn is_current_context(line: &str, character: usize) -> bool {
    let clamped = character.min(line.len());
    line[..clamped].contains("$current")
}

fn typed_current_fields_at_line(text: &str, uri: &Url, line_number: usize) -> Vec<String> {
    let Some((type_name, _)) = enclosing_executable_signature(text, line_number) else {
        return Vec::new();
    };

    resolve_fields_for_type_ref(text, uri, &type_name)
}

fn resolve_fields_for_type_ref(text: &str, uri: &Url, type_ref: &str) -> Vec<String> {
    if let Some((namespace, type_name)) = type_ref.split_once("::") {
        return resolve_imported_type_fields(text, uri, namespace, type_name);
    }

    let structs = parse_struct_fields(text);
    structs.get(type_ref).cloned().unwrap_or_default()
}

fn parse_struct_fields(text: &str) -> HashMap<String, Vec<String>> {
    let mut out = HashMap::new();
    let mut current_struct: Option<String> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if current_struct.is_none() {
            if let Some(rest) = trimmed.strip_prefix("struct ") {
                let name = rest
                    .split(|c: char| c.is_ascii_whitespace() || c == '{')
                    .next()
                    .unwrap_or("")
                    .trim();
                if !name.is_empty() {
                    current_struct = Some(name.to_string());
                    out.entry(name.to_string()).or_insert_with(Vec::new);
                }
            }
            continue;
        }

        if trimmed.starts_with('}') {
            current_struct = None;
            continue;
        }

        let Some(struct_name) = current_struct.as_ref() else {
            continue;
        };

        if let Some((left, _)) = trimmed.split_once(':') {
            let field = left.trim().trim_end_matches('?').trim();
            if is_ident(field) {
                out.entry(struct_name.clone())
                    .or_insert_with(Vec::new)
                    .push(field.to_string());
            }
        }
    }

    out
}

fn enclosing_executable_signature(text: &str, line_number: usize) -> Option<(String, Option<String>)> {
    let mut current_signature: Option<String> = None;
    let mut current_output: Option<String> = None;
    let mut brace_depth: i32 = 0;

    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if brace_depth == 0 {
            let sig = parse_executable_input_and_output_type(trimmed);
            current_signature = sig.as_ref().map(|(input, _)| input.clone());
            current_output = sig.and_then(|(_, output)| output);
        }

        let opens = line.chars().filter(|c| *c == '{').count() as i32;
        let closes = line.chars().filter(|c| *c == '}').count() as i32;
        brace_depth += opens - closes;
        if brace_depth < 0 {
            brace_depth = 0;
        }

        if idx == line_number {
            if let Some(input) = current_signature {
                return Some((input, current_output));
            }
            return None;
        }

        if brace_depth == 0 {
            current_signature = None;
            current_output = None;
        }
    }

    None
}

fn parse_executable_input_and_output_type(trimmed_line: &str) -> Option<(String, Option<String>)> {
    let starts_with_executable = trimmed_line.starts_with("query ")
        || trimmed_line.starts_with("mutation ")
        || trimmed_line.starts_with("iterator ")
        || trimmed_line.starts_with("subscription ");

    if !starts_with_executable {
        return None;
    }

    let on_index = trimmed_line.find(" on ")?;
    let after_on = &trimmed_line[on_index + 4..];
    let input = after_on
        .split(|c: char| c.is_ascii_whitespace() || c == '-' || c == '{' || c == '@')
        .next()
        .unwrap_or("")
        .trim();

    if input.is_empty() || !is_type_ref(input) {
        return None;
    }

    let output = if let Some((_, right)) = after_on.split_once("->") {
        let output = right
            .split(|c: char| c.is_ascii_whitespace() || c == '{' || c == '@')
            .next()
            .unwrap_or("")
            .trim();
        if output.is_empty() || !is_type_ref(output) {
            None
        } else {
            Some(output.to_string())
        }
    } else {
        None
    };

    Some((input.to_string(), output))
}

fn parse_executable_signatures(text: &str) -> HashMap<String, ExecutableSignatureIndex> {
    let mut out = HashMap::new();

    for line in text.lines() {
        let trimmed = line.trim_start();
        let Some((_, rest)) = executable_head(trimmed) else {
            continue;
        };

        let name = rest
            .split(|c: char| c.is_ascii_whitespace() || c == '(' || c == '{')
            .next()
            .unwrap_or("")
            .trim();
        if !is_ident(name) {
            continue;
        }

        let Some((input_type, output_type)) = parse_executable_input_and_output_type(trimmed) else {
            continue;
        };

        out.insert(
            name.to_string(),
            ExecutableSignatureIndex {
                name: name.to_string(),
                input_type,
                output_type,
            },
        );
    }

    out
}

fn executable_head(trimmed_line: &str) -> Option<(&'static str, &str)> {
    if let Some(rest) = trimmed_line.strip_prefix("query ") {
        return Some(("query", rest));
    }
    if let Some(rest) = trimmed_line.strip_prefix("mutation ") {
        return Some(("mutation", rest));
    }
    if let Some(rest) = trimmed_line.strip_prefix("iterator ") {
        return Some(("iterator", rest));
    }
    if let Some(rest) = trimmed_line.strip_prefix("subscription ") {
        return Some(("subscription", rest));
    }
    None
}

fn is_type_ref(value: &str) -> bool {
    if value.contains("::") {
        let mut parts = value.split("::");
        let Some(ns) = parts.next() else {
            return false;
        };
        let Some(name) = parts.next() else {
            return false;
        };
        parts.next().is_none() && is_ident(ns) && is_ident(name)
    } else {
        is_ident(value)
    }
}

const TOKEN_KEYWORD: u32 = 0;
const TOKEN_FUNCTION: u32 = 1;
const TOKEN_TYPE: u32 = 2;
const TOKEN_NAMESPACE: u32 = 3;
const TOKEN_VARIABLE: u32 = 4;
const TOKEN_STRING: u32 = 5;
const TOKEN_NUMBER: u32 = 6;
const TOKEN_OPERATOR: u32 = 7;

fn semantic_tokens_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: vec![
            SemanticTokenType::KEYWORD,
            SemanticTokenType::FUNCTION,
            SemanticTokenType::TYPE,
            SemanticTokenType::NAMESPACE,
            SemanticTokenType::VARIABLE,
            SemanticTokenType::STRING,
            SemanticTokenType::NUMBER,
            SemanticTokenType::OPERATOR,
        ],
        token_modifiers: vec![],
    }
}

struct SemanticTokenBuilder {
    data: Vec<SemanticToken>,
    prev_line: u32,
    prev_start: u32,
    has_prev: bool,
}

impl SemanticTokenBuilder {
    fn new() -> Self {
        Self {
            data: Vec::new(),
            prev_line: 0,
            prev_start: 0,
            has_prev: false,
        }
    }

    fn push(&mut self, line: u32, start: u32, length: u32, token_type: u32) {
        if length == 0 {
            return;
        }

        let (delta_line, delta_start) = if !self.has_prev {
            (line, start)
        } else if line == self.prev_line {
            (0, start.saturating_sub(self.prev_start))
        } else {
            (line.saturating_sub(self.prev_line), start)
        };

        self.data.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type,
            token_modifiers_bitset: 0,
        });

        self.prev_line = line;
        self.prev_start = start;
        self.has_prev = true;
    }

    fn finish(self) -> Vec<SemanticToken> {
        self.data
    }
}

fn build_semantic_tokens(text: &str) -> Vec<SemanticToken> {
    let mut builder = SemanticTokenBuilder::new();

    for (line_idx, line) in text.lines().enumerate() {
        add_line_semantic_tokens(line, line_idx as u32, &mut builder);
    }

    builder.finish()
}

fn add_line_semantic_tokens(line: &str, line_num: u32, out: &mut SemanticTokenBuilder) {
    let line_no_comment = line.split("//").next().unwrap_or("");
    if line_no_comment.trim().is_empty() {
        return;
    }

    add_declaration_tokens(line_no_comment, line_num, out);
    add_module_call_tokens(line_no_comment, line_num, out);
    add_namespaced_type_tokens(line_no_comment, line_num, out);

    let bytes = line_no_comment.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                let start = i;
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' {
                        i = (i + 2).min(bytes.len());
                        continue;
                    }
                    if bytes[i] == b'"' {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                out.push(
                    line_num,
                    start as u32,
                    (i.saturating_sub(start)) as u32,
                    TOKEN_STRING,
                );
            }
            b'$' => {
                let start = i;
                i += 1;
                while i < bytes.len() && (is_ident_char(bytes[i]) || bytes[i] == b'.') {
                    i += 1;
                }
                out.push(
                    line_num,
                    start as u32,
                    (i.saturating_sub(start)) as u32,
                    TOKEN_VARIABLE,
                );
            }
            b'|' if i + 1 < bytes.len() && bytes[i + 1] == b'>' => {
                out.push(line_num, i as u32, 2, TOKEN_OPERATOR);
                i += 2;
            }
            b'-' if i + 1 < bytes.len() && bytes[i + 1] == b'>' => {
                out.push(line_num, i as u32, 2, TOKEN_OPERATOR);
                i += 2;
            }
            b':' => {
                out.push(line_num, i as u32, 1, TOKEN_OPERATOR);
                i += 1;
            }
            b'-' | b'0'..=b'9' => {
                if let Some((len, starts_number)) = number_span(&bytes[i..]) {
                    if starts_number {
                        out.push(line_num, i as u32, len as u32, TOKEN_NUMBER);
                        i += len;
                    } else {
                        i += 1;
                    }
                } else {
                    i += 1;
                }
            }
            _ if is_ident_char(bytes[i]) => {
                let start = i;
                i += 1;
                while i < bytes.len() && is_ident_char(bytes[i]) {
                    i += 1;
                }
                let ident = &line_no_comment[start..i];
                if is_keyword(ident) {
                    out.push(
                        line_num,
                        start as u32,
                        (i - start) as u32,
                        TOKEN_KEYWORD,
                    );
                } else if is_builtin_type(ident) {
                    out.push(line_num, start as u32, (i - start) as u32, TOKEN_TYPE);
                }
            }
            _ => {
                i += 1;
            }
        }
    }
}

fn add_declaration_tokens(line: &str, line_num: u32, out: &mut SemanticTokenBuilder) {
    for head in ["glyph", "query", "mutation", "iterator", "subscription"] {
        if let Some((name_start, name_len)) = declaration_name_span(line, head) {
            let kw_pos = line.find(head).unwrap_or(0) as u32;
            out.push(line_num, kw_pos, head.len() as u32, TOKEN_KEYWORD);
            out.push(line_num, name_start as u32, name_len as u32, TOKEN_FUNCTION);
            return;
        }
    }

    if let Some((name_start, name_len)) = declaration_name_span(line, "struct") {
        let kw_pos = line.find("struct").unwrap_or(0) as u32;
        out.push(line_num, kw_pos, 6, TOKEN_KEYWORD);
        out.push(line_num, name_start as u32, name_len as u32, TOKEN_TYPE);
    }
}

fn declaration_name_span(line: &str, head: &str) -> Option<(usize, usize)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with(head) {
        return None;
    }
    let offset = line.len().saturating_sub(trimmed.len());
    let mut i = head.len();
    let bytes = trimmed.as_bytes();
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    let start = i;
    while i < bytes.len() && is_ident_char(bytes[i]) {
        i += 1;
    }
    if i == start {
        return None;
    }
    Some((offset + start, i - start))
}

fn add_module_call_tokens(line: &str, line_num: u32, out: &mut SemanticTokenBuilder) {
    let bytes = line.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if !is_ident_char(bytes[i]) {
            i += 1;
            continue;
        }

        let ns_start = i;
        while i < bytes.len() && is_ident_char(bytes[i]) {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'.' {
            continue;
        }

        let ns_len = i - ns_start;
        i += 1;
        let fn_start = i;
        while i < bytes.len() && is_ident_char(bytes[i]) {
            i += 1;
        }
        if fn_start == i {
            continue;
        }
        let fn_len = i - fn_start;

        let mut j = i;
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        if j < bytes.len() && bytes[j] == b'(' {
            out.push(line_num, ns_start as u32, ns_len as u32, TOKEN_NAMESPACE);
            out.push(line_num, fn_start as u32, fn_len as u32, TOKEN_FUNCTION);
        }
    }
}

fn add_namespaced_type_tokens(line: &str, line_num: u32, out: &mut SemanticTokenBuilder) {
    let bytes = line.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if !is_ident_char(bytes[i]) {
            i += 1;
            continue;
        }

        let ns_start = i;
        while i < bytes.len() && is_ident_char(bytes[i]) {
            i += 1;
        }
        if i + 1 >= bytes.len() || bytes[i] != b':' || bytes[i + 1] != b':' {
            continue;
        }

        let ns_len = i - ns_start;
        i += 2;
        let ty_start = i;
        while i < bytes.len() && is_ident_char(bytes[i]) {
            i += 1;
        }
        if ty_start == i {
            continue;
        }
        let ty_len = i - ty_start;
        out.push(line_num, ns_start as u32, ns_len as u32, TOKEN_NAMESPACE);
        out.push(line_num, ty_start as u32, ty_len as u32, TOKEN_TYPE);
    }
}

fn number_span(slice: &[u8]) -> Option<(usize, bool)> {
    if slice.is_empty() {
        return None;
    }

    let mut i = 0usize;
    if slice[i] == b'-' {
        i += 1;
    }

    let int_start = i;
    while i < slice.len() && slice[i].is_ascii_digit() {
        i += 1;
    }

    if i == int_start {
        return Some((1, false));
    }

    if i < slice.len() && slice[i] == b'.' {
        i += 1;
        let frac_start = i;
        while i < slice.len() && slice[i].is_ascii_digit() {
            i += 1;
        }
        if i == frac_start {
            return Some((frac_start, true));
        }
    }

    Some((i, true))
}

fn is_keyword(value: &str) -> bool {
    matches!(
        value,
        "import"
            | "from"
            | "glyph"
            | "query"
            | "mutation"
            | "iterator"
            | "subscription"
            | "fragment"
            | "on"
            | "schema"
            | "type"
            | "types"
            | "struct"
            | "module"
            | "propose"
            | "return"
            | "then"
            | "else"
            | "when"
            | "true"
            | "false"
            | "null"
    )
}

fn is_builtin_type(value: &str) -> bool {
    matches!(
        value,
        "String" | "Float" | "Int" | "Bool" | "Object" | "Any" | "Json" | "JsonValue"
    )
}

fn active_user_call_at(line: &str, cursor: usize) -> Option<(String, u32)> {
    if line.is_empty() {
        return None;
    }

    let clamped = cursor.min(line.len());
    let before = &line[..clamped];
    let open_idx = before.rfind('(')?;

    let call_head = before[..open_idx].trim_end();
    let mut end = call_head.len();
    while end > 0 && !is_ident_char(call_head.as_bytes()[end - 1]) {
        end -= 1;
    }
    if end == 0 {
        return None;
    }

    let mut start = end;
    while start > 0 && is_ident_char(call_head.as_bytes()[start - 1]) {
        start -= 1;
    }

    let token = &call_head[start..end];
    if token.is_empty() || !is_ident(token) {
        return None;
    }

    let args_so_far = &before[open_idx + 1..];
    let active_parameter = args_so_far.chars().filter(|c| *c == ',').count() as u32;
    Some((token.to_string(), active_parameter))
}

fn missing_if_branch_targets_at_line(
    text: &str,
    line_number: usize,
    existing_defs: &std::collections::HashSet<String>,
) -> Vec<String> {
    let Some(line) = line_at(text, line_number) else {
        return Vec::new();
    };

    let Some((then_target, else_target)) = parse_if_step_targets(line) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for target in [then_target, else_target] {
        if is_branch_return_target(&target) || existing_defs.contains(&target) {
            continue;
        }

        if !out.contains(&target) {
            out.push(target);
        }
    }

    out
}

fn parse_if_step_targets(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if !trimmed.starts_with("if ") {
        return None;
    }

    let then_idx = trimmed.find(" then ")?;
    let else_idx = trimmed[then_idx + 6..].find(" else ")? + then_idx + 6;

    let then_raw = &trimmed[then_idx + 6..else_idx];
    let else_raw = &trimmed[else_idx + 6..];

    let then_target = parse_branch_target_token(then_raw)?;
    let else_target = parse_branch_target_token(else_raw)?;
    Some((then_target, else_target))
}

fn parse_branch_target_token(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let token = trimmed
        .split(|c: char| c.is_ascii_whitespace() || c == ',' || c == ')' || c == '(')
        .next()
        .unwrap_or("")
        .trim();

    if token.is_empty() {
        return None;
    }

    if is_ident(token) || token == "return" {
        Some(token.to_string())
    } else {
        None
    }
}

fn is_branch_return_target(target: &str) -> bool {
    target == "return" || target == "$return"
}

fn build_iterator_skeleton(target: &str, input_type: &str, output_type: Option<&str>) -> String {
    let signature = match output_type {
        Some(out) => format!("on {} -> {}", input_type, out),
        None => format!("on {}", input_type),
    };

    format!(
        "\n\niterator {} {} {{\n  core.echo(message: \"todo:{}\")\n}}\n",
        target, signature, target
    )
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

        collect_gr_files(&path, &mut docs);
    }

    docs
}

fn collect_gr_files(dir: &Path, out: &mut HashMap<Url, String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if should_skip_dir(&path) {
                continue;
            }
            collect_gr_files(&path, out);
            continue;
        }

        if !is_grapheme_source_file(&path) {
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

fn is_grapheme_source_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("gr"))
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
