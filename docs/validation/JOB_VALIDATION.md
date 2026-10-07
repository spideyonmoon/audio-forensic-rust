# Background job lifecycle — 2026-10-04

Engine 0.20.0 adds `AnalysisJob`, `StartJobError` and `AnalysisJobError` without
dependencies, report schema changes, detector changes or ancestry policy changes.
The source and path APIs run on a named Rust worker thread. The background
consumer example uses the public wrapper rather than assembling its own thread,
cancellation token and result channel.

Admission is one unfinished background job per process, separate from the
existing single active core-analysis budget shared with synchronous calls.
An atomic RAII permit covers thread creation, source use/release and panic
unwinding. Additional jobs return Busy, with no pending queue. A failed source
start consumes/drops the supplied source. A completed job no longer reserves
admission, even if its result has not been consumed.

Progress occupies one replaceable mutex-protected value. Polling clones it;
slow hosts may miss stages. Completion uses the standard JoinHandle's finished
check before joining, so polling never waits for ongoing analysis. Results are
consumed once. Ordinary file outcomes remain AnalysisReport values; worker
panics become a separate host error and need not emit terminal progress.

Handle drop requests cancellation and detaches without waiting. Admission stays
held until the worker releases its input. Blocked source I/O remains cooperative,
not forcibly interruptible. A host must bound its own completed report history.
Thread startup/path opening are outside the existing decode deadline; waiting
for the synchronous core worker is inside it. Android remains deferred.

## Checks

Generated-only lifecycle checks in `tests/jobs.rs` cover exact report parity,
single result consumption, final progress retention, completed-handle admission,
Busy rejection, cancellation during a deliberately blocked source read,
non-waiting handle drop, retained admission/source ownership after teardown,
recovery after I/O release, worker/source panic recovery, path-open failure,
valid path analysis, unsupported input and zero-deadline reports. Test scenarios
serialize admission within their binary while remaining compatible with ordinary
parallel Cargo test execution.

Actual outcomes:

- Rust 1.85 offline no-CLI focused run: **9 passed**, zero failed/ignored (four
  lifecycle, four progress and one core worker-budget test), ordinary parallel
  test scheduling. The initial sandbox build failed at linker execution with
  Permission denied; its authorized retry passed. Both logs are retained.
- Rust 1.85 all-target Clippy passed with warnings denied; the updated no-CLI
  background example built successfully.
- The existing generated consumer checker passed all **five** report/exit/schema
  cases: WAV, FLAC, missing path, unsupported bytes and immediate cancellation.
  WAV/FLAC PCM hashes exactly matched the independent fixture oracle. Binary and
  input hashes remained unchanged. Immediate-cancellation smoke timing remains
  scheduling-dependent; the blocked-read unit supplies deterministic control.
- Stable formatting, whitespace and schema drift passed (47 unchanged model
  definitions). The reference checkout remains clean at the pinned commit.

Logs are local/ignored in `corpus/local/results/jobs-v20-20261004/`. No task jobs
remain running. Full regression, forced OS thread-creation failure, abort-mode
panic recovery and Android linking/device behavior were not tested for 0.20.0.
No private-audio analysis, commit, upload or publication occurred.
