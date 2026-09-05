use std::{fs, io, path, process, time};

use crate::{build, log};

const PRELUDE: &str = include_str!("check/prelude.ts");

#[derive(Debug, Eq, PartialEq)]
struct TypeScriptDiagnostic {
	file: Option<String>,
	line: Option<usize>,
	column: Option<usize>,
	level: log::LogLevel,
	code: Option<String>,
	message: String,
}

struct TemporaryFiles {
	paths: Vec<path::PathBuf>,
}

impl Drop for TemporaryFiles {
	fn drop(&mut self) {
		for path in &self.paths {
			let _ = fs::remove_file(path);
		}
	}
}

fn parse_level_and_message(source: &str) -> Option<(log::LogLevel, Option<String>, String)> {
	let (head, message) = source.split_once(": ")?;
	let mut words = head.split_ascii_whitespace();
	let level = match words.next()? {
		"error" => log::LogLevel::Error,
		"warning" => log::LogLevel::Warn,
		"info" => log::LogLevel::Info,
		_ => return None,
	};
	let code = words.next().map(str::to_string);
	Some((level, code, message.to_string()))
}

fn parse_typescript_diagnostic(source: &str) -> Option<TypeScriptDiagnostic> {
	if let Some((location, rest)) = source.split_once("): ") {
		if let Some(open_parenthesis) = location.rfind('(') {
			let file = &location[..open_parenthesis];
			let (line, column) = location[open_parenthesis + 1..].split_once(',')?;
			let line = line.parse().ok()?;
			let column = column.parse().ok()?;
			let (level, code, message) = parse_level_and_message(rest)?;
			let file = Some(file.to_string());
			let (line, column) = (Some(line), Some(column));
			return Some(TypeScriptDiagnostic { file, line, column, level, code, message });
		}
	}

	let (level, code, message) = parse_level_and_message(source)?;
	Some(TypeScriptDiagnostic { file: None, line: None, column: None, level, code, message })
}

fn checker_directory(compilation: &build::Compilation) -> path::PathBuf {
	let project_root = compilation.config.path.parent().unwrap_or_else(|| path::Path::new("."));
	compilation.config.target.path.as_deref()
		.and_then(|target| path::Path::new(target).parent())
		.map_or_else(|| project_root.to_path_buf(), |target_parent| project_root.join(target_parent))
}

fn temporary_paths(compilation: &build::Compilation) -> (path::PathBuf, path::PathBuf) {
	let nonce = time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap_or_default().as_nanos();
	let stem = format!(".vue-script-check-{}-{nonce}", std::process::id());
	let directory = checker_directory(compilation);
	(directory.join(format!("{stem}.js")), directory.join(format!("{stem}.d.ts")))
}

fn typescript_command(config_path: &path::Path, configured: Option<&str>) -> path::PathBuf {
	#[cfg(windows)]
	let command_name = "tsc.cmd";
	#[cfg(not(windows))]
	let command_name = "tsc";

	let Some(configured) = configured else {
		return path::PathBuf::from(command_name);
	};
	let configured = path::Path::new(configured);
	if configured.is_absolute() {
		configured.to_path_buf()
	}
	else {
		config_path.parent().unwrap_or_else(|| path::Path::new(".")).join(configured)
	}
}

fn run_typescript(compilation: &build::Compilation, javascript_path: &path::Path, declarations_path: &path::Path) -> io::Result<process::Output> {
	let command = typescript_command(&compilation.config.path, compilation.config.check.typescript.as_deref());
	let target = compilation.config.check.target.as_deref().expect("check target should be validated before running TypeScript");
	process::Command::new(command)
		.current_dir(checker_directory(compilation))
		.args([
			"--pretty", "false",
			"--noEmit",
			"--allowJs",
			"--checkJs",
			"--noImplicitAny", "false",
			"--noImplicitThis", "true",
			"--target", target,
			"--module", "esnext",
			"--moduleResolution", "bundler",
			"--skipLibCheck",
		])
		.arg(declarations_path)
		.arg(javascript_path)
		.output()
}

fn path_matches(left: &str, right: &path::Path, working_directory: &path::Path) -> bool {
	let left = path::Path::new(left);
	left == right || (left.is_relative() && working_directory.join(left) == right)
}

fn display_source_path(path: &path::Path, project_root: &path::Path, working_directory: &path::Path) -> path::PathBuf {
	let path = if path.is_relative() { working_directory.join(path) } else { path.to_path_buf() };
	path.strip_prefix(project_root).unwrap_or(&path).to_path_buf()
}

