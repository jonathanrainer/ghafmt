//! Inserts blank lines between jobs in the emitted event stream.
use fyaml::{EmitEvent, WriteType};

use crate::presentation_transformers::{
    PresentationTransformer, insert_blank_line_before_comment_block,
};

#[derive(Default)]
/// Inserts a blank line before every job (after the first) in the `jobs` mapping.
pub(crate) struct JobsBlankLines {}

#[derive(Default, Clone, Copy)]
/// Tracks where in the event stream the `jobs` block begins.
enum State {
    /// No `jobs` key has been seen yet.
    #[default]
    Init,
    /// Currently inside the `jobs` mapping.
    Jobs {
        /// Index into the event buffer from which blank-line insertion is searched backwards.
        start: usize,
        /// Whether the next job encountered is the first (no blank line inserted before it).
        is_first: bool,
    },
}

impl PresentationTransformer for JobsBlankLines {
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
                ((WriteType::PlainScalarKey, c), State::Init) if c == "jobs" => {
                    state = State::Jobs {
                        start: result.len(),
                        is_first: true,
                    }
                }
                ((WriteType::Indicator, c), State::Jobs { start, is_first })
                    if c == ":" && indent_level == 2 =>
                {
                    // len - 1 because the PlainScalarKey of the new job
                    // has already been pushed and must be excluded from the scan
                    if !is_first {
                        let len = result.len();
                        insert_blank_line_before_comment_block(
                            &mut result,
                            start,
                            len - 1,
                            Some(2),
                        );
                    }
                    state = State::Jobs {
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
        "Blank lines between jobs"
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
    #[case::no_jobs(
        Document::from_string(indoc! {"
            a: b
            b: c
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            a: b
            b: c
        "}.to_string()
    )]
    #[case::single_job(
        Document::from_string(indoc! {"
            jobs:
              build:
                runs-on: ubuntu-latest
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              build:
                runs-on: ubuntu-latest
        "}.to_string()
    )]
    #[case::two_jobs(
        Document::from_string(indoc! {"
            jobs:
              build:
                runs-on: ubuntu-latest
              test:
                runs-on: ubuntu-latest
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              build:
                runs-on: ubuntu-latest

              test:
                runs-on: ubuntu-latest
        "}.to_string()
    )]
    #[case::three_jobs(
        Document::from_string(indoc! {"
            jobs:
              build:
                runs-on: ubuntu-latest
              test:
                runs-on: ubuntu-latest
              deploy:
                runs-on: ubuntu-latest
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              build:
                runs-on: ubuntu-latest

              test:
                runs-on: ubuntu-latest

              deploy:
                runs-on: ubuntu-latest
        "}.to_string()
    )]
    #[case::comment_between_jobs(
        Document::from_string(indoc! {"
            jobs:
              build:
                runs-on: ubuntu-latest
              # Run tests
              test:
                runs-on: ubuntu-latest
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              build:
                runs-on: ubuntu-latest

              # Run tests
              test:
                runs-on: ubuntu-latest
        "}.to_string()
    )]
    #[case::multiline_comment_between_jobs(
        Document::from_string(indoc! {"
            jobs:
              build:
                runs-on: ubuntu-latest
              # Run tests
              # on all platforms
              test:
                runs-on: ubuntu-latest
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              build:
                runs-on: ubuntu-latest

              # Run tests
              # on all platforms
              test:
                runs-on: ubuntu-latest
        "}.to_string()
    )]
    #[case::comment_before_first_job(
        Document::from_string(indoc! {"
            jobs:
              # the first job
              build:
                runs-on: ubuntu-latest
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              # the first job
              build:
                runs-on: ubuntu-latest
        "}.to_string()
    )]
    #[case::comment_before_first_job_with_second(
        Document::from_string(indoc! {"
            jobs:
              # the first job
              build:
                runs-on: ubuntu-latest
              test:
                runs-on: ubuntu-latest
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              # the first job
              build:
                runs-on: ubuntu-latest

              test:
                runs-on: ubuntu-latest
        "}.to_string()
    )]
    fn test_jobs_emitter(#[case] source_doc: Document, #[case] expected: String) {
        let transformer = JobsBlankLines::default();
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
