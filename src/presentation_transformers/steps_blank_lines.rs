//! Inserts blank lines between steps in the emitted event stream.
use fyaml::{EmitEvent, WriteType};

use crate::presentation_transformers::{
    PresentationTransformer, insert_blank_line_before_comment_block,
};

#[derive(Default)]
/// Inserts a blank line before every step (after the first) in each `steps` sequence.
pub(crate) struct StepsBlankLines {}

#[derive(Default, Clone, Copy)]
/// Tracks where in the event stream the current `steps` sequence begins.
enum State {
    /// No `steps` key has been seen yet.
    #[default]
    Init,
    /// Seen the `steps` key; waiting for the `:` indicator to confirm entry.
    Steps,
    /// Currently inside a `steps` sequence.
    Step {
        /// Index into the event buffer from which blank-line insertion is searched backwards.
        start: usize,
        /// Whether the next step encountered is the first (no blank line inserted before it).
        is_first: bool,
    },
}

impl PresentationTransformer for StepsBlankLines {
    fn process(&self, event_stream: Vec<EmitEvent>) -> Vec<EmitEvent> {
        let mut result: Vec<EmitEvent> = vec![];
        let mut state = State::Init;
        let mut indent_level = 0;

        for EmitEvent {
            write_type,
            content,
        } in event_stream
        {
            match ((write_type, content.clone()), state) {
                ((WriteType::Indent, c), _) => indent_level += c.len(),
                ((WriteType::Linebreak, _), _) => indent_level = 0,
                ((WriteType::PlainScalarKey, c), _) if c == "steps" && indent_level == 4 => {
                    state = State::Steps;
                }
                // Make sure we move to this state when we detect the ":" indicator
                // so that the "start" of us tracking the step is correct and not off by 1
                ((WriteType::Indicator, c), State::Steps) if c == ":" => {
                    state = State::Step {
                        start: result.len(),
                        is_first: true,
                    };
                }
                // Detect every list element that is at indent level 6 (each step) and
                // only apply the blank line spacing if it's not the first one
                ((WriteType::Indicator, c), State::Step { start, is_first })
                    if c == "-" && indent_level == 6 =>
                {
                    if !is_first {
                        let len = result.len();
                        insert_blank_line_before_comment_block(&mut result, start, len);
                    }
                    state = State::Step {
                        start,
                        is_first: false,
                    };
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
        "Blank lines between steps"
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
    #[case::no_steps(
        Document::from_string(indoc! {"
            a: b
            b: c
            c: d
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            a: b
            b: c
            c: d
        "}.to_string()
    )]
    #[case::steps_present(
        Document::from_string(indoc! {"
            jobs:
              foo:
                steps:
                  - id: a
                  - id: b
                  - id: c
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              foo:
                steps:
                  - id: a

                  - id: b

                  - id: c
        "}.to_string()
    )]
    #[case::single_comment_between_steps(
        Document::from_string(indoc! {"
            jobs:
              foo:
                steps:
                  - id: a
                  # between
                  - id: b
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              foo:
                steps:
                  - id: a

                  # between
                  - id: b
        "}.to_string()
    )]
    #[case::multiline_comment_between_steps(
        Document::from_string(indoc! {"
            jobs:
              foo:
                steps:
                  - id: a
                  # line one
                  # line two
                  - id: b
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              foo:
                steps:
                  - id: a

                  # line one
                  # line two
                  - id: b
        "}.to_string()
    )]
    #[case::comments_between_all_steps(
        Document::from_string(indoc! {"
            jobs:
              foo:
                steps:
                  - id: a
                  # comment b
                  - id: b
                  # comment c
                  - id: c
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              foo:
                steps:
                  - id: a

                  # comment b
                  - id: b

                  # comment c
                  - id: c
        "}.to_string()
    )]
    #[case::multi_job_comments_between_steps(
        Document::from_string(indoc! {"
            jobs:
              foo:
                steps:
                  - id: a
                  # comment
                  - id: b
              bar:
                steps:
                  - id: c
                  # comment
                  - id: d
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              foo:
                steps:
                  - id: a

                  # comment
                  - id: b
              bar:
                steps:
                  - id: c

                  # comment
                  - id: d
        "}.to_string()
    )]
    #[case::comment_before_first_step(
        Document::from_string(indoc! {"
            jobs:
              foo:
                steps:
                  # before first
                  - id: a
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              foo:
                steps:
                  # before first
                  - id: a
        "}.to_string()
    )]
    #[case::literal_keep_block_no_double_blank(
        // A |+ scalar already provides a trailing blank line; StepsBlankLines must not
        // insert a second one (idempotency check).
        Document::from_string(indoc! {"
            jobs:
              foo:
                steps:
                  - name: step a
                    run: |+
                      echo hello

                  - name: step b
                    run: echo world
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              foo:
                steps:
                  - name: step a
                    run: |+
                      echo hello

                  - name: step b
                    run: echo world
        "}.to_string()
    )]
    #[case::comment_before_first_step_and_between_subsequent(
        Document::from_string(indoc! {"
            jobs:
              foo:
                steps:
                  # before first
                  - id: a
                  # between
                  - id: b
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              foo:
                steps:
                  # before first
                  - id: a

                  # between
                  - id: b
        "}.to_string()
    )]
    fn test_steps_emitter(#[case] source_doc: Document, #[case] expected: String) {
        let transformer = StepsBlankLines::default();
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
