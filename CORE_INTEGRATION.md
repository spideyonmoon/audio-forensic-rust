# Measurement core integration baseline

The core supplies an offline WAV/FLAC **measurement component** for a
host application. It does not supply a calibrated authenticity classifier.
The first application can expose the implemented measurements and provisional
observations without waiting for source-history research to finish. Android
bindings, NDK linking, lifecycle handling and phone validation remain separate
engineering work; this document does not declare those complete.

The desktop baseline passed its closing validation on 2026-10-03: all 164
release tests, Rust 1.85 all-target Clippy and CLI-free consumer build, plus
five frozen-consumer report checks (including two exact PCM controls). See
`CORE_ACCEPTANCE_VALIDATION.md` for retained failures, checks and scope.

## Use the library without the desktop CLI

```toml
[dependencies]
audio-forensic = { path = "../audio-forensic-rust", default-features = false }
```

`examples/background_analysis.rs` is a runnable host consumer using only public
API and existing dependencies:

```text
cargo run --locked --offline --no-default-features --example background_analysis -- input.wav
cargo run --locked --offline --no-default-features --example background_analysis -- input.flac 25
```

The second command requests cancellation after approximately 25 ms (the console
polls every 10 ms). It can still succeed if analysis finishes first. The example
uses `AnalysisJob::spawn_path` to open and analyze on a worker, polls its result,
returns one JSON report and exits 0 only for `analyzed`. The console continues
polling after cancellation; a UI polls from its event loop. A worker panic is a
host error, not a fabricated analysis report. The example also prints changes
in the latest progress on stderr. The CLI exposes the callback-based
capability with `--progress`; JSON remains exclusively on stdout.

## Background jobs (0.20.0)

`AnalysisJob::spawn_source(source, name, options)` transfers an app-owned source;
`AnalysisJob::spawn_path(path, options)` also moves file opening off the caller
thread. Both return a job handle or `StartJobError`. At most one unfinished job
is admitted per process. `Busy` immediately rejects additional work without
spawning waiting threads or retaining a queue. On a failed start, supplied
sources are dropped; retrying requires a fresh source. Thread creation errors
are separate from file reports.

- `progress()` returns a copy of the latest event. Storage is one replaceable
  value; slow polling can miss intermediate stages, including entire decode
  passes. It does not accumulate an unbounded notification queue.
- `try_finish()` returns `None` while running and one `Some(Result<AnalysisReport,
  AnalysisJobError>)` when the worker has exited. Further calls return `None`.
  `is_finished()` remains true after consumption. Worker panics use the normal
  Rust panic hook and become `WorkerPanicked`; file failures retain their report
  status. A Finished progress event can precede result availability briefly.
- `cancel()` requests cooperative cancellation. Dropping the handle also cancels
  and detaches without joining ongoing analysis. Even after drop, admission stays
  occupied until the source is released and the worker is exiting. A blocked
  source read cannot be forcibly interrupted; the next start remains Busy.

This wrapper creates no async-runtime dependency and builds without the CLI.
Direct synchronous calls still use the same core worker budget and can make a
background job wait; that waiting counts toward its analysis deadline. The
deadline starts inside the analysis call, excluding thread startup and path
opening. A completed handle/report is caller-owned; the host must bound its own
retained result history. This is a portable Rust lifecycle interface, not an
Android binding or device validation.

For an application-owned handle use `analyze_source(Box<dyn MediaSource>, name,
options, cancel)`. The name is a display label; it does not select the format.
The source must implement Read/Seek and the MediaSource contract and remain
stable across preflight and both passes. Unknown byte length is supported.
Nonseekable streams require caller-managed staging with an application storage
limit. No platform handle adapter or staging facility is supplied by this core.

## Host lifecycle and result contract

1. Assign a job identity, create a fresh cancellation token and move the source
   into a background call. The source is released when the call returns.
2. For direct synchronous integration, retain a token clone for cancellation. A cancelled token cannot be reset;
   retries need a new one. Limit queued direct calls at the host: the core serializes
   active analyses but does not bound caller threads or retained inputs.
3. Treat cancellation and deadlines as cooperative. Waiting for the core worker
   counts toward the deadline; blocked caller I/O cannot be interrupted. Dropping
   a custom UI receiver or raw thread handle does not itself cancel analysis;
   the supplied AnalysisJob handle does. On lifecycle
   teardown request cancellation and handle eventual completion outside the UI.
4. Match all five file statuses: `analyzed`, `unsupported`, `failed`, `cancelled`,
   `timed_out`. Show diagnostics for unsuccessful outcomes; partial stream
   metadata is not a successful measurement. Handle host thread/adapter errors
   separately. Version 0.19.0 adds `analyze_source_with_progress` and the public
   `AnalysisProgress` enum. It emits worker waiting, metadata, per-pass decoded
   frame counts, detector analysis and one terminal status on normal return.
   Decode updates are throttled to 100 ms except pass start/end. Callbacks run
   synchronously on the analysis thread: forward to a bounded host channel or
   replace a latest-value slot without blocking; never reenter analysis.
   Callback time counts toward the deadline; panics unwind normally.
   `expected_frames` is a declared length or requested upper bound on pass one
   and the actual first-pass length on pass two. It can be unknown or inaccurate.
   Keep detector progress indeterminate; decoded frames are not a time estimate
   and reaching a pass boundary does not mean the report is ready.
5. On success preserve actual coverage, native channel indices, detector status,
   units and null values. A detector can abstain within a successful file report.
   A prefix describes that prefix, not whole-file integrity or ancestry.
6. Persist the original versioned report. Dispatch by `schema_version`, preserve
   exact u64 integers, and treat unsupported future versions explicitly. The
   current schema is 0.18.0 and policy is `observations-only-v18`; these are
   distinct from engine version 0.21.0. Rust API/ABI is not frozen or a C ABI.

For non-MQA interpretation, call `assess_evidence(&report)`. Its domain groups
reference original detector indices, preserving detailed scope/statuses while
deduplicating named method hits. Store a serialized assessment with its original
report. `assessment_version` is independent of the report schema. The CLI offers
the same grouping with `--summary`; `examples/explain_report.rs` can interpret
saved reports without re-decoding audio. Read `EVIDENCE_INTERPRETATION.md` before
presenting any inferred source-history claims.

All ancestry results remain `INCONCLUSIVE`, evidence index remains null, and
hits remain provisional observations. Do not translate absence of hits into
"authentic", "lossless source" or "clean". The support matrix and precise
limitations are in README.md and the linked validation records.

## Finite completion boundary

The desktop measurement baseline consists of the implemented modules, tested
WAV/FLAC support, source ownership/control, structured outcomes, versioned
report shape, and a CLI-free consumer. Its existing numerical, corruption,
resource and source tests are the acceptance evidence. New speculative parser
matrices are follow-up hardening unless a concrete defect blocks this scope.

Separate follow-up work includes additional formats, finer AAC phase/window
research, independently grouped detector accuracy, ancestry aggregation,
source-medium/bit-depth inference and structural MQA confirmation. Those are
not prerequisites for displaying the current measurements correctly. They
remain prerequisites for any future claims depending on them.

The next application engineering stage is Android binding/link and lifecycle
work, followed by on-device validation. The user authorized this delivery track
on 2026-10-05; follow A01–A08 in ROADMAP_TASKS.md. It does not wait for endgame
calibration. P01 defines the faithful product scope and separate reference
assessment; the historical measurement baseline above remains its own scope.
