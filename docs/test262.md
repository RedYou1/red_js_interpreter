# Test262 runner

The project includes a first-stage native Test262 runner at
`test262_runner/src/main.rs`. It is a separate Cargo crate that depends on
the interpreter by path. It executes each test in a fresh realm and isolates
parser or interpreter panics with `catch_unwind`. Each test runs in a child
process so native stack exhaustion is reported as a test failure instead of
terminating the whole sweep.

## Setup

Clone Test262 beside this repository with automatic line-ending conversion
disabled:

```powershell
Set-Location <parent folder>
git -c core.autocrlf=false clone --depth 1 https://github.com/tc39/test262.git
```

## Run

Run the runner in release mode:

```powershell
Set-Location <project folder>\test262_runner
cargo run --release -- `
  --test262-dir ..\..\test262 `
  --fail-fast `
  ..\..\test262\test\language\expressions\addition
```

Each test has a 2-second timeout. A test that does not finish before the
deadline is killed and reported as `FAIL`; use `--timeout-ms <milliseconds>`
to choose a different limit.

Features not implemented by the interpreter can be skipped explicitly:

```powershell
Set-Location <project folder>\test262_runner
cargo run --release -- `
  --test262-dir ..\..\test262 `
  --skip-feature BigInt `
  --skip-feature Symbol `
  ..\..\test262\test\language
```

The runner loads `harness/assert.js`, `harness/sta.js`, and metadata-listed
harness includes. Ordinary tests run in both default and strict mode. Tests
marked `module` or `async` are reported as skipped because those execution paths
are not implemented yet. Parse and early negative tests pass when the
interpreter rejects the source as a `SyntaxError`; resolution negatives remain
skipped because module resolution is not implemented. Runtime negative tests
validate the expected error constructor when the interpreter returns a
`CodeResult::Error`.

The output status is one of `PASS`, `FAIL`, or `SKIP`. The process exits with
status `0` when no test fails, `1` when a test fails, and `2` for an invalid
invocation or missing input. A stack-overflowing interpreter case is reported
as `FAIL ...: interpreter stack overflow`; the parent runner continues unless
`--fail-fast` is supplied.