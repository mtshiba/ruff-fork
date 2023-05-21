use ruff_text_size::{TextLen, TextRange};
use rustpython_parser::ast::{self, Cmpop, Expr};
use unicode_width::UnicodeWidthStr;

use ruff_python_ast::newlines::Line;
use ruff_python_ast::source_code::Generator;

use crate::settings::line_width::{LineWidth, TabSize};

pub(crate) fn is_ambiguous_name(name: &str) -> bool {
    name == "l" || name == "I" || name == "O"
}

pub(crate) fn compare(
    left: &Expr,
    ops: &[Cmpop],
    comparators: &[Expr],
    generator: Generator,
) -> String {
    let node = ast::ExprCompare {
        left: Box::new(left.clone()),
        ops: ops.to_vec(),
        comparators: comparators.to_vec(),
        range: TextRange::default(),
    };
    generator.expr(&node.into())
}

pub(super) fn is_overlong(
    line: &Line,
    limit: LineWidth,
    ignore_overlong_task_comments: bool,
    task_tags: &[String],
    tab_size: TabSize,
) -> Option<Overlong> {
    let mut start_offset = line.start();
    let mut width = LineWidth::new(tab_size);

    for c in line.chars() {
        if width < limit {
            start_offset += c.text_len();
        }
        width = width.add_char(c);
    }

    if width <= limit {
        return None;
    }

    let mut chunks = line.split_whitespace();
    let (Some(first_chunk), Some(second_chunk)) = (chunks.next(), chunks.next()) else {
        // Single word / no printable chars - no way to make the line shorter
        return None;
    };

    if first_chunk == "#" {
        if ignore_overlong_task_comments {
            let second = second_chunk.trim_end_matches(':');
            if task_tags.iter().any(|task_tag| task_tag == second) {
                return None;
            }
        }
    }

    // Do not enforce the line length for lines that end with a URL, as long as the URL
    // begins before the limit.
    let last_chunk = chunks.last().unwrap_or(second_chunk);
    if last_chunk.contains("://") {
        if width.width() - last_chunk.width() <= limit.width() {
            return None;
        }
    }

    Some(Overlong {
        range: TextRange::new(start_offset, line.end()),
        width: width.width(),
    })
}

pub(super) struct Overlong {
    range: TextRange,
    width: usize,
}

impl Overlong {
    pub(super) const fn range(&self) -> TextRange {
        self.range
    }

    pub(super) const fn width(&self) -> usize {
        self.width
    }
}
