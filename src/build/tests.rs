use super::*;

// Keep build fixtures under src/build/tests so they are visible in the repository.
// Parse-only tests can use include_str!, while filesystem traversal tests should add
// dedicated fixture files here and resolve them from CARGO_MANIFEST_DIR.

fn parse_component(path: &str, text: &str) -> Option<Component> {
	let mut log = crate::log::Logger::new();
	Component::parse(&mut log, path, text)
}

#[test]
fn collects_components_in_stable_sorted_order() {
	let project_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

	let mut log = crate::log::Logger::new();
	let components = collect_components(
		&mut log,
		&project_dir,
		"src/build/tests/collects_components_in_stable_sorted_order_main.vue",
	);
	let paths: Vec<_> = components.iter().map(|component| component.path.as_str()).collect();

	assert_eq!(
		paths,
		vec![
			"src/build/tests/collects_components_in_stable_sorted_order_main.vue",
			"src/build/tests/collects_components_in_stable_sorted_order_a_first.vue",
			"src/build/tests/collects_components_in_stable_sorted_order_z_last.vue",
		]
	);
	assert!(log.finished(), "component collection should not emit errors");
}

#[test]
fn build_reports_cycle_and_missing_import_without_filesystem() {
	let main_component = parse_component(
		"src/build/tests/build_reports_cycle_and_missing_import_main.vue",
		include_str!("tests/build_reports_cycle_and_missing_import_main.vue"),
	)
	.expect("main component should parse");
	let child_component = parse_component(
		"src/build/tests/build_reports_cycle_and_missing_import_child.vue",
		include_str!("tests/build_reports_cycle_and_missing_import_child.vue"),
	)
	.expect("child component should parse");
	let config = crate::Config {
		path: std::path::PathBuf::from("vue-script.toml"),
		app: crate::config::ConfigApp {
			page: "app/page.html".to_string(),
			main: "src/build/tests/build_reports_cycle_and_missing_import_main.vue".to_string(),
		},
		target: crate::config::ConfigTarget {
			path: None,
		},
		serve: crate::config::ConfigServe::default(),
	};
	let mut log = crate::log::Logger::new();
	let output = render_scripts(&mut log, &config, &[main_component, child_component]);

	assert!(!output.is_empty(), "rendered script output should still be produced for inspection");
	assert!(!log.finished(), "cycles and missing imports should be reported as errors");
}

#[test]
fn build_reports_missing_page_placeholders_without_filesystem() {
	let page = "<html>\n<head></head>\n<body>\n<div>No placeholders here</div>\n</body>\n</html>\n";
	let mut log = crate::log::Logger::new();
	let with_scripts = replace(&mut log, "app/page.html", page, "<!-- SCRIPTS -->", "<script />");
	let with_styles = replace(&mut log, "app/page.html", &with_scripts, "<!-- STYLES -->", "<style />");
	let with_templates = replace(&mut log, "app/page.html", &with_styles, "<!-- TEMPLATES -->", "<div />");

	assert_eq!(with_templates, page, "page contents should be unchanged when placeholders are missing");
	assert!(!log.finished(), "missing placeholders should be reported as errors");
}

#[test]
fn parses_valid_vue_fragment() {
	let component = parse_component(
		"src/build/tests/parses_valid_vue_fragment.vue",
		include_str!("tests/parses_valid_vue_fragment.vue"),
	)
	.expect("component should parse");

	assert_eq!(component.path, "src/build/tests/parses_valid_vue_fragment.vue");
	assert_eq!(component.links, vec!["src/build/tests/components/child.vue"]);
	assert_eq!(component.imports, vec!["import { createApp } from 'vue';\n"]);
	assert!(component.script.as_deref().unwrap().contains("console.log"));
	assert!(!component.script.as_deref().unwrap().contains("import { createApp } from 'vue';"));
	assert!(component.template.as_deref().unwrap().starts_with("<template>"));
	assert!(component.style.as_deref().unwrap().contains("div { color: red; }"));
	assert!(!component.style.as_deref().unwrap().contains("<style>"));
}

