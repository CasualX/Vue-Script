use crate::log;

fn element_span<'a>(file: &'a str, element: &html_parser::Element) -> log::LineSpan<'a> {
	let start = element.source_span.start_column.saturating_sub(1);
	let end = if element.source_span.start_line == element.source_span.end_line {
		usize::max(start + 1, element.source_span.end_column.saturating_sub(1))
	}
	else {
		let first_line_len = element.source_span.text.lines().next().map_or(1, str::len);
		start + usize::max(1, first_line_len)
	};

	log::LineSpan {
		file,
		line: element.source_span.start_line,
		span: start..end,
	}
}

pub struct Component {
	pub path: String,
	pub source: String,
	pub links: Vec<String>,
	pub imports: Vec<String>,
	pub template: Option<String>,
	pub script: Option<String>,
	pub style: Option<String>,
}

impl Component {
	pub fn parse(log: &mut log::Logger, path: &str, text: &str) -> Option<Component> {
		if path.ends_with(".vue") {
			parse_component_vue(log, path, text)
		}
		else if path.ends_with(".vue.js") {
			parse_component_js(path, text)
		}
		else if path.ends_with(".vue.css") {
			parse_component_css(path, text)
		}
		else {
			log.log(None, log::LogEntry {
				level: log::LogLevel::Error,
				span: None,
				message: format!("Unsupported component file type for \"{}\".", path),
				note: Some("Use .vue, .vue.js, or .vue.css files for components."),
			});
			None
		}
	}
}

fn join_component_path(base_path: &str, relative_path: &str) -> Option<String> {
	let mut parts: Vec<&str> = base_path.split('/').filter(|part| !part.is_empty()).collect();

	for part in relative_path.split('/') {
		match part {
			"" | "." => (),
			".." => {
				if parts.pop().is_none() {
					return None;
				}
			},
			_ => parts.push(part),
		}
	}

	Some(parts.join("/"))
}

fn extract_element_inner_text(element: &html_parser::Element) -> Option<String> {
	let mut text = String::new();
	for child in &element.children {
		match child {
			html_parser::Node::Text(text_node) => text.push_str(&text_node),
			html_parser::Node::Element(_) | html_parser::Node::Comment(_) => return None,
		}
	}
	Some(text)
}

fn parse_component_link(log: &mut log::Logger, component_path: &str, source: &str, component_base_path: &str, element: &html_parser::Element) -> Option<Option<String>> {
	let rel = element.attributes.get("rel").and_then(|value| value.as_deref()).unwrap_or("");
	if rel != "component" {
		log.log(Some(source), log::LogEntry {
			level: log::LogLevel::Error,
			span: Some(element_span(component_path, element)),
			message: format!("Top-level <link> in component \"{}\" must use rel=\"component\".", component_path),
			note: Some("Remove the link or change it to <link rel=\"component\" href=\"...\">."),
		});
		return None;
	}

	let Some(href) = element.attributes.get("href").and_then(|value| value.as_deref()) else {
		log.log(Some(source), log::LogEntry {
			level: log::LogLevel::Error,
			span: Some(element_span(component_path, element)),
			message: format!("Component import in \"{}\" is missing an href value.", component_path),
			note: Some("Add href=\"...\" to the component import link."),
		});
		return None;
	};

	let Some(href) = join_component_path(component_base_path, href) else {
		log.log(Some(source), log::LogEntry {
			level: log::LogLevel::Error,
			span: Some(element_span(component_path, element)),
			message: format!("Component import in \"{}\" resolves outside the project root.", component_path),
			note: Some("Use a relative href that stays within the project directory."),
		});
		return None;
	};

	Some(Some(href))
}

fn is_import_statement_start(line: &str) -> bool {
	let Some(suffix) = line.strip_prefix("import") else {
		return false;
	};

	match suffix.chars().next() {
		Some(next) => next.is_ascii_whitespace() || matches!(next, '{' | '*' | '"' | '\''),
		None => false,
	}
}

