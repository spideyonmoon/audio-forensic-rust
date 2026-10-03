//! Offline audio measurements and bounded provisional detector observations.
//! No shell tools or Android runtime needed. Lossy ancestry remains inconclusive.
mod container;
mod decode;
mod detectors;
pub mod dsp;
mod flac_frame;
mod loudness;
mod loudness_range;
pub mod model;
mod source;
mod stereo;
mod true_peak;
mod worker;

pub use decode::{analyze_path, analyze_source};
pub use model::{AnalysisOptions, AnalysisReport, CancellationToken, FileStatus};
pub use symphonia::core::io::MediaSource;
