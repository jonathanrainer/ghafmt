//! Presentation transformers that post-process the emitted YAML event stream.
mod jobs_blank_lines;
mod steps_blank_lines;
mod top_level_blank_lines;
mod variable_spacer;

use fyaml::{EmitEvent, WriteType};
pub(crate) use jobs_blank_lines::JobsBlankLines;
pub(crate) use steps_blank_lines::StepsBlankLines;
pub(crate) use top_level_blank_lines::TopLevelBlankLines;
pub(crate) use variable_spacer::VariableSpacer;

/// This trait captures what it means for a transform operation to act on the presentation
/// of a YAML document. As such anything using this trait should not change the semantic meaning
/// of the YAML document, but only the presentation. This could be used for spacing, or similar.
pub(crate) trait PresentationTransformer {
    /// Perform the action on the event stream, producing a new (potentially altered) event
    /// stream.
    fn process(&self, event_stream: Vec<EmitEvent>) -> Vec<EmitEvent>;

    /// A short human-readable name for the transformer, used in log messages.
    fn description(&self) -> &'static str;
}

/// Scans backward through events to find the correct insertion point for a blank line, skipping
/// past whole-line comment blocks. Inserts a `Linebreak` event if there is content before the scan
/// range to separate from.
///
/// The algorithm detects whole-line comments by looking for the pattern:
///   Linebreak → (Indent)* → Comment
/// and pushes the insertion point before such blocks.
pub(crate) fn insert_blank_line_before_comment_block(
    events: &mut Vec<EmitEvent>,
    scan_start: usize,
    scan_end: usize,
) {
    #[derive(Clone, Copy)]
    enum State {
        Init,
        CommentDetected,
        WholeLineCommentDetected,
    }

    let mut state = State::Init;
    let mut insertion_point = scan_end;
    let mut found_content = false;

    for i in (scan_start..scan_end).rev() {
        match (events[i].write_type, state) {
            (WriteType::Indent, _) => {}
            (WriteType::Comment, State::Init | State::WholeLineCommentDetected) => {
                state = State::CommentDetected;
            }
            (WriteType::Linebreak, State::CommentDetected) => {
                state = State::WholeLineCommentDetected;
                insertion_point = i;
            }
            (WriteType::Linebreak, _) => {
                insertion_point = i;
            }
            _ => {
                found_content = true;
                break;
            }
        }
    }

    if !found_content {
        return;
    }

    // If there are already two consecutive Linebreaks at the insertion point, a blank line
    // already exists (e.g. the trailing newlines kept by a `|+` scalar). Inserting another
    // would produce a double blank line, breaking idempotency.
    if events
        .get(insertion_point)
        .is_some_and(|e| e.write_type == WriteType::Linebreak)
        && events
            .get(insertion_point + 1)
            .is_some_and(|e| e.write_type == WriteType::Linebreak)
    {
        return;
    }

    events.insert(
        insertion_point,
        EmitEvent {
            write_type: WriteType::Linebreak,
            content: "\n".to_string(),
        },
    );
}

#[cfg(test)]
mod tests {
    use WriteType::*;
    use rstest::rstest;
    use similar_asserts::assert_eq;

    use super::*;

    fn ev(write_type: WriteType, content: &str) -> EmitEvent {
        EmitEvent {
            write_type,
            content: content.to_string(),
        }
    }

