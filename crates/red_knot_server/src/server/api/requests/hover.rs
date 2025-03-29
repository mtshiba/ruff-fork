use std::borrow::Cow;

use crate::document::{FileRangeExt, PositionExt, ToLink, ToRangeExt};
use crate::server::api::traits::{BackgroundDocumentRequestHandler, RequestHandler};
use crate::server::client::Notifier;
use crate::DocumentSnapshot;
use lsp_types::request::HoverRequest;
use lsp_types::{
    GotoDefinitionResponse, Hover, HoverContents, HoverParams, MarkupContent, MarkupKind, Url,
};
use red_knot_ide::{go_to_type_definition, hover};
use red_knot_project::ProjectDatabase;
use ruff_db::source::{line_index, source_text};
use ruff_text_size::Ranged;

pub(crate) struct HoverRequestHandler;

impl RequestHandler for HoverRequestHandler {
    type RequestType = HoverRequest;
}

impl BackgroundDocumentRequestHandler for HoverRequestHandler {
    fn document_url(params: &HoverParams) -> Cow<Url> {
        Cow::Borrowed(&params.text_document_position_params.text_document.uri)
    }

    fn run_with_snapshot(
        snapshot: DocumentSnapshot,
        db: ProjectDatabase,
        _notifier: Notifier,
        params: HoverParams,
    ) -> crate::server::Result<Option<lsp_types::Hover>> {
        let Some(file) = snapshot.file(&db) else {
            tracing::debug!("Failed to resolve file for {:?}", params);
            return Ok(None);
        };

        let source = source_text(&db, file);
        let line_index = line_index(&db, file);
        let offset = params.text_document_position_params.position.to_text_size(
            &source,
            &line_index,
            snapshot.encoding(),
        );

        let Some(range_info) = hover(&db, file, offset) else {
            return Ok(None);
        };

        // TODO: Respect the clients preferred content format
        Ok(Some(lsp_types::Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("```\n{ty}\n```", ty = range_info.info.to_string(&db)),
            }),
            range: Some(range_info.file_range.range().to_range(
                &source,
                &line_index,
                snapshot.encoding(),
            )),
        }))
    }
}
