use std::io::{self, IsTerminal, Write};
use std::ops;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum LogLevel {
	Error,
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
	pub line: usize,
	pub span: ops::Range<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogEntry<'a> {
	pub level: LogLevel,
	pub span: Option<LineSpan<'a>>,
	pub message: String,
	pub note: Option<&'a str>,
}


pub struct Logger {
	errors: usize,
	warns: usize,
	infos: usize,
}

impl Logger {
	pub fn new() -> Logger {
		Logger {
			errors: 0,
			warns: 0,
			infos: 0,
		}
	}

	pub fn has_errors(&self) -> bool {
		self.errors > 0
	}

	pub fn log(&mut self, source: Option<&str>, entry: LogEntry<'_>) {
		match entry.level {
			LogLevel::Error => self.errors += 1,
			LogLevel::Warn => self.warns += 1,
			LogLevel::Info => self.infos += 1,
		}

		let stderr = io::stderr();
		let mut stderr = stderr.lock();

		let write_fn = if stderr.is_terminal() { write_colored_entry } else { write_plain_entry };
		write_fn(&mut stderr, source, &entry);
	}

	pub fn finished(&self) -> bool {
		let errors = self.errors;
		let warns = self.warns;
		let stderr = io::stderr();
		let mut stderr = stderr.lock();
		let _ = writeln!(stderr, "Finished with {errors} error(s), {warns} warning(s).");
		errors == 0
	}
}

fn get_source_line<'a>(source: Option<&'a str>, span: &LineSpan) -> Option<&'a str> {
	source.and_then(|source| source.lines().nth(span.line.saturating_sub(1)))
}

fn write_plain_entry(mut writer: impl Write, source: Option<&str>, entry: &LogEntry<'_>) {
	let level_str = entry.level.as_str();
	let message = &entry.message;
	let _ = writeln!(writer, "{level_str}: {message}");

	if let Some(span) = &entry.span {
		let span_file = span.file;
		let span_line = span.line;
		let location_column = span.span.start.saturating_add(1);
		let _ = writeln!(writer, " --> {span_file}:{span_line}:{location_column}");

		if let Some(source_line) = get_source_line(source, span) {
			let gutter_width = span_line.to_string().len();
			let caret_padding = span.span.start;
			let caret_count = usize::max(1, span.span.end.saturating_sub(span.span.start));
			let blank = "";

			let _ = writeln!(writer, "{0:>gutter_width$} |", blank);
			let _ = writeln!(writer, "{span_line:>gutter_width$} | {source_line}");
			let _ = writeln!(writer, "{0:>gutter_width$} | {0:>caret_padding$}{0:^>caret_count$}", blank);
		}
	}

	if let Some(note) = entry.note {
		let _ = writeln!(writer, " help: {}", note);
	}
}

fn write_colored_entry(mut writer: impl Write, source: Option<&str>, entry: &LogEntry<'_>) {
	let level_color = entry.level.ansi_color();
	let level_str = entry.level.as_str();
	let message = &entry.message;
	let _ = writeln!(writer, "\x1b[{level_color}m{level_str}\x1b[0m\x1b[1m: {message}\x1b[0m");

	if let Some(span) = &entry.span {
		let span_file = span.file;
		let span_line = span.line;
		let location_column = span.span.start.saturating_add(1);
		let _ = writeln!(writer, " \x1b[1;34m-->\x1b[0m {span_file}:{span_line}:{location_column}");

		if let Some(source_line) = get_source_line(source, span) {
			let gutter_width = span_line.to_string().len();
			let caret_padding = span.span.start;
			let caret_count = usize::max(1, span.span.end.saturating_sub(span.span.start));

			let _ = writeln!(writer, " \x1b[2;34m{0:>gutter_width$} |\x1b[0m", "");
			let _ = writeln!(writer, " \x1b[1;34m{span_line:>gutter_width$}\x1b[0m \x1b[2;34m|\x1b[0m {source_line}");
			let _ = writeln!(writer, " \x1b[2;34m{0:>gutter_width$} |\x1b[0m {0:>caret_padding$}\x1b[{level_color}m{0:^>caret_count$}\x1b[0m", "");
		}
	}

	if let Some(note) = entry.note {
		let _ = writeln!(writer, " \x1b[1;32mhelp\x1b[0m\x1b[2m:\x1b[0m {note}");
	}
}
