//! Grapheme language server binary entrypoint.

#[tokio::main]
async fn main() {
    grapheme_lsp::run_stdio().await;
}
