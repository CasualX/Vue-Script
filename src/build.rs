use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::Path;

use super::*;

mod component;
use component::*;

#[derive(Debug)]
pub struct BuildError;

pub type BuildResult = Result<(), BuildError>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LineMapping {
	pub generated_line_start: usize,
	pub generated_line_end: usize,
	pub source_file: String,
	pub source_line_start: usize,
	pub source_column_offset: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MappedJavaScript {
	pub source: String,
	pub mappings: Vec<LineMapping>,
}

impl MappedJavaScript {
	pub fn map_line(&self, generated_line: usize) -> Option<(&str, usize)> {
		let mapping = self.mappings.iter().find(|mapping| {
			generated_line >= mapping.generated_line_start && generated_line <= mapping.generated_line_end
		})?;
		let source_line = mapping.source_line_start + generated_line - mapping.generated_line_start;
		Some((&mapping.source_file, source_line))
	}

	pub fn map_position(&self, generated_line: usize, generated_column: usize) -> Option<(&str, usize, usize)> {
		let mapping = self.mappings.iter().find(|mapping| {
			generated_line >= mapping.generated_line_start && generated_line <= mapping.generated_line_end
		})?;
		let source_line = mapping.source_line_start + generated_line - mapping.generated_line_start;
		Some((&mapping.source_file, source_line, generated_column + mapping.source_column_offset))
	}
}

pub struct Compilation {
	pub config: Config,
	pub javascript: MappedJavaScript,
	pub output: String,
}

struct MappedJavaScriptWriter {
	source: String,
	mappings: Vec<LineMapping>,
}

impl MappedJavaScriptWriter {
	fn new() -> MappedJavaScriptWriter {
		MappedJavaScriptWriter {
			source: String::new(),
			mappings: Vec::new(),
		}
	}

	fn current_line(&self) -> usize {
		self.source.bytes().filter(|byte| *byte == b'\n').count() + 1
	}

	fn push_unmapped(&mut self, source: &str) {
		self.source.push_str(source);
	}

	fn push_mapped(&mut self, source: &str, source_file: &str, source_line_start: usize, source_column_offset: usize) {
		let occupied_lines = source.bytes().filter(|byte| *byte == b'\n').count()
			+ usize::from(!source.is_empty() && !source.ends_with('\n'));
		if occupied_lines == 0 {
			return;
		}

		let generated_line_start = self.current_line();
		self.source.push_str(source);
		self.mappings.push(LineMapping {
			generated_line_start,
			generated_line_end: generated_line_start + occupied_lines - 1,
			source_file: source_file.to_string(),
			source_line_start,
			source_column_offset,
		});
	}

	fn finish(self) -> MappedJavaScript {
		MappedJavaScript {
			source: self.source,
			mappings: self.mappings,
		}
	}
}

fn log_span<'a>(file: &'a str, source: &str, span: tagsoup::SourceSpan) -> log::LineSpan<'a> {
	let resolved = span.resolve(source).unwrap();
	let line_start = resolved.start_line as usize;
	let line_end = resolved.end_line as usize;
	let column_start = resolved.start_column as usize;
	let column_end = resolved.end_column as usize;

	log::LineSpan {
		file,
		line_start,
		line_end,
		column_start,
		column_end,
	}
}

fn find_source_span<'a>(file: &'a str, source: &str, needle: &str) -> Option<log::LineSpan<'a>> {
	let start = source.find(needle)?;
	let prefix = &source[..start];
	let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
	let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
	let column_start = prefix[line_start..].chars().count();
	let width = usize::max(1, needle.chars().count());

	Some(log::LineSpan {
		file,
		line_start: line,
		line_end: line,
		column_start,
		column_end: column_start + width,
	})
}

fn replace(log: &mut log::Logger, file: &str, source: &str, tag: &str, replace: &str) -> String {
	if let Some(index) = source.find(tag) {
		let mut result = String::new();
		result.push_str(&source[..index]);
		result.push_str(replace);
		result.push_str(&source[index + tag.len()..]);
		result
	}
	else {
		log.log(Some(source), log::LogEntry {
			level: log::LogLevel::Error,
			span: find_source_span(file, source, "</body>").or_else(|| find_source_span(file, source, "</head>")),
			message: format!("Page template \"{}\" is missing the required {} placeholder.", file, tag),
			note: Some("Add the required placeholder comment to the page template."),
		});
		source.to_string()
	}
}