    #[rstest]
    #[case::basic_insertion(
        vec![ev(PlainScalar, "v1"), ev(Linebreak, "\n"), ev(PlainScalar, "v2")],
        0, 2,
        vec![ev(PlainScalar, "v1"), ev(Linebreak, "\n"), ev(Linebreak, "\n"), ev(PlainScalar, "v2")],
    )]
    #[case::single_whole_line_comment(
        vec![ev(PlainScalar, "v1"), ev(Linebreak, "\n"), ev(Comment, "# hello"), ev(Linebreak, "\n"), ev(PlainScalar, "v2")],
        0, 4,
        vec![ev(PlainScalar, "v1"), ev(Linebreak, "\n"), ev(Linebreak, "\n"), ev(Comment, "# hello"), ev(Linebreak, "\n"), ev(PlainScalar, "v2")],
    )]
    #[case::multi_line_comment_block(
        vec![ev(PlainScalar, "v1"), ev(Linebreak, "\n"), ev(Comment, "# one"), ev(Linebreak, "\n"), ev(Comment, "# two"), ev(Linebreak, "\n"), ev(PlainScalar, "v2")],
        0, 6,
        vec![ev(PlainScalar, "v1"), ev(Linebreak, "\n"), ev(Linebreak, "\n"), ev(Comment, "# one"), ev(Linebreak, "\n"), ev(Comment, "# two"), ev(Linebreak, "\n"), ev(PlainScalar, "v2")],
    )]
    #[case::inline_comment_not_pushed_back(
        vec![ev(PlainScalar, "v1"), ev(Comment, "# inline"), ev(Linebreak, "\n"), ev(PlainScalar, "v2")],
        0, 3,
        vec![ev(PlainScalar, "v1"), ev(Comment, "# inline"), ev(Linebreak, "\n"), ev(Linebreak, "\n"), ev(PlainScalar, "v2")],
    )]
    #[case::indent_events_skipped(
        vec![ev(PlainScalar, "v1"), ev(Linebreak, "\n"), ev(Indent, "  "), ev(Comment, "# hello"), ev(Linebreak, "\n"), ev(PlainScalar, "v2")],
        0, 5,
        vec![ev(PlainScalar, "v1"), ev(Linebreak, "\n"), ev(Linebreak, "\n"), ev(Indent, "  "), ev(Comment, "# hello"), ev(Linebreak, "\n"), ev(PlainScalar, "v2")],
    )]
    #[case::empty_scan_range(
        vec![ev(PlainScalar, "v1"), ev(PlainScalar, "v2")],
        1, 1,
        vec![ev(PlainScalar, "v1"), ev(PlainScalar, "v2")],
    )]
    #[case::only_comments_no_insert(
        vec![ev(Comment, "# top"), ev(Linebreak, "\n"), ev(PlainScalarKey, "name")],
        0, 2,
        vec![ev(Comment, "# top"), ev(Linebreak, "\n"), ev(PlainScalarKey, "name")],
    )]
    #[case::no_insert_when_blank_line_already_exists(
        // Two consecutive Linebreaks already form a blank line (e.g. from a |+ scalar).
        // A third must not be inserted.
        vec![ev(PlainScalar, "v1"), ev(Linebreak, "\n"), ev(Linebreak, "\n"), ev(PlainScalar, "v2")],
        0, 3,
        vec![ev(PlainScalar, "v1"), ev(Linebreak, "\n"), ev(Linebreak, "\n"), ev(PlainScalar, "v2")],
    )]
    #[case::no_insert_when_blank_line_exists_before_comment(
        // Blank line already exists before a standalone comment block.
        vec![ev(PlainScalar, "v1"), ev(Linebreak, "\n"), ev(Linebreak, "\n"), ev(Comment, "# c"), ev(Linebreak, "\n"), ev(PlainScalar, "v2")],
        0, 5,
        vec![ev(PlainScalar, "v1"), ev(Linebreak, "\n"), ev(Linebreak, "\n"), ev(Comment, "# c"), ev(Linebreak, "\n"), ev(PlainScalar, "v2")],
    )]
    fn test_insert_blank_line(
        #[case] mut input: Vec<EmitEvent>,
        #[case] scan_start: usize,
        #[case] scan_end: usize,
        #[case] expected: Vec<EmitEvent>,
    ) {
        insert_blank_line_before_comment_block(&mut input, scan_start, scan_end);
        assert_eq!(input, expected);
    }
}
