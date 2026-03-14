//! Inserts blank lines between top-level workflow sections in the emitted event stream.
use fyaml::{EmitEvent, WriteType};

use crate::{
    constants::TOP_LEVEL_KEY_ORDERING,
    presentation_transformers::{PresentationTransformer, insert_blank_line_before_comment_block},
};

#[derive(Default)]
/// Inserts a blank line before every known top-level key (`on`, `jobs`, etc.) except the first.
pub(crate) struct TopLevelBlankLines {}

impl PresentationTransformer for TopLevelBlankLines {
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
                (WriteType::PlainScalarKey, c)
                    if TOP_LEVEL_KEY_ORDERING.contains(&c.as_str()) && indent_level == 0 =>
                {
                    if !result.is_empty() {
                        let len = result.len();
                        insert_blank_line_before_comment_block(&mut result, 0, len);
                    }
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
        "Blank lines between top-level elements ('jobs', 'on' etc.)"
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
    #[case::no_top_level_keys(
        Document::from_string(indoc! {"
            x: 1
            y: 2
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            x: 1
            y: 2
        "}.to_string()
    )]
    #[case::single_top_level_key(
        Document::from_string(indoc! {"
            name: my-workflow
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            name: my-workflow
        "}.to_string()
    )]
    #[case::two_top_level_keys(
        Document::from_string(indoc! {"
            name: my-workflow
            on: push
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            name: my-workflow

            on: push
        "}.to_string()
    )]
    #[case::three_top_level_keys(
        Document::from_string(indoc! {"
            name: my-workflow
            on: push
            jobs:
              build:
                runs-on: ubuntu-latest
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            name: my-workflow

            on: push

            jobs:
              build:
                runs-on: ubuntu-latest
        "}.to_string()
    )]
    #[case::comment_before_top_level_key(
        Document::from_string(indoc! {"
            name: my-workflow
            # trigger
            on: push
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            name: my-workflow

            # trigger
            on: push
        "}.to_string()
    )]
    #[case::multiline_comment_before_top_level_key(
        Document::from_string(indoc! {"
            name: my-workflow
            # line one
            # line two
            on: push
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            name: my-workflow

            # line one
            # line two
            on: push
        "}.to_string()
    )]
    #[case::non_top_level_keys_unaffected(
        Document::from_string(indoc! {"
            name: my-workflow
            on:
              push:
                branches:
                  - main
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            name: my-workflow

            on:
              push:
                branches:
                  - main
        "}.to_string()
    )]
    fn test_top_level_blank_lines(#[case] source_doc: Document, #[case] expected: String) {
        let transformer = TopLevelBlankLines::default();
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