fn read_component(log: &mut log::Logger, project_dir: &Path, component_path: &str) -> Option<Component> {
	let full_path = project_dir.join(component_path);
	let source = match fs::read_to_string(&full_path) {
		Ok(source) => source,
		Err(err) => {
			log.log(None, log::LogEntry {
				level: log::LogLevel::Error,
				span: None,
				message: format!("Failed to read component \"{}\": {}", component_path, err),
				note: Some("Check that the component file exists and is readable."),
			});
			return None;
		},
	};

	Component::parse(log, component_path, &source)
}

fn render_templates(components: &[Component]) -> String {
	components.iter().filter_map(|c| c.template.as_ref()).cloned().collect::<Vec<_>>().join("\n")
}

fn render_styles(components: &[Component]) -> String {
	let styles: Vec<_> = components.iter().filter_map(|component| component.style.as_deref()).collect();

	format!("<style>\n{}\n</style>", styles.join("\n"))
}

fn validate_components(log: &mut log::Logger, components: &[Component]) {
	let collection: HashMap<&str, &Component> = components.iter().map(|component| (component.path.as_str(), component)).collect();

	for component in components {
		let direct_imports: HashMap<&str, (&Component, &Link)> = component.component_links()
			.filter_map(|link| collection.get(link.href.as_str()).copied().map(|imported| (link, imported)))
			.filter_map(|(link, imported)| imported.custom_tag.as_deref().map(|tag| (tag, (imported, link))))
			.collect();
		let used_tags: HashSet<&str> = component.used_custom_tags.iter().map(|used| used.tag.as_str()).collect();

		for (imported, link) in direct_imports.values().copied() {
			let Some(custom_tag) = imported.custom_tag.as_deref() else {
				continue;
			};

			if !used_tags.contains(custom_tag) && !link.dynamic {
				log.log(Some(component.source.as_str()), log::LogEntry {
					level: log::LogLevel::Warn,
					span: None,
					message: format!("Component \"{}\" imports \"{}\" but never uses <{}>.", component.path, imported.path, custom_tag),
					note: Some("Remove the unused link import, add the matching custom element, or mark a dynamically referenced import with the dynamic attribute."),
				});
			}
		}

		for used in &component.used_custom_tags {
			if !direct_imports.contains_key(used.tag.as_str()) {
				log.log(Some(component.source.as_str()), log::LogEntry {
					level: log::LogLevel::Error,
					span: Some(log_span(component.path.as_str(), component.source.as_str(), used.span)),
					message: format!("Component \"{}\" uses <{}> without a direct component import.", component.path, used.tag),
					note: Some("Add a matching top-level <link rel=\"component\" href=\"...\"> import for this custom element."),
				});
			}
		}
	}
}

fn render_scripts(log: &mut log::Logger, config: &Config, components: &[Component]) -> MappedJavaScript {
	// Topologically sort components based on used dependencies.
	// import statements are external module imports and are emitted before all script bodies.
	fn visit<'a>(
		log: &mut log::Logger,
		component_path: &str,
		importer: Option<&'a Component>,
		collection: &HashMap<&'a str, &'a Component>,
		visiting: &mut HashSet<&'a str>,
		visited: &mut HashSet<&'a str>,
		ordered_components: &mut Vec<&'a Component>,
	) {
		if visited.contains(component_path) {
			return;
		}

		let Some(component) = collection.get(component_path).copied() else {
			let (file_contents, span, message) = match importer {
				Some(importer) => (
					Some(importer.source.as_str()),
					find_source_span(importer.path.as_str(), importer.source.as_str(), component_path),
					format!("Component \"{}\" imports missing component \"{}\".", importer.path, component_path),
				),
				None => (None, None, format!("Missing root component \"{}\".", component_path)),
			};

			log.log(file_contents, log::LogEntry {
				level: log::LogLevel::Error,
				span,
				message,
				note: Some("Check the href path in the component import link."),
			});
			return;
		};

		if visiting.contains(component.path.as_str()) {
			let (file_contents, span, message) = match importer {
				Some(importer) => (
					Some(importer.source.as_str()),
					find_source_span(importer.path.as_str(), importer.source.as_str(), component_path),
					format!("Component import cycle detected: \"{}\" recursively depends on \"{}\".", importer.path, component_path),
				),
				None => (
					Some(component.source.as_str()),
					None,
					format!("Component import cycle detected at \"{}\".", component.path),
				),
			};

			log.log(file_contents, log::LogEntry {
				level: log::LogLevel::Error,
				span,
				message,
				note: Some("Remove the cycle so component imports form a directed acyclic graph."),
			});
			return;
		}

		visiting.insert(component.path.as_str());
		for link in component.component_links() {
			visit(log, &link.href, Some(component), collection, visiting, visited, ordered_components);
		}
		visiting.remove(component.path.as_str());
		visited.insert(component.path.as_str());
		ordered_components.push(component);
	}

	let collection: HashMap<&str, &Component> = components.iter().map(|component| (component.path.as_str(), component)).collect();
	let mut visiting = HashSet::new();
	let mut visited = HashSet::new();
	let mut ordered_components = Vec::new();
	visit(log, &config.app.main, None, &collection, &mut visiting, &mut visited, &mut ordered_components);

	let mut ordered_imports: Vec<_> = ordered_components.iter()
		.flat_map(|component| component.imports.iter().map(move |import| (*component, import)))
		.collect();
	ordered_imports.sort_by(|left, right| left.1.text.cmp(&right.1.text));
	ordered_imports.dedup_by(|left, right| left.1.text == right.1.text);

	let mut writer = MappedJavaScriptWriter::new();
	for (component, import) in ordered_imports {
		writer.push_mapped(&import.text, &component.path, import.source_line, import.source_column_offset);
	}
	writer.push_unmapped("\n");

	let mut first_script = true;
	for component in ordered_components {
		let (Some(script), Some(source_line)) = (component.script.as_deref(), component.script_source_line) else {
			continue;
		};
		if !first_script {
			writer.push_unmapped("\n");
		}
		first_script = false;
		writer.push_mapped(script, &component.path, source_line, 0);
	}

	writer.finish()
}

