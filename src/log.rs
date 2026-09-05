use std::io::{self, IsTerminal, Write};

use serde::Serialize;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
	Error,
	#[serde(rename = "warning")]
	Warn,
	Info,
}

impl LogLevel {
	pub fn as_str(&self) -> &'static str {
		match self {
			LogLevel::Error => "error",
			LogLevel::Warn => "warn",
			LogLevel::Info => "info",
		}
	}

	fn ansi_color(&self) -> &'static str {
		match self {
			LogLevel::Error => "1;31",
			LogLevel::Warn => "1;33",
			LogLevel::Info => "1;36",
		}
	}
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineSpan<'a> {
	pub file: &'a str,
	pub line_start: usize,
	pub line_end: usize,
	/// Zero-indexed, matching tagsoup source spans.
	pub column_start: usize,
	/// Zero-indexed, matching tagsoup source spans.
	pub column_end: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogEntry<'a> {
	pub level: LogLevel,
	pub span: Option<LineSpan<'a>>,
	pub message: String,
	pub note: Option<&'a str>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DiagnosticSpan {
	pub file: String,
	pub line_start: usize,
	pub line_end: usize,
	/// One-indexed for human- and tool-facing output.
	pub column_start: usize,
	/// One-indexed for human- and tool-facing output.
	pub column_end: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Diagnostic {
	pub level: LogLevel,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub code: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub span: Option<DiagnosticSpan>,
	pub message: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub note: Option<String>,
	#[serde(skip)]
	source_line: Option<String>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MessageFormat {
	Human,
	Json,
}

pub struct Logger {
	diagnostics: Vec<Diagnostic>,
	format: MessageFormat,
}

impl Logger {
	pub fn new() -> Logger {
		Logger::with_format(MessageFormat::Human)
	}

	pub fn with_format(format: MessageFormat) -> Logger {
		Logger {
			diagnostics: Vec::new(),
			format,
		}
	}

	pub fn has_errors(&self) -> bool {
		self.diagnostics.iter().any(|diagnostic| diagnostic.level == LogLevel::Error)
	}

	pub fn reset(&mut self) {
		self.diagnostics.clear();
	}

	pub fn log(&mut self, source: Option<&str>, entry: LogEntry<'_>) {
		self.log_with_code(source, entry, None);
	}

	pub fn log_with_code(&mut self, source: Option<&str>, entry: LogEntry<'_>, code: Option<String>) {
		let source_line = entry.span.as_ref().and_then(|span| get_source_line(source, span)).map(str::to_string);
		let span = entry.span.map(|span| DiagnosticSpan {
			file: span.file.to_string(),
			line_start: span.line_start,
			line_end: span.line_end,
			column_start: span.column_start + 1,
			column_end: span.column_end + 1,
		});
		let diagnostic = Diagnostic {
			level: entry.level,
			code,
			span,
			message: entry.message,
			note: entry.note.map(str::to_string),
			source_line,
		};

		if self.format == MessageFormat::Human {
			let stderr = io::stderr();
			let mut stderr = stderr.lock();
			let write_fn = if stderr.is_terminal() { write_colored_diagnostic } else { write_plain_diagnostic };
			write_fn(&mut stderr, &diagnostic);
		}

		self.diagnostics.push(diagnostic);
	}

	pub fn finished(&self) -> bool {
		let errors = self.error_count();
		let warns = self.warning_count();

		match self.format {
			MessageFormat::Human => {
				let stderr = io::stderr();
				let mut stderr = stderr.lock();
				let _ = writeln!(stderr, "Finished with {errors} error(s), {warns} warning(s).");
			},
			MessageFormat::Json => {
				let stdout = io::stdout();
				let mut stdout = stdout.lock();
				let _ = write_json(&mut stdout, &self.diagnostics);
			},
		}

		errors == 0
	}

	pub fn error_count(&self) -> usize {
		self.diagnostics.iter().filter(|diagnostic| diagnostic.level == LogLevel::Error).count()
	}

	#[cfg(test)]
	pub fn has_warnings(&self) -> bool {
		self.diagnostics.iter().any(|diagnostic| diagnostic.level == LogLevel::Warn)
	}

	pub fn warning_count(&self) -> usize {
		self.diagnostics.iter().filter(|diagnostic| diagnostic.level == LogLevel::Warn).count()
	}
}

fn get_source_line<'a>(source: Option<&'a str>, span: &LineSpan) -> Option<&'a str> {
	source.and_then(|source| source.lines().nth(span.line_start.saturating_sub(1)))
}

fn caret_prefix(source_line: &str, column_start: usize) -> String {
	source_line.chars().take(column_start.saturating_sub(1)).map(|ch| match ch { '\t' => '\t', _ => ' ' }).collect()
}

fn write_json(mut writer: impl Write, diagnostics: &[Diagnostic]) -> io::Result<()> {
	serde_json::to_writer_pretty(&mut writer, diagnostics)?;
	writeln!(writer)
}

