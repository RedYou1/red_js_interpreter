use std::{
    cell::RefCell,
    collections::HashSet,
    env,
    fs::{self, File},
    io::{self, Read},
    panic::{self, AssertUnwindSafe},
    path::{Path, PathBuf},
    process::{self, Child, Command, Output, Stdio},
    rc::Rc,
    thread,
    time::{Duration, Instant},
};

use red_js_interpreter::{
    CodeResult, Environment, JsValue, LogLevel, Logger, Prototype, default_console_config,
    new_runnable, parse, prebuild_prototypes, run_function_object,
};
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
struct Metadata {
    #[serde(default)]
    flags: Vec<String>,
    #[serde(default)]
    includes: Vec<String>,
    #[serde(default)]
    features: Vec<String>,
    negative: Option<NegativeMetadata>,
}

#[derive(Debug, Deserialize)]
struct NegativeMetadata {
    phase: String,
    #[serde(rename = "type")]
    error_type: String,
}

struct Arguments {
    test262_dir: PathBuf,
    paths: Vec<PathBuf>,
    skip_features: HashSet<String>,
    fail_fast: bool,
    timeout: Duration,
    child: bool,
}

struct Harness {
    assert: String,
    sta: String,
}

#[derive(Debug)]
enum TestOutcome {
    Pass,
    Fail(String),
    Skip(String),
}

enum CaseOutcome {
    Completed(CodeResult),
    ParsePanic,
    CompilePanic,
    Panic,
}

struct Test262Logger;

impl Logger for Test262Logger {
    fn logln(&mut self, _level: LogLevel, _message: &dyn Fn() -> String) {}
}

const STACK_OVERFLOW_EXIT_CODE: i32 = -1_073_741_571;
const DEFAULT_TEST_TIMEOUT: Duration = Duration::from_secs(2);

fn main() {
    let arguments = match parse_arguments(env::args().skip(1)) {
        Ok(arguments) => arguments,
        Err(error) => {
            eprintln!("error: {error}");
            print_usage();
            process::exit(2);
        }
    };

    let tests = match collect_tests(&arguments.paths) {
        Ok(tests) => tests,
        Err(error) => {
            eprintln!("error: {error}");
            process::exit(2);
        }
    };
    if tests.is_empty() {
        eprintln!("error: no JavaScript tests found");
        process::exit(2);
    }

    let harness = match load_harness(&arguments.test262_dir) {
        Ok(harness) => harness,
        Err(error) => {
            eprintln!("error: could not load Test262 harness: {error}");
            process::exit(2);
        }
    };

    if arguments.child {
        let outcome = run_test_in_process(&tests[0], &arguments, &harness);
        match outcome {
            Ok(TestOutcome::Pass) => eprintln!("__TEST262_RESULT__ PASS"),
            Ok(TestOutcome::Fail(reason)) => eprintln!("__TEST262_RESULT__ FAIL {reason}"),
            Ok(TestOutcome::Skip(reason)) => eprintln!("__TEST262_RESULT__ SKIP {reason}"),
            Err(error) => eprintln!("__TEST262_RESULT__ ERROR {error}"),
        }
        return;
    }

    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;
    for test in tests {
        let display_path = test
            .strip_prefix(&arguments.test262_dir)
            .unwrap_or(&test)
            .display();
        match run_test(&test, &arguments) {
            Ok(TestOutcome::Pass) => {
                passed += 1;
                println!("PASS {display_path}");
            }
            Ok(TestOutcome::Fail(reason)) => {
                failed += 1;
                println!("FAIL {display_path}: {reason}");
                if arguments.fail_fast {
                    break;
                }
            }
            Ok(TestOutcome::Skip(reason)) => {
                skipped += 1;
                println!("SKIP {display_path}: {reason}");
            }
            Err(error) => {
                failed += 1;
                println!("FAIL {display_path}: {error}");
                if arguments.fail_fast {
                    break;
                }
            }
        }
    }

    println!("summary: {passed} passed, {failed} failed, {skipped} skipped");
    if failed > 0 {
        process::exit(1);
    }
}