fn report_typescript_diagnostic(
	log: &mut log::Logger,
	compilation: &build::Compilation,
	javascript_path: &path::Path,
	diagnostic: TypeScriptDiagnostic,
) {
	let project_root = compilation.config.path.parent().unwrap_or_else(|| path::Path::new("."));
	let working_directory = checker_directory(compilation);
	let mut file = diagnostic.file;
	let mut line = diagnostic.line;
	let mut column = diagnostic.column;

	if let (Some(diagnostic_file), Some(generated_line)) = (file.as_deref(), line) {
		if path_matches(diagnostic_file, javascript_path, &working_directory) {
			if let Some((source_file, source_line, source_column)) = column
				.and_then(|generated_column| compilation.javascript.map_position(generated_line, generated_column))
			{
				file = Some(source_file.to_string());
				line = Some(source_line);
				column = Some(source_column);
			}
			else if let Some((source_file, source_line)) = compilation.javascript.map_line(generated_line) {
				file = Some(source_file.to_string());
				line = Some(source_line);
			}
			else {
				file = None;
				line = None;
			}
		}
		else {
			let path = display_source_path(path::Path::new(diagnostic_file), project_root, &working_directory);
			file = Some(path.to_string_lossy().into_owned());
		}
	}

	let source = file.as_deref().and_then(|file| fs::read_to_string(project_root.join(file)).ok());
	let span = match (file.as_deref(), line, column) {
		(Some(file), Some(line), Some(column)) => Some(log::LineSpan {
			file,
			line_start: line,
			line_end: line,
			column_start: column.saturating_sub(1),
			column_end: column,
		}),
		_ => None,
	};
	log.log_with_code(source.as_deref(), log::LogEntry {
		level: diagnostic.level,
		span,
		message: diagnostic.message,
		note: None,
	}, diagnostic.code);
}

fn check_typescript(log: &mut log::Logger, compilation: &build::Compilation) -> Result<(), build::BuildError> {
	if compilation.config.check.target.as_deref().is_none_or(str::is_empty) {
		log.log(None, log::LogEntry {
			level: log::LogLevel::Error,
			span: None,
			message: "Missing required [check].target setting.".to_string(),
			note: Some("Set [check].target to the ECMAScript version supported by the project, for example target = \"es2022\"."),
		});
		return Err(build::BuildError);
	}
	let (javascript_path, declarations_path) = temporary_paths(compilation);
	let Some(directory) = javascript_path.parent() else {
		return Err(build::BuildError);
	};
	if let Err(err) = fs::create_dir_all(directory) {
		log.log(None, log::LogEntry {
			level: log::LogLevel::Error,
			span: None,
			message: format!("Failed to create the TypeScript check directory \"{}\": {}", directory.display(), err),
			note: Some("Check that the generated target directory is writable."),
		});
		return Err(build::BuildError);
	}

	let temporary_files = TemporaryFiles {
		paths: vec![javascript_path.clone(), declarations_path.clone()],
	};
	let mut javascript = compilation.javascript.source.clone();
	if !javascript.ends_with('\n') {
		javascript.push('\n');
	}
	javascript.push_str("export {};\n");
	if let Err(err) = fs::write(&javascript_path, javascript) {
		log.log(None, log::LogEntry {
			level: log::LogLevel::Error,
			span: None,
			message: format!("Failed to write temporary JavaScript for TypeScript: {}", err),
			note: Some("Check that the generated target directory is writable."),
		});
		return Err(build::BuildError);
	}
	if let Err(err) = fs::write(&declarations_path, PRELUDE) {
		log.log(None, log::LogEntry {
			level: log::LogLevel::Error,
			span: None,
			message: format!("Failed to write temporary declarations for TypeScript: {}", err),
			note: Some("Check that the generated target directory is writable."),
		});
		return Err(build::BuildError);
	}

	let output = match run_typescript(compilation, &javascript_path, &declarations_path) {
		Ok(output) => output,
		Err(err) if err.kind() == io::ErrorKind::NotFound => {
			log.log(None, log::LogEntry {
				level: log::LogLevel::Error,
				span: None,
				message: "Could not find the TypeScript compiler.".to_string(),
				note: Some("Check [check].typescript or make tsc available on PATH."),
			});
			return Err(build::BuildError);
		},
		Err(err) => {
			log.log(None, log::LogEntry {
				level: log::LogLevel::Error,
				span: None,
				message: format!("Failed to run the TypeScript compiler: {}", err),
				note: Some("Check [check].typescript and the compiler's executable permissions."),
			});
			return Err(build::BuildError);
		},
	};

	let mut parsed = 0;
	for line in String::from_utf8_lossy(&output.stdout).lines()
		.chain(String::from_utf8_lossy(&output.stderr).lines())
	{
		if let Some(diagnostic) = parse_typescript_diagnostic(line) {
			parsed += 1;
			report_typescript_diagnostic(log, compilation, &javascript_path, diagnostic);
		}
	}

	if !output.status.success() && parsed == 0 {
		let stderr = String::from_utf8_lossy(&output.stderr);
		let stdout = String::from_utf8_lossy(&output.stdout);
		let details = if stderr.trim().is_empty() { stdout.trim() } else { stderr.trim() };
		log.log(None, log::LogEntry {
			level: log::LogLevel::Error,
			span: None,
			message: if details.is_empty() {
				format!("TypeScript exited with status {} without reporting a diagnostic.", output.status)
			}
			else {
				format!("TypeScript failed: {details}")
			},
			note: None,
		});
	}

	drop(temporary_files);
	if log.has_errors() { Err(build::BuildError) } else { Ok(()) }
}

pub fn main(log: &mut log::Logger) -> Result<(), build::BuildError> {
	let compilation = build::compile(log)?;
	let _ = check_typescript(log, &compilation);
	if log.has_errors() { Err(build::BuildError) } else { Ok(()) }
}

#[cfg(test)]
mod tests;
