//! Converts a processed document into a formatted YAML string via presentation transformers.
use fyaml::{Document, EmitEvent, EmitMode::Original};
use tracing::{info, trace};

use crate::{
    errors::{Error, Result},
    presentation_transformers::PresentationTransformer,
};

/// Converts a processed [`Document`] into a formatted YAML string via the presentation pipeline.
pub(crate) struct PresentationPipeline {
    /// Ordered list of presentation transformers to apply in sequence.
    transformers: Vec<Box<dyn PresentationTransformer>>,
}

impl PresentationPipeline {
    /// Create a [`PresentationPipeline`] with the given transformer pipeline.
    pub(crate) fn new(transformers: Vec<Box<dyn PresentationTransformer>>) -> Self {
        Self { transformers }
    }
    /// Produce the raw [`EmitEvent`] stream from a document using canonical emit settings.
    pub(crate) fn create_event_stream(doc: &Document) -> Result<Vec<EmitEvent>> {
        doc.emitter()
            .indent(2)
            .width_infinite()
            .indented_seq_in_map(true)
            .preserve_flow_layout(true)
            .mode(Original)
            .emit_events()
            .map_err(|source| Error::Emit { source })
    }

    /// Apply all presentation transformers and serialise the event stream to a YAML string.
    pub(crate) fn emit(&mut self, doc: &Document) -> Result<String> {
        let mut event_stream = Self::create_event_stream(doc)?;

        for EmitEvent {
            content,
            write_type,
        } in &event_stream
        {
            trace!("{:?} - {:?}", write_type, content);
        }

        for transformer in &self.transformers {
            info!(
                "Applying presentation transformer - {}",
                transformer.description()
            );
            event_stream = transformer.process(event_stream);
        }

        let output: String = event_stream.iter().map(|e| e.content.as_str()).collect();
        Ok(output
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n")
    }
}