fn parse_arguments<I>(arguments: I) -> Result<Arguments, String>
where
    I: IntoIterator<Item = String>,
{
    let mut arguments = arguments.into_iter();
    let mut test262_dir = env::current_dir().map_err(|error| error.to_string())?;
    let mut paths = Vec::new();
    let mut skip_features = HashSet::new();
    let mut fail_fast = false;
    let mut timeout = DEFAULT_TEST_TIMEOUT;
    let mut child = false;

    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--help" | "-h" => {
                print_usage();
                process::exit(0);
            }
            "--test262-dir" => {
                test262_dir = PathBuf::from(
                    arguments
                        .next()
                        .ok_or("--test262-dir requires a directory")?,
                );
            }
            "--skip-feature" => {
                skip_features.insert(
                    arguments
                        .next()
                        .ok_or("--skip-feature requires a feature name")?,
                );
            }
            "--fail-fast" => fail_fast = true,
            "--timeout-ms" => {
                let milliseconds = arguments
                    .next()
                    .ok_or("--timeout-ms requires a positive integer")?
                    .parse::<u64>()
                    .map_err(|_| "--timeout-ms requires a positive integer".to_owned())?;
                if milliseconds == 0 {
                    return Err("--timeout-ms requires a positive integer".to_owned());
                }
                timeout = Duration::from_millis(milliseconds);
            }
            "--child" => child = true,
            value if value.starts_with('-') => {
                return Err(format!("unknown option {value}"));
            }
            value => paths.push(PathBuf::from(value)),
        }
    }

    if paths.is_empty() {
        return Err("at least one test file or directory is required".to_owned());
    }
    Ok(Arguments {
        test262_dir,
        paths,
        skip_features,
        fail_fast,
        timeout,
        child,
    })
}

fn print_usage() {
    eprintln!(
        "Usage: test262 [OPTIONS] <FILE|DIRECTORY>...\n\n\
         Options:\n\
         --test262-dir <DIR>   Test262 checkout containing harness/\n\
         --skip-feature <NAME> Skip tests declaring this feature (repeatable)\n\
         --fail-fast           Stop after the first failure\n\
         --timeout-ms <MS>     Kill tests running longer than this (default: 2000)\n\
         -h, --help            Show this help"
    );
}

fn collect_tests(paths: &[PathBuf]) -> io::Result<Vec<PathBuf>> {
    let mut tests = Vec::new();
    for path in paths {
        collect_tests_from(path, &mut tests)?;
    }
    tests.sort();
    tests.dedup();
    Ok(tests)
}

fn collect_tests_from(path: &Path, tests: &mut Vec<PathBuf>) -> io::Result<()> {
    let metadata = fs::metadata(path)?;
    if metadata.is_file() {
        let is_test = path.extension().is_some_and(|extension| extension == "js")
            && !path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().contains("_FIXTURE"));
        if is_test {
            tests.push(path.to_owned());
        }
        return Ok(());
    }

    for entry in fs::read_dir(path)? {
        collect_tests_from(&entry?.path(), tests)?;
    }
    Ok(())
}

fn load_harness(test262_dir: &Path) -> io::Result<Harness> {
    Ok(Harness {
        assert: read_file(&test262_dir.join("harness/assert.js"))?,
        sta: read_file(&test262_dir.join("harness/sta.js"))?,
    })
}

fn run_test(path: &Path, arguments: &Arguments) -> Result<TestOutcome, String> {
    let executable = env::current_exe().map_err(|error| error.to_string())?;
    let mut command = Command::new(executable);
    command
        .arg("--child")
        .arg("--test262-dir")
        .arg(&arguments.test262_dir);
    for feature in &arguments.skip_features {
        command.arg("--skip-feature").arg(feature);
    }
    let child = command
        .arg(path)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    let output =
        match wait_for_output(child, arguments.timeout).map_err(|error| error.to_string())? {
            ChildResult::Completed(output) => output,
            ChildResult::TimedOut => {
                return Ok(TestOutcome::Fail(format!(
                    "timed out after {} ms",
                    arguments.timeout.as_millis()
                )));
            }
        };
    if output.status.code() == Some(STACK_OVERFLOW_EXIT_CODE) {
        return Ok(TestOutcome::Fail("interpreter stack overflow".to_owned()));
    }
    if !output.status.success() {
        return Ok(TestOutcome::Fail(format!(
            "test process exited with {}",
            output
                .status
                .code()
                .map_or_else(|| "an unknown status".to_owned(), |code| code.to_string())
        )));
    }

    parse_child_result(&output.stderr)
}