fn extract_script_imports(source: &str) -> (Vec<String>, String) {
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

fn parse_component_vue(log: &mut log::Logger, component_path: &str, source: &str) -> Option<Component> {
	let component_base_path = component_path.rfind("/").map_or("", |index| &component_path[..index]);

	let path = component_path.to_string();
	let mut links = Vec::new();
	let mut imports = Vec::new();
	let mut script = None;
	let mut template = None;
	let mut style = None;

	let html = match html_parser::Dom::parse(source) {
		Ok(html) => html,
		Err(err) => {
			log.log(None, log::LogEntry {
				level: log::LogLevel::Error,
				span: None,
				message: format!("Failed to parse component \"{}\": {}", component_path, err),
				note: Some("Check the component for invalid HTML fragment syntax."),
			});
			return None;
		},
	};

	if html.tree_type != html_parser::DomVariant::DocumentFragment {
		let span = html.children.iter().find_map(|node| match node {
			html_parser::Node::Element(element) => Some(element_span(component_path, element)),
			_ => None,
		});
		log.log(Some(source), log::LogEntry {
			level: log::LogLevel::Error,
			span,
			message: format!("Component \"{}\" must be an HTML fragment, not a full document.", component_path),
			note: Some("Remove <html>, <head>, <body>, and doctype markup from component files."),
		});
		return None;
	}

	let mut elements = Vec::new();
	for node in &html.children {
		match node {
			html_parser::Node::Element(element) => elements.push(element),
			html_parser::Node::Text(_) | html_parser::Node::Comment(_) => {
				log.log(Some(source), log::LogEntry {
					level: log::LogLevel::Error,
					span: None,
					message: format!("Component \"{}\" has unsupported top-level text or comments.", component_path),
					note: Some("Keep only top-level <link>, <script>, <template>/<div>, and <style> elements."),
				});
				return None;
			},
		}
	}

	let mut index = 0;

	while let Some(element) = elements.get(index)
		&& element.name == "link"
	{
		if let Some(use_path) = parse_component_link(log, component_path, source, component_base_path, element)? {
			links.push(use_path);
		}
		index += 1;
	}

	if let Some(element) = elements.get(index)
		&& element.name == "script"
	{
		let script_contents = match extract_element_inner_text(element) {
			Some(script_contents) => script_contents,
			None => {
				log.log(Some(source), log::LogEntry {
					level: log::LogLevel::Error,
					span: Some(element_span(component_path, element)),
					message: format!("Top-level <script> in component \"{}\" must have an explicit closing tag.", component_path),
					note: Some("Write script blocks as <script>...</script>."),
				});
				return None;
			},
		};
		let (script_imports, script_contents) = extract_script_imports(&script_contents);
		imports = script_imports;
		script = Some(script_contents);
		index += 1;
	}

	if let Some(element) = elements.get(index)
		&& (element.name == "template" || element.name == "div")
	{
		template = Some(element.source_span.text.clone());
		index += 1;
	}

	if let Some(element) = elements.get(index)
		&& element.name == "style"
	{
		style = match extract_element_inner_text(element) {
			Some(style_contents) => Some(style_contents),
			None => {
				log.log(Some(source), log::LogEntry {
					level: log::LogLevel::Error,
					span: Some(element_span(component_path, element)),
					message: format!("Top-level <style> in component \"{}\" must have an explicit closing tag.", component_path),
					note: Some("Write style blocks as <style>...</style>."),
				});
				return None;
			},
		};
		index += 1;
	}

	if index != elements.len() {
		let element = elements[index];
		let (message, note) = match element.name.as_str() {
			"link" => (
				format!("Component imports in \"{}\" must appear before the script, template, and style.", component_path),
				"Move all <link rel=\"component\"> elements to the top of the component.",
			),
			"script" if script.is_some() => (
				format!("Component \"{}\" has more than one top-level <script> element.", component_path),
				"Keep a single top-level <script> block in each component.",
			),
			"script" => (
				format!("Top-level <script> in component \"{}\" must appear before the template and style.", component_path),
				"Place the top-level <script> before the template and style.",
			),
			"template" | "div" if template.is_some() => (
				format!("Component \"{}\" has more than one top-level template root.", component_path),
				"Use a single top-level <template> or <div> element.",
			),
			"template" | "div" => (
				format!("The top-level template in component \"{}\" must appear before the style.", component_path),
				"Place the top-level <template> or <div> before the <style> block.",
			),
			"style" if style.is_some() => (
				format!("Component \"{}\" has more than one top-level <style> element.", component_path),
				"Keep a single top-level <style> block in each component.",
			),
			"style" => (
				format!("Top-level <style> in component \"{}\" must be the last top-level element.", component_path),
				"Place the top-level <style> after the template.",
			),
			name => (
				format!("Component \"{}\" has unsupported top-level <{}> content.", component_path, name),
				"Use only top-level <link>, <script>, <template>/<div>, and <style> elements.",
			),
		};

		log.log(Some(source), log::LogEntry {
			level: log::LogLevel::Error,
			span: Some(element_span(component_path, element)),
			message,
			note: Some(note),
		});
		return None;
	}

	Some(Component {
		path,
		source: source.to_string(),
		links,
		imports,
		template,
		script,
		style,
	})
}

fn parse_component_js(component_path: &str, source: &str) -> Option<Component> {
	let (imports, script) = extract_script_imports(source);
	Some(Component {
		path: component_path.to_string(),
		source: source.to_string(),
		links: Vec::new(),
		imports,
		template: None,
		script: Some(script),
		style: None,
	})
}

fn parse_component_css(component_path: &str, source: &str) -> Option<Component> {
	Some(Component {
		path: component_path.to_string(),
		source: source.to_string(),
		links: Vec::new(),
		imports: Vec::new(),
		template: None,
		script: None,
		style: Some(source.to_string()),
	})
}
