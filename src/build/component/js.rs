
fn is_import_statement_start(line: &str) -> bool {
	let Some(suffix) = line.strip_prefix("import") else {
		return false;
	};

	match suffix.chars().next() {
		Some(next) => next.is_ascii_whitespace() || matches!(next, '{' | '*' | '"' | '\''),
		None => false,
	}
}

pub fn get_imports(source: &str) -> (Vec<String>, String) {
	let mut imports = Vec::new();
	let mut script = String::new();

	for line in source.split_inclusive('\n') {
		let trimmed = line.trim_start();
		if is_import_statement_start(trimmed) {
			imports.push(format!("{}\n", trimmed.trim_end()));
		}
		else {
			script.push_str(line);
		}
	}

	if !source.ends_with('\n') {
		let trailing_line = source.rsplit_once('\n').map_or(source, |(_, line)| line);
		if is_import_statement_start(trailing_line.trim_start()) && script.ends_with('\n') {
			script.pop();
		}
	}

	(imports, script)
}