enum ChildResult {
    Completed(Output),
    TimedOut,
}

fn wait_for_output(mut child: Child, timeout: Duration) -> io::Result<ChildResult> {
    let stdout_reader = child.stdout.take().map(|mut stdout| {
        thread::spawn(move || {
            let mut output = Vec::new();
            stdout.read_to_end(&mut output).map(|_| output)
        })
    });
    let stderr_reader = child.stderr.take().map(|mut stderr| {
        thread::spawn(move || {
            let mut output = Vec::new();
            stderr.read_to_end(&mut output).map(|_| output)
        })
    });
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(ChildResult::Completed(Output {
                status,
                stdout: finish_reader(stdout_reader)?,
                stderr: finish_reader(stderr_reader)?,
            }));
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            child.wait()?;
            finish_reader(stdout_reader)?;
            finish_reader(stderr_reader)?;
            return Ok(ChildResult::TimedOut);
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn finish_reader(reader: Option<thread::JoinHandle<io::Result<Vec<u8>>>>) -> io::Result<Vec<u8>> {
    match reader {
        Some(reader) => reader
            .join()
            .map_err(|_| io::Error::other("child output reader panicked"))?,
        None => Ok(Vec::new()),
    }
}

fn run_test_in_process(
    path: &Path,
    arguments: &Arguments,
    harness: &Harness,
) -> Result<TestOutcome, String> {
    let source = read_file(path).map_err(|error| error.to_string())?;
    let metadata = parse_metadata(&source)?;

    if let Some(feature) = metadata
        .features
        .iter()
        .find(|feature| arguments.skip_features.contains(*feature))
    {
        return Ok(TestOutcome::Skip(format!("unsupported feature {feature}")));
    }
    if metadata.flags.iter().any(|flag| flag == "module") {
        return Ok(TestOutcome::Skip(
            "modules are not supported by this runner".to_owned(),
        ));
    }
    if metadata.flags.iter().any(|flag| flag == "async") {
        return Ok(TestOutcome::Skip(
            "async tests are not supported by this runner".to_owned(),
        ));
    }
    if let Some(negative) = &metadata.negative
        && !matches!(negative.phase.as_str(), "parse" | "early" | "runtime")
    {
        return Ok(TestOutcome::Skip(format!(
            "negative {} tests are not supported by this runner",
            negative.phase
        )));
    }

    let raw = metadata.flags.iter().any(|flag| flag == "raw");
    let only_strict = metadata.flags.iter().any(|flag| flag == "onlyStrict");
    let no_strict = metadata.flags.iter().any(|flag| flag == "noStrict") || raw;
    let strict_variants = if no_strict {
        vec![false]
    } else if only_strict {
        vec![true]
    } else {
        vec![false, true]
    };

    let mut failures = Vec::new();
    for strict in strict_variants {
        let test_source = if strict {
            format!("\"use strict\";\n{source}")
        } else {
            source.clone()
        };
        let source = if raw {
            test_source
        } else {
            build_source(&arguments.test262_dir, harness, &metadata, &test_source)?
        };
        match execute_case(&source) {
            CaseOutcome::Completed(result) => {
                let reason = if matches!(
                    metadata
                        .negative
                        .as_ref()
                        .map(|negative| negative.phase.as_str()),
                    Some("parse" | "early")
                ) {
                    Some(format!(
                        "expected {} during {} phase, but completed normally",
                        metadata.negative.as_ref().unwrap().error_type,
                        metadata.negative.as_ref().unwrap().phase
                    ))
                } else {
                    check_result(result, metadata.negative.as_ref())
                };
                if let Some(reason) = reason {
                    failures.push(format!("{} mode: {reason}", mode_name(strict)));
                }
            }
            CaseOutcome::ParsePanic => {
                if let Some(negative) = metadata.negative.as_ref()
                    && matches!(negative.phase.as_str(), "parse" | "early")
                {
                    if negative.error_type != "SyntaxError" {
                        failures.push(format!(
                            "{} mode: expected {}, got SyntaxError",
                            mode_name(strict),
                            negative.error_type
                        ));
                    }
                } else {
                    failures.push(format!("{} mode: parser panic", mode_name(strict)));
                }
            }
            CaseOutcome::CompilePanic => {
                if let Some(negative) = metadata.negative.as_ref()
                    && negative.phase == "early"
                {
                    if negative.error_type != "SyntaxError" {
                        failures.push(format!(
                            "{} mode: expected {}, got SyntaxError",
                            mode_name(strict),
                            negative.error_type
                        ));
                    }
                } else {
                    failures.push(format!("{} mode: interpreter panic", mode_name(strict)));
                }
            }
            CaseOutcome::Panic => {
                failures.push(format!("{} mode: interpreter panic", mode_name(strict)));
            }
        }
    }

    if failures.is_empty() {
        Ok(TestOutcome::Pass)
    } else {
        Ok(TestOutcome::Fail(failures.join("; ")))
    }
}

fn parse_child_result(output: &[u8]) -> Result<TestOutcome, String> {
    let output = String::from_utf8_lossy(output);
    let result = output
        .lines()
        .rev()
        .find_map(|line| line.strip_prefix("__TEST262_RESULT__ "))
        .ok_or_else(|| "test process returned no result".to_owned())?;
    match result
        .split_once(' ')
        .map_or((result, ""), |(status, reason)| (status, reason))
    {
        ("PASS", _) => Ok(TestOutcome::Pass),
        ("FAIL", reason) => Ok(TestOutcome::Fail(reason.to_owned())),
        ("SKIP", reason) => Ok(TestOutcome::Skip(reason.to_owned())),
        ("ERROR", reason) => Err(reason.to_owned()),
        (status, _) => Err(format!("unknown child result {status}")),
    }
}

fn parse_metadata(source: &str) -> Result<Metadata, String> {
    let Some(start) = source.find("/*---") else {
        return Ok(Metadata::default());
    };
    let metadata_start = start + "/*---".len();
    let metadata_end = source[metadata_start..]
        .find("---*/")
        .ok_or("unterminated Test262 frontmatter")?
        + metadata_start;
    serde_yaml::from_str(&source[metadata_start..metadata_end])
        .map_err(|error| format!("invalid Test262 frontmatter: {error}"))
}

fn build_source(
    test262_dir: &Path,
    harness: &Harness,
    metadata: &Metadata,
    test_source: &str,
) -> Result<String, String> {
    let mut source = format!("{}\n{}\n", harness.assert, harness.sta);
    for include in &metadata.includes {
        let include_path = test262_dir.join("harness").join(include);
        let include_source = read_file(&include_path).map_err(|error| {
            format!(
                "could not load harness include {}: {error}",
                include_path.display()
            )
        })?;
        source.push_str(&include_source);
        source.push('\n');
    }
    source.push_str(test_source);
    Ok(source)
}

fn execute_case(source: &str) -> CaseOutcome {
    let logger: Rc<RefCell<dyn Logger>> = Rc::new(RefCell::new(Test262Logger));
    let env = Environment {
        mem: prebuild_prototypes(default_console_config, logger.clone()),
        logger,
    };

    let parsed = panic::catch_unwind(AssertUnwindSafe(|| parse(source, env.clone())));
    let program = match parsed {
        Ok(program) => program,
        Err(_) => return CaseOutcome::ParsePanic,
    };

    let compiled = panic::catch_unwind(AssertUnwindSafe(|| program.compile(env.clone())));
    let program = match compiled {
        Ok(program) => program,
        Err(_) => return CaseOutcome::CompilePanic,
    };

    let executed = panic::catch_unwind(AssertUnwindSafe(|| {
        let function = Prototype::find(env.mem.clone(), &JsValue::String("Function".to_owned()))
            .1
            .borrow()
            .unwrap_proto("test262 Function prototype");
        let main = new_runnable(function, "__test262__", program);
        let main = main.borrow().unwrap_proto("test262 main function");
        run_function_object(
            main,
            Rc::new(std::cell::RefCell::new(JsValue::Undefined)),
            vec![],
            env.logger.clone(),
        )
    }));

    match executed {
        Ok(result) => CaseOutcome::Completed(result),
        Err(_) => CaseOutcome::Panic,
    }
}

fn check_result(result: CodeResult, negative: Option<&NegativeMetadata>) -> Option<String> {
    match (result, negative) {
        (CodeResult::Error(error), Some(negative)) => {
            let actual = error_name(&error);
            if actual.as_deref() == Some(negative.error_type.as_str()) {
                None
            } else {
                Some(format!(
                    "expected {}, got {}",
                    negative.error_type,
                    actual.unwrap_or_else(|| "unknown error".to_owned())
                ))
            }
        }
        (CodeResult::Error(error), None) => Some(format!(
            "uncaught {}",
            error_name(&error).unwrap_or_else(|| "unknown error".to_owned())
        )),
        (_, Some(negative)) => Some(format!(
            "expected {} but completed normally",
            negative.error_type
        )),
        _ => None,
    }
}

fn error_name(error: &Rc<std::cell::RefCell<JsValue>>) -> Option<String> {
    let JsValue::Prototype(error_object) = &*error.borrow() else {
        return None;
    };
    let constructor = Prototype::find(
        error_object.clone(),
        &JsValue::String("constructor".to_owned()),
    )
    .1;
    let JsValue::Prototype(constructor) = &*constructor.borrow() else {
        return None;
    };
    constructor.borrow().name.map(str::to_owned)
}

fn mode_name(strict: bool) -> &'static str {
    if strict { "strict" } else { "default" }
}

