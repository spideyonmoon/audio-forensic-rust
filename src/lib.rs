//! Offline audio measurements and bounded provisional detector observations.
//! No shell tools or Android runtime needed. Lossy ancestry remains inconclusive.
mod decode;
mod detectors;
pub mod dsp;
pub mod model;

pub use decode::{analyze_path, analyze_source};
pub use model::{AnalysisOptions, AnalysisReport, CancellationToken, FileStatus};
pub use symphonia::core::io::MediaSource;
