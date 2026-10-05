//! Offline audio measurements and bounded provisional detector observations.
//! No shell tools or Android runtime needed. Lossy ancestry remains inconclusive.
pub mod byproducts;
mod container;
mod decode;
mod detectors;
pub mod dsp;
pub mod evaluation;
pub mod evidence;
mod flac_frame;
mod job;
mod loudness;
mod loudness_range;
pub mod metadata;
pub mod model;
mod progress;
pub mod reference_inputs;
mod source;
mod stereo;
pub mod tool_statistics;
mod true_peak;
mod worker;

pub use decode::{
    analyze_path, analyze_path_with_byproducts, analyze_path_with_progress,
    analyze_path_with_reference_inputs, analyze_path_with_tool_statistics, analyze_source,
    analyze_source_with_byproducts, analyze_source_with_progress,
    analyze_source_with_reference_inputs, analyze_source_with_tool_statistics,
};
pub use evidence::{EvidenceAssessment, assess_evidence};
pub use job::{AnalysisJob, AnalysisJobError, StartJobError};
pub use model::{AnalysisOptions, AnalysisReport, CancellationToken, FileStatus};
pub use progress::AnalysisProgress;
pub use symphonia::core::io::MediaSource;