fn read_file(path: &Path) -> io::Result<String> {
    let mut source = String::new();
    File::open(path)?.read_to_string(&mut source)?;
    Ok(source)
}

#[cfg(test)]
mod tests {
    use std::{
        process::{Command, Stdio},
        time::Duration,
    };

    use super::{CaseOutcome, ChildResult, execute_case, parse_metadata, wait_for_output};

    #[test]
    fn parses_test262_frontmatter() {
        let metadata = parse_metadata(
            "/*---\nflags: [onlyStrict]\nincludes: [propertyHelper.js]\nfeatures: [Array]\nnegative:\n  phase: runtime\n  type: TypeError\n---*/\n",
        )
        .expect("frontmatter should parse");

        assert_eq!(metadata.flags, ["onlyStrict"]);
        assert_eq!(metadata.includes, ["propertyHelper.js"]);
        assert_eq!(metadata.features, ["Array"]);
        assert_eq!(metadata.negative.as_ref().unwrap().error_type, "TypeError");
    }

    #[test]
    fn accepts_tests_without_frontmatter() {
        let metadata = parse_metadata("1 + 1;").expect("metadata should be optional");
        assert!(metadata.flags.is_empty());
        assert!(metadata.negative.is_none());
    }

    #[test]
    fn executes_simple_supported_source() {
        assert!(matches!(execute_case("1 + 1;"), CaseOutcome::Completed(_)));
    }

