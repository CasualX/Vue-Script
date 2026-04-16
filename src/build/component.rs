use crate::log;

mod vue;
mod js;

fn outer_html<'a>(source: &'a str, span: tagsoup::Span) -> &'a str {
	&source[span.start as usize..span.end as usize]
}

fn log_span<'a>(file: &'a str, source: &str, span: tagsoup::Span) -> log::LineSpan<'a> {
	let resolved = span.resolve(source);
	let line_start = resolved.start_line as usize;
	let line_end = resolved.end_line as usize;
	let column_start = resolved.start_column.saturating_sub(1) as usize;
	let column_end = resolved.end_column.saturating_sub(1) as usize;

	log::LineSpan {
		file,
		line_start,
		line_end,
		column_start,
		column_end,
	}
}

#[derive(Debug)]
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