fn collect_components(log: &mut log::Logger, project_path: &Path, main_component_path: &str) -> Vec<Component> {
	let mut components = Vec::new();
	let mut visited = HashMap::new();
	let mut to_visit = BTreeSet::new();
	to_visit.insert(main_component_path.to_string());

	while let Some(component_path) = to_visit.iter().next().cloned() {
		to_visit.remove(&component_path);
		if visited.contains_key(&component_path) {
			continue;
		}
		visited.insert(component_path.clone(), ());

		let component = match read_component(log, project_path, &component_path) {
			Some(component) => component,
			None => continue,
		};

		for link in component.component_links() {
			to_visit.insert(link.href.clone());
		}

		components.push(component);
	}

	components
}

pub fn compile(log: &mut log::Logger) -> Result<Compilation, BuildError> {
	log.reset();
	let config = match Config::load(log) {
		Ok(config) => config,
		Err(err) => {
			log.log(None, log::LogEntry {
				level: log::LogLevel::Error,
				span: None,
				message: format!("Failed to load configuration: {}", err),
				note: Some("Check that the configuration file exists and is valid TOML."),
			});
			return Err(BuildError);
		},
	};

	let project_path = config.path.parent().unwrap();
	let components = collect_components(log, project_path, &config.app.main);
	validate_components(log, &components);

	let javascript = render_scripts(log, &config, &components);
	let scripts = format!("<script type=\"module\">\n{}\n</script>", javascript.source);
	let styles = render_styles(&components);
	let templates = render_templates(&components);

	let output = match fs::read_to_string(&project_path.join(&config.app.page)) {
		Ok(source) => {
			let source = replace(log, &config.app.page, &source, "<!-- SCRIPTS -->", &scripts);
			let source = replace(log, &config.app.page, &source, "<!-- STYLES -->", &styles);
			replace(log, &config.app.page, &source, "<!-- TEMPLATES -->", &templates)
		},
		Err(err) => {
			log.log(None, log::LogEntry {
				level: log::LogLevel::Error,
				span: None,
				message: format!("Failed to read app page \"{}\": {}", config.app.page, err),
				note: Some("Check that the page file exists and is readable."),
			});
			return Err(BuildError);
		}
	};

	Ok(Compilation { config, javascript, output })
}

pub fn main(log: &mut log::Logger) -> BuildResult {
	let compilation = compile(log)?;
	if log.has_errors() {
		return Err(BuildError);
	}

	let project_path = compilation.config.path.parent().unwrap();
	if let Some(target_path) = &compilation.config.target.path {
		let target_full_path = project_path.join(target_path);
		match fs::write(&target_full_path, &compilation.output) {
			Ok(()) => log.log(None, log::LogEntry {
				level: log::LogLevel::Info,
				span: None,
				message: format!("Wrote \"{}\".", target_full_path.display()),
				note: None,
			}),
			Err(err) => {
				log.log(None, log::LogEntry {
					level: log::LogLevel::Error,
					span: None,
					message: format!("Failed to write \"{}\": {}", target_full_path.display(), err),
					note: Some("Check that the target path exists and is writable."),
				});
				return Err(BuildError);
			},
		}
	}
	else {
		println!("{}", compilation.output);
	}

	if log.has_errors() { Err(BuildError) } else { Ok(()) }
}

#[cfg(test)]
mod tests;