    #[test]
    fn rejects_invalid_source_for_parse_negative_tests() {
        assert!(matches!(execute_case("{ 1 2 } 3"), CaseOutcome::ParsePanic));
    }

    #[test]
    fn kills_child_after_timeout() {
        let child = if cfg!(windows) {
            Command::new("cmd")
                .args(["/C", "ping 127.0.0.1 -n 6 > nul"])
                .spawn()
                .expect("sleeping child should start")
        } else {
            Command::new("sh")
                .args(["-c", "sleep 5"])
                .spawn()
                .expect("sleeping child should start")
        };

        let result = wait_for_output(child, Duration::from_millis(25))
            .expect("timed-out child should be reaped");
        assert!(matches!(result, ChildResult::TimedOut));
    }

    #[test]
    fn drains_large_child_output_before_waiting() {
        let child = if cfg!(windows) {
            Command::new("cmd")
                .args(["/C", "for /L %i in (1,1,20000) do @echo test"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("output-producing child should start")
        } else {
            Command::new("sh")
                .args(["-c", "yes test | head -n 20000"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("output-producing child should start")
        };

        let ChildResult::Completed(output) =
            wait_for_output(child, Duration::from_secs(2)).expect("child should complete")
        else {
            panic!("output-producing child should not time out");
        };
        assert!(output.stdout.len() > 65_536);
    }
}
