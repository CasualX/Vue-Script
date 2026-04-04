use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::Path;

use super::*;

mod component;
use component::Component;

fn find_source_span<'a>(file: &'a str, source: &str, needle: &str) -> Option<log::LineSpan<'a>> {
	let start = source.find(needle)?;
	let prefix = &source[..start];
	let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
	let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
	let column_start = prefix[line_start..].chars().count();
	let width = usize::max(1, needle.chars().count());

	Some(log::LineSpan {
		file,
		line,
		span: column_start..(column_start + width),
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

fn render_scripts(log: &mut log::Logger, config: &Config, components: &[Component]) -> String {
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
		for used_path in &component.links {
			visit(log, used_path, Some(component), collection, visiting, visited, ordered_components);
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

	let mut ordered_imports: Vec<_> = ordered_components.iter().flat_map(|component| component.imports.iter().map(String::as_str)).collect();
	ordered_imports.sort();
	ordered_imports.dedup();

	let ordered_scripts: Vec<_> = ordered_components.iter().filter_map(|component| component.script.as_deref()).collect();

	format!("<script type=\"module\">\n{}\n{}\n</script>", ordered_imports.join(""), ordered_scripts.join("\n"))
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

		for import in &component.links {
			to_visit.insert(import.clone());
		}

		components.push(component);
	}

	components
}

pub fn main(log: &mut log::Logger) {
	let ref config = match Config::load(log) {
		Ok(config) => config,
		Err(err) => {
			log.log(None, log::LogEntry {
				level: log::LogLevel::Error,
				span: None,
				message: format!("Failed to load configuration: {}", err),
				note: Some("Check that the configuration file exists and is valid TOML."),
			});
			return;
		},
	};

	let project_path = config.path.parent().unwrap();
	let components = collect_components(log, project_path, &config.app.main);

	let scripts = render_scripts(log, config, &components);
	let styles = render_styles(&components);
	let templates = render_templates(&components);

	match fs::read_to_string(&project_path.join(&config.app.page)) {
		Ok(source) => {
			let source = replace(log, &config.app.page, &source, "<!-- SCRIPTS -->", &scripts);
			let source = replace(log, &config.app.page, &source, "<!-- STYLES -->", &styles);
			let source = replace(log, &config.app.page, &source, "<!-- TEMPLATES -->", &templates);

			if !log.has_errors() {
				if let Some(target_path) = &config.target.path {
					let target_full_path = project_path.join(target_path);
					match fs::write(&target_full_path, &source) {
						Ok(()) => log.log(None, log::LogEntry {
							level: log::LogLevel::Info,
							span: None,
							message: format!("Wrote \"{}\".", target_full_path.display()),
							note: None,
						}),
						Err(err) => log.log(None, log::LogEntry {
							level: log::LogLevel::Error,
							span: None,
							message: format!("Failed to write \"{}\": {}", target_full_path.display(), err),
							note: Some("Check that the target path exists and is writable."),
						}),
					}
				}
				else {
					println!("{}", source);
				}
			}
		},
		Err(err) => {
			log.log(None, log::LogEntry {
				level: log::LogLevel::Error,
				span: None,
				message: format!("Failed to read app page \"{}\": {}", config.app.page, err),
				note: Some("Check that the page file exists and is readable."),
			});
		}
	}
}

#[cfg(test)]
mod tests;
