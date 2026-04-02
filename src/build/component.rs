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

fn join_component_path(base_path: &str, relative_path: &str) -> String {
	if base_path.is_empty() {
		relative_path.to_string()
	}
	else {
		format!("{}/{}", base_path, relative_path)
	}
}

fn warn_invalid_vue_component(component_path: &str, message: &str) -> Option<Component> {
	eprintln!("warn: Invalid Vue component \"{}\": {}", component_path, message);
	None
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
				return warn_invalid_vue_component(component_path, "top level may only contain script, template or div, and style elements");
			},
		}
	}

	let mut index = 0;

	if let Some(element) = elements.get(index)
		&& element.name == "script"
	{
		for (attribute, value) in &element.attributes {
			match attribute.as_str() {
				"use" => {
					let Some(import_path) = value.as_deref() else {
						return warn_invalid_vue_component(component_path, "script use attribute must have a value");
					};
					uses.push(join_component_path(component_base_path, import_path));
				},
				"import" => {
					let Some(import_statement) = value.as_deref() else {
						return warn_invalid_vue_component(component_path, "script import attribute must have a value");
					};
					imports.push(format!("import {};\n", import_statement));
				},
				_ => (),
			}
		}

		script = match extract_element_inner_text(element) {
			Some(script_contents) => Some(script_contents),
			None => return warn_invalid_vue_component(component_path, "script tag must have an explicit closing tag"),
		};
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
		return warn_invalid_vue_component(component_path, "top level must be ordered as optional script, optional template or div, then optional style");
	}

	Some(Component { path, uses, imports, template, script, style })
}

fn parse_component_js(component_path: &str, contents: &str) -> Option<Component> {
	Some(Component { path: component_path.to_string(), uses: Vec::new(), imports: Vec::new(), template: None, script: Some(contents.to_string()), style: None })
}

fn parse_component_css(component_path: &str, contents: &str) -> Option<Component> {
	Some(Component { path: component_path.to_string(), uses: Vec::new(), imports: Vec::new(), template: None, script: None, style: Some(contents.to_string()) })
}
