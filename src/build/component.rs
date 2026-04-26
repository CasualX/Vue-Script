use crate::log;
use super::*;

mod vue;
mod js;

#[derive(Debug, Clone)]
pub struct UsedCustomTag {
	pub tag: String,
	pub span: tagsoup::SourceSpan,
}

fn outer_html<'a>(source: &'a str, span: tagsoup::SourceSpan) -> &'a str {
	&source[span.start as usize..span.end as usize]
}

#[derive(Debug)]
pub struct Component {
	pub path: String,
	pub source: String,
	pub links: Vec<String>,
	pub imports: Vec<String>,
	pub custom_tag: Option<String>,
	pub used_custom_tags: Vec<UsedCustomTag>,
	pub template: Option<String>,
	pub script: Option<String>,
	pub style: Option<String>,
}

impl Component {
	pub fn parse(log: &mut log::Logger, path: &str, text: &str) -> Option<Component> {
		if path.ends_with(".vue") {
			vue::parse(log, path, text)
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

fn parse_component_js(component_path: &str, source: &str) -> Option<Component> {
	let (imports, script) = js::get_imports(source);
	Some(Component {
		path: component_path.to_string(),
		source: source.to_string(),
		links: Vec::new(),
		imports,
		custom_tag: None,
		used_custom_tags: Vec::new(),
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
		custom_tag: None,
		used_custom_tags: Vec::new(),
		template: None,
		script: None,
		style: Some(source.to_string()),
	})
}
