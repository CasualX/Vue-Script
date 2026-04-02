use std::fs;
use std::collections::HashMap;
use std::path::Path;

mod component;

use crate::config::Config;
use component::Component;

fn replace(contents: &str, tag: &str, replace: &str) -> String {
	if let Some(index) = contents.find(tag) {
		let mut result = String::new();
		result.push_str(&contents[..index]);
		result.push_str(replace);
		result.push_str(&contents[index + tag.len()..]);
		result
	}
	else {
		eprintln!("Failed to replace \"{}\" because the tag wasn't found.", tag);
		contents.to_string()
	}
}

fn read_component(project_dir: &Path, component_path: &str) -> Option<Component> {
	let full_path = project_dir.join(component_path);
	let contents = match fs::read_to_string(&full_path) {
		Ok(contents) => contents,
		Err(err) => {
			eprintln!("warn: Failed to read_to_string(\"{}\"): {}", full_path.display(), err);
			return None;
		},
	};

	Component::parse(component_path, &contents)
}

fn render_templates(components: &[Component]) -> String {
	components.iter().filter_map(|c| c.template.as_ref()).cloned().collect::<Vec<_>>().join("\n")
}

fn render_styles(components: &[Component]) -> String {
	let styles: Vec<_> = components.iter().filter_map(|component| component.style.as_deref()).collect();

	format!("<style>\n{}\n</style>", styles.join("\n"))
}

fn render_scripts(config: &Config, components: &[Component]) -> String {
	// Topologically sort components based on used dependencies.
	// import statements are external module imports and are emitted before all script bodies.
	fn visit<'a>(
		component_path: &str,
		collection: &HashMap<&'a str, &'a Component>,
		visiting: &mut HashMap<&'a str, ()>,
		visited: &mut HashMap<&'a str, ()>,
		ordered_components: &mut Vec<&'a Component>,
	) {
		if visited.contains_key(component_path) {
			return;
		}

		let Some(component) = collection.get(component_path).copied() else {
			eprintln!("warn: Missing imported component \"{}\" while rendering scripts.", component_path);
			return;
		};

		if visiting.contains_key(component.path.as_str()) {
			eprintln!("warn: Cyclic component import involving \"{}\".", component.path);
			return;
		}

		visiting.insert(component.path.as_str(), ());
		for used_path in &component.uses {
			visit(used_path, collection, visiting, visited, ordered_components);
		}
		visiting.remove(component.path.as_str());
		visited.insert(component.path.as_str(), ());
		ordered_components.push(component);
	}

	let collection: HashMap<&str, &Component> = components.iter().map(|component| (component.path.as_str(), component)).collect();
	let mut visiting = HashMap::new();
	let mut visited = HashMap::new();
	let mut ordered_components = Vec::new();
	visit(&config.app.main, &collection, &mut visiting, &mut visited, &mut ordered_components);

	let mut ordered_imports: Vec<_> = ordered_components.iter().flat_map(|component| component.imports.iter().map(String::as_str)).collect();
	ordered_imports.sort();
	ordered_imports.dedup();

	let ordered_scripts: Vec<_> = ordered_components.iter().filter_map(|component| component.script.as_deref()).collect();

	format!("<script type=\"module\">\n{}\n{}\n</script>", ordered_imports.join(""), ordered_scripts.join("\n"))
}

pub fn main(config: &Config) {
	let project_path = config.path.parent().unwrap();

	let mut components = Vec::new();

	let mut visited = HashMap::new();
	let mut to_visit = HashMap::new();
	to_visit.insert(config.app.main.clone(), ());
	while !to_visit.is_empty() {
		let component_path = to_visit.keys().next().unwrap().to_string();
		to_visit.remove(&component_path);
		if visited.contains_key(&component_path) {
			continue;
		}
		visited.insert(component_path.clone(), ());

		let component = match read_component(project_path, &component_path) {
			Some(component) => component,
			None => continue,
		};

		for import in &component.uses {
			to_visit.insert(import.clone(), ());
		}

		components.push(component);
	}

	let scripts = render_scripts(config, &components);
	let styles = render_styles(&components);
	let templates = render_templates(&components);

	match fs::read_to_string(&project_path.join(&config.app.page)) {
		Ok(contents) => {
			let contents = replace(&contents, "<!-- SCRIPTS -->", &scripts);
			let contents = replace(&contents, "<!-- STYLES -->", &styles);
			let contents = replace(&contents, "<!-- TEMPLATES -->", &templates);

			if let Some(target_path) = &config.target.path {
				let target_full_path = project_path.join(target_path);
				match fs::write(&target_full_path, &contents) {
					Ok(()) => eprintln!("Successfully wrote to \"{}\".", target_full_path.display()),
					Err(err) => eprintln!("error: Failed to write to \"{}\": {}", target_full_path.display(), err),
				}
			}
			else {
				println!("{}", contents);
			}
		},
		Err(err) => {
			eprintln!("warn: Failed to read_to_string(\"{}\"): {}", config.app.page, err);
		}
	}
}

#[cfg(test)]
mod tests;