fn write_plain_diagnostic(mut writer: impl Write, diagnostic: &Diagnostic) {
	let level_str = diagnostic.level.as_str();
	let code = diagnostic.code.as_deref().map(|code| format!("[{code}]")).unwrap_or_default();
	let message = &diagnostic.message;
	let _ = writeln!(writer, "{level_str}{code}: {message}");

	if let Some(span) = &diagnostic.span {
		let span_file = &span.file;
		let span_line = span.line_start;
		let location_column = span.column_start;
		let _ = writeln!(writer, " --> {span_file}:{span_line}:{location_column}");

		if let Some(source_line) = &diagnostic.source_line {
			let gutter_width = span_line.to_string().len();
			let caret_prefix = caret_prefix(source_line, span.column_start);
			let caret_count = usize::max(1, span.column_end.saturating_sub(span.column_start));
			let blank = "";

			let _ = writeln!(writer, "{0:>gutter_width$} |", blank);
			let _ = writeln!(writer, "{span_line:>gutter_width$} | {source_line}");
			let _ = writeln!(writer, "{0:>gutter_width$} | {caret_prefix}{0:^>caret_count$}", blank);
		}
	}

	if let Some(note) = &diagnostic.note {
		let _ = writeln!(writer, " help: {note}");
	}
}

fn write_colored_diagnostic(mut writer: impl Write, diagnostic: &Diagnostic) {
	let level_color = diagnostic.level.ansi_color();
	let level_str = diagnostic.level.as_str();
	let code = diagnostic.code.as_deref().map(|code| format!("[{code}]")).unwrap_or_default();
	let message = &diagnostic.message;
	let _ = writeln!(writer, "\x1b[{level_color}m{level_str}{code}\x1b[0m\x1b[1m: {message}\x1b[0m");

	if let Some(span) = &diagnostic.span {
		let span_file = &span.file;
		let span_line_start = span.line_start;
		let span_column_start = span.column_start;
		let _ = writeln!(writer, " \x1b[1;34m-->\x1b[0m {span_file}:{span_line_start}:{span_column_start}");

		if let Some(source_line) = &diagnostic.source_line {
			let gutter_width = span_line_start.to_string().len();
			let caret_prefix = caret_prefix(source_line, span.column_start);
			let caret_count = usize::max(1, span.column_end.saturating_sub(span.column_start));

			let _ = writeln!(writer, " \x1b[2;34m{0:>gutter_width$} |\x1b[0m", "");
			let _ = writeln!(writer, " \x1b[1;34m{span_line_start:>gutter_width$}\x1b[0m \x1b[2;34m|\x1b[0m {source_line}");
			let _ = writeln!(writer, " \x1b[2;34m{0:>gutter_width$} |\x1b[0m {caret_prefix}\x1b[{level_color}m{0:^>caret_count$}\x1b[0m", "");
		}
	}

	if let Some(note) = &diagnostic.note {
		let _ = writeln!(writer, " \x1b[1;32mhelp\x1b[0m\x1b[2m:\x1b[0m {note}");
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn plain_log_carets_preserve_tab_indentation() {
		let entry = LogEntry {
			level: LogLevel::Error,
			span: Some(LineSpan {
				file: "component.vue",
				line_start: 2,
				line_end: 2,
				column_start: 2,
				column_end: 15,
			}),
			message: "bad tag".to_string(),
			note: None,
		};
		let mut log = Logger::with_format(MessageFormat::Json);
		log.log(Some("<div>\n\t<missing-child></missing-child>\n</div>\n"), entry);
		let mut output = Vec::new();

		write_plain_diagnostic(&mut output, &log.diagnostics[0]);

		let rendered = String::from_utf8(output).unwrap();
		assert!(rendered.contains(" --> component.vue:2:3\n"));
		assert!(rendered.contains("2 | \t<missing-child></missing-child>\n"));
		assert!(rendered.contains("  | \t ^^^^^^^^^^^^^\n"));
	}

	#[test]
	fn json_log_is_structured_and_uses_one_indexed_columns() {
		let entry = LogEntry {
			level: LogLevel::Warn,
			span: Some(LineSpan {
				file: "component.vue",
				line_start: 4,
				line_end: 4,
				column_start: 0,
				column_end: 3,
			}),
			message: "test warning".to_string(),
			note: Some("test note"),
		};
		let mut log = Logger::with_format(MessageFormat::Json);
		log.log(Some("a\nb\nc\nbad\n"), entry);
		let mut output = Vec::new();

		write_json(&mut output, &log.diagnostics).unwrap();

		let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
		assert_eq!(value[0]["level"], "warning");
		assert_eq!(value[0]["span"]["file"], "component.vue");
		assert_eq!(value[0]["span"]["column_start"], 1);
		assert_eq!(value[0]["message"], "test warning");
	}
}
