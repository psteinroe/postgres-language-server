use crate::session::Session;
use anyhow::Result;
use pgt_workspace::{WorkspaceError, workspace};
use tower_lsp::lsp_types::{self, CompletionItem, CompletionItemLabelDetails};

use super::helper;

#[tracing::instrument(level = "trace", skip_all)]
pub fn get_completions(
    session: &Session,
    params: lsp_types::CompletionParams,
) -> Result<lsp_types::CompletionResponse> {
    let url = params.text_document_position.text_document.uri;
    let path = session.file_path(&url)?;

    let completion_result =
        match session
            .workspace
            .get_completions(workspace::GetCompletionsParams {
                path,
                position: helper::get_cursor_position(
                    session,
                    &url,
                    params.text_document_position.position,
                )?,
            }) {
            Ok(result) => result,
            Err(e) => match e {
                WorkspaceError::DatabaseConnectionError(_) => {
                    return Ok(lsp_types::CompletionResponse::Array(vec![]));
                }
                _ => {
                    return Err(e.into());
                }
            },
        };

    let items: Vec<CompletionItem> = completion_result
        .into_iter()
        .map(|i| CompletionItem {
            label: i.label,
            label_details: Some(CompletionItemLabelDetails {
                description: Some(i.description),
                detail: None,
            }),
            preselect: Some(i.preselected),
            kind: Some(to_lsp_types_completion_item_kind(i.kind)),
            ..CompletionItem::default()
        })
        .collect();

    Ok(lsp_types::CompletionResponse::Array(items))
}

fn to_lsp_types_completion_item_kind(
    pg_comp_kind: pgt_completions::CompletionItemKind,
) -> lsp_types::CompletionItemKind {
    match pg_comp_kind {
        pgt_completions::CompletionItemKind::Function => lsp_types::CompletionItemKind::FUNCTION,
        pgt_completions::CompletionItemKind::Table => lsp_types::CompletionItemKind::CLASS,
        pgt_completions::CompletionItemKind::Column => lsp_types::CompletionItemKind::FIELD,
    }
}
