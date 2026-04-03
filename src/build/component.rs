pub struct Component {
	pub path: String,
	pub uses: Vec<String>,
	pub imports: Vec<String>,
	pub template: Option<String>,
	pub script: Option<String>,
	pub style: Option<String>,
}

impl Component {
	pub fn parse(path: &str, text: &str) -> Option<Component> {
		if path.ends_with(".vue") {
			parse_component_vue(path, text)
		}
		else if path.ends_with(".vue.js") {
			parse_component_js(path, text)
		}
		else if path.ends_with(".vue.css") {
			parse_component_css(path, text)
		}
		else {
			eprintln!("warn: Unsupported component file type for \"{}\".", path);
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

fn warn_invalid_vue_component<T>(component_path: &str, message: &str) -> Option<T> {
	eprintln!("warn: Invalid Vue component \"{}\": {}", component_path, message);
	None
}

fn warn_ignored_component_link(component_path: &str, rel: &str) {
	eprintln!("warn: Ignoring link rel=\"{}\" in component \"{}\"; only rel=\"component\" is supported.", rel, component_path);
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

fn parse_component_link(component_path: &str, component_base_path: &str, element: &html_parser::Element) -> Option<Option<String>> {
	let rel = element.attributes.get("rel").and_then(|value| value.as_deref()).unwrap_or("");
	if rel != "component" {
		warn_ignored_component_link(component_path, rel);
		return Some(None);
	}

	let Some(href) = element.attributes.get("href").and_then(|value| value.as_deref()) else {
		return warn_invalid_vue_component(component_path, "component link href attribute must have a value");
	};

	let Some(href) = join_component_path(component_base_path, href) else {
		return warn_invalid_vue_component(component_path, "component link href must not walk above the project root");
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

fn extract_script_imports(contents: &str) -> (Vec<String>, String) {
	let mut imports = Vec::new();
	let mut script = String::new();

	for line in contents.split_inclusive('\n') {
		let trimmed = line.trim_start();
		if is_import_statement_start(trimmed) {
			imports.push(format!("{}\n", trimmed.trim_end()));
		}
		else {
			script.push_str(line);
		}
	}

	if !contents.ends_with('\n') {
		let trailing_line = contents.rsplit_once('\n').map_or(contents, |(_, line)| line);
		if is_import_statement_start(trailing_line.trim_start()) && script.ends_with('\n') {
			script.pop();
		}
	}

	(imports, script)
}

fn parse_component_vue(component_path: &str, contents: &str) -> Option<Component> {
	let component_base_path = component_path.rfind("/").map_or("", |index| &component_path[..index]);

	let path = component_path.to_string();
	let mut uses = Vec::new();
	let mut imports = Vec::new();
	let mut script = None;
	let mut template = None;
	let mut style = None;

	let html = match html_parser::Dom::parse(contents) {
		Ok(html) => html,
		Err(err) => {
			eprintln!("warn: Failed to parse component \"{}\": {}", component_path, err);
			return None;
		},
	};

	if html.tree_type != html_parser::DomVariant::DocumentFragment {
		return warn_invalid_vue_component(component_path, "top level must be an HTML document fragment");
	}

	let mut elements = Vec::new();
	for node in &html.children {
		match node {
			html_parser::Node::Element(element) => elements.push(element),
			html_parser::Node::Text(_) | html_parser::Node::Comment(_) => {
				return warn_invalid_vue_component(component_path, "top level may only contain link, script, template or div, and style elements");
			},
		}
	}

	let mut index = 0;

	while let Some(element) = elements.get(index)
		&& element.name == "link"
	{
		if let Some(use_path) = parse_component_link(component_path, component_base_path, element)? {
			uses.push(use_path);
		}
		index += 1;
	}

	if let Some(element) = elements.get(index)
		&& element.name == "script"
	{
		let script_contents = match extract_element_inner_text(element) {
			Some(script_contents) => script_contents,
			None => return warn_invalid_vue_component(component_path, "script tag must have an explicit closing tag"),
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
			None => return warn_invalid_vue_component(component_path, "style tag must have an explicit closing tag"),
		};
		index += 1;
	}

	if index != elements.len() {
		return warn_invalid_vue_component(component_path, "top level must be ordered as zero or more link elements, optional script, optional template or div, then optional style");
	}

	Some(Component { path, uses, imports, template, script, style })
}

fn parse_component_js(component_path: &str, contents: &str) -> Option<Component> {
	let (imports, script) = extract_script_imports(contents);
	Some(Component { path: component_path.to_string(), uses: Vec::new(), imports, template: None, script: Some(script), style: None })
}

fn parse_component_css(component_path: &str, contents: &str) -> Option<Component> {
	Some(Component { path: component_path.to_string(), uses: Vec::new(), imports: Vec::new(), template: None, script: None, style: Some(contents.to_string()) })
}