#[test]
fn normalizes_component_use_paths() {
	let component = parse_component(
		"src/build/tests/normalizes_component_use_paths.vue",
		include_str!("tests/normalizes_component_use_paths.vue"),
	)
	.expect("component should parse");

	assert_eq!(component.links, vec!["src/build/child.vue"]);
}

#[test]
fn extracts_multiple_import_lines_from_script_contents() {
	let component = parse_component(
		"src/build/tests/extracts_multiple_import_lines_from_script_contents.vue",
		include_str!("tests/extracts_multiple_import_lines_from_script_contents.vue"),
	)
	.expect("component should parse");

	assert_eq!(component.links, vec!["src/build/tests/components/child.vue", "src/build/tests/components/sibling.vue"]);
	assert_eq!(component.imports, vec!["import { createApp } from 'vue';\n", "import helper from './helper.js';\n"]);
	assert_eq!(component.script.as_deref().unwrap().trim(), "console.log(\"ok\");");
}

#[test]
fn rejects_component_use_paths_above_root() {
	assert!(parse_component(
		"src/build/tests/rejects_component_use_paths_above_root.vue",
		include_str!("tests/rejects_component_use_paths_above_root.vue"),
	)
	.is_none());
}

#[test]
fn rejects_non_component_links() {
	assert!(parse_component(
		"src/build/tests/ignores_non_component_links.vue",
		include_str!("tests/ignores_non_component_links.vue"),
	)
	.is_none());
}

#[test]
fn rejects_component_links_without_href() {
	assert!(parse_component(
		"src/build/tests/rejects_component_links_without_href.vue",
		include_str!("tests/rejects_component_links_without_href.vue"),
	)
	.is_none());
}

#[test]
fn extracts_import_lines_from_vue_js_helpers() {
	let component = parse_component(
		"src/build/tests/extracts_import_lines_from_vue_js_helpers.vue.js",
		include_str!("tests/extracts_import_lines_from_vue_js_helpers.vue.js"),
	)
	.expect("helper should parse");

	assert_eq!(component.imports, vec!["import helper from './helper.js';\n"]);
	assert_eq!(component.script.as_deref().unwrap().trim(), "const answer = 42;");
}

#[test]
fn ignores_non_statement_import_prefixes() {
	let component = parse_component(
		"src/build/tests/ignores_non_statement_import_prefixes.vue.js",
		include_str!("tests/ignores_non_statement_import_prefixes.vue.js"),
	)
	.expect("helper should parse");

	assert_eq!(component.imports, vec!["import{ named } from './helper.js';\n"]);
	assert!(component.script.as_deref().unwrap().contains("importedAt = Date.now();"));
	assert!(component.script.as_deref().unwrap().contains("import.meta.env;"));
	assert!(component.script.as_deref().unwrap().contains("const value = import('helper');"));
}

#[test]
fn rejects_multiple_script_tags() {
	assert!(parse_component(
		"src/build/tests/rejects_multiple_script_tags.vue",
		include_str!("tests/rejects_multiple_script_tags.vue"),
	)
	.is_none());
}

#[test]
fn rejects_template_and_div_together() {
	assert!(parse_component(
		"src/build/tests/rejects_template_and_div_together.vue",
		include_str!("tests/rejects_template_and_div_together.vue"),
	)
	.is_none());
}

#[test]
fn rejects_full_document() {
	assert!(parse_component(
		"src/build/tests/rejects_full_document.vue",
		include_str!("tests/rejects_full_document.vue"),
	)
	.is_none());
}

#[test]
fn renders_styles_in_single_tag() {
	let components = vec![
		Component {
			path: "app/one.vue".to_string(),
			source: String::new(),
			links: Vec::new(),
			imports: Vec::new(),
			template: None,
			script: None,
			style: Some(".one { color: red; }".to_string()),
		},
		Component {
			path: "app/two.vue.css".to_string(),
			source: String::new(),
			links: Vec::new(),
			imports: Vec::new(),
			template: None,
			script: None,
			style: Some(".two { color: blue; }".to_string()),
		},
	];

	assert_eq!(
		render_styles(&components),
		"<style>\n.one { color: red; }\n.two { color: blue; }\n</style>"
	);
}
