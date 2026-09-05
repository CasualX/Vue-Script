use super::ScriptImport;

fn is_import_statement_start(line: &str) -> bool {
	let Some(suffix) = line.strip_prefix("import") else {
		return false;
	};

	match suffix.chars().next() {
		Some(next) => next.is_ascii_whitespace() || matches!(next, '{' | '*' | '"' | '\''),
		None => false,
	}
}

pub fn get_imports(source: &str, source_line_start: usize) -> (Vec<ScriptImport>, String) {
	let mut imports = Vec::new();
	let mut script = String::new();

	for (line_index, line) in source.split_inclusive('\n').enumerate() {
		let trimmed = line.trim_start();
		if is_import_statement_start(trimmed) {
			let leading_bytes = trimmed.as_ptr() as usize - line.as_ptr() as usize;
			imports.push(ScriptImport {
				text: format!("{}\n", trimmed.trim_end()),
				source_line: source_line_start + line_index,
				source_column_offset: line[..leading_bytes].encode_utf16().count(),
			});
			if line.ends_with('\n') {
				script.push('\n');
			}
		}
		else {
			script.push_str(line);
		}
	}

	(imports, script)
}
