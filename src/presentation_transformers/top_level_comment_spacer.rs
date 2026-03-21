//! Ensures top-level standalone comments are visually separated from surrounding content.
use fyaml::{EmitEvent, WriteType};

use crate::presentation_transformers::{
    PresentationTransformer, insert_blank_line_before_comment_block,
};

#[derive(Default)]
/// Inserts a blank line before any block of standalone top-level comments (col 0, no preceding
/// indent) that follows content at a deeper indentation level.
///
/// This covers comments that do not precede a known top-level key and would therefore be missed
/// by `TopLevelBlankLines` — most commonly end-of-file comments. The idempotency guard inside
/// `insert_blank_line_before_comment_block` prevents double-blank-lines when a following
/// top-level key triggers a second insertion via `TopLevelBlankLines`.
pub(crate) struct TopLevelCommentSpacer {}

impl PresentationTransformer for TopLevelCommentSpacer {
    fn process(&self, event_stream: Vec<EmitEvent>) -> Vec<EmitEvent> {
        let mut result: Vec<EmitEvent> = vec![];
        let mut indent_level = 0;

        for EmitEvent {
            write_type,
            content,
        } in event_stream
        {
            match (write_type, content.clone()) {
                (WriteType::Indent, c) => indent_level += c.len(),
                (WriteType::Linebreak, _) => indent_level = 0,
                (WriteType::Comment, _)
                    if indent_level == 0
                        && result
                            .last()
                            .is_some_and(|e| e.write_type == WriteType::Linebreak) =>
                {
                    let len = result.len();
                    insert_blank_line_before_comment_block(&mut result, 0, len, Some(0));
                }
                _ => {}
            }
            result.push(EmitEvent {
                write_type,
                content,
            });
        }

        result
    }

    fn description(&self) -> &'static str {
        "Blank lines before top-level standalone comments"
    }
}

#[cfg(test)]
mod tests {
    use fyaml::Document;
    use indoc::indoc;
    use rstest::rstest;
    use similar_asserts::assert_eq;

    use super::*;
    use crate::workflow_emitter::WorkflowEmitter;

    #[rstest]
    #[case::no_comments(
        Document::from_string(indoc! {"
            a: b
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            a: b
        "}.to_string()
    )]
    #[case::top_level_comment_after_nested_content(
        Document::from_string(indoc! {"
            jobs:
              build:
                runs-on: ubuntu-latest
            # end-of-file comment
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              build:
                runs-on: ubuntu-latest

            # end-of-file comment
        "}.to_string()
    )]
    #[case::no_blank_line_at_start(
        Document::from_string(indoc! {"
            # header comment
            name: my-workflow
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            # header comment
            name: my-workflow
        "}.to_string()
    )]
    #[case::idempotent_when_blank_line_already_present(
        Document::from_string(indoc! {"
            jobs:
              build:
                runs-on: ubuntu-latest

            # end-of-file comment
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              build:
                runs-on: ubuntu-latest

            # end-of-file comment
        "}.to_string()
    )]
    fn test_top_level_comment_spacer(#[case] source_doc: Document, #[case] expected: String) {
        let transformer = TopLevelCommentSpacer::default();
        let events = WorkflowEmitter::create_event_stream(&source_doc)
            .expect("could not create event stream");
        let result: String = transformer
            .process(events)
            .into_iter()
            .map(|a| a.content)
            .collect();

        assert_eq!(result, expected);
    }
}
