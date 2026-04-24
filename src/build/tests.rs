use super::*;

// Keep build fixtures under src/build/tests so they are visible in the repository.
// Parse-only tests can use include_str!, while filesystem traversal tests should add
// dedicated fixture files here and resolve them from CARGO_MANIFEST_DIR.

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
	let mut log = crate::log::Logger::new();
	let main_component = Component::parse(&mut log,
		"src/build/tests/build_reports_cycle_and_missing_import_main.vue",
		include_str!("tests/build_reports_cycle_and_missing_import_main.vue"),
	).unwrap();
	let child_component = Component::parse(&mut log,
		"src/build/tests/build_reports_cycle_and_missing_import_child.vue",
		include_str!("tests/build_reports_cycle_and_missing_import_child.vue"),
	).unwrap();
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
	let mut log = crate::log::Logger::new();
	let component = Component::parse(&mut log,
		"src/build/tests/parses_valid_vue_fragment.vue",
		include_str!("tests/parses_valid_vue_fragment.vue"),
	).unwrap();

	assert_eq!(component.path, "src/build/tests/parses_valid_vue_fragment.vue");
	assert_eq!(component.links, vec!["src/build/tests/components/child.vue"]);
	assert_eq!(component.imports, vec!["import { createApp } from 'vue';\n"]);
	assert_eq!(component.custom_tag, None);
	assert!(component.script.as_deref().unwrap().contains("console.log"));
	assert!(!component.script.as_deref().unwrap().contains("import { createApp } from 'vue';"));
	assert!(component.template.as_deref().unwrap().starts_with("<template>"));
	assert!(component.style.as_deref().unwrap().contains("div { color: red; }"));
	assert!(!component.style.as_deref().unwrap().contains("<style>"));
}

#[test]
fn warns_when_component_root_is_missing_id() {
	let mut log = crate::log::Logger::new();
	let component = Component::parse(&mut log,
		"src/build/tests/missing_root_id.vue",
		"<template><example-child></example-child></template>\n",
	).unwrap();

	assert_eq!(component.custom_tag, None);
	assert_eq!(component.used_custom_tags.len(), 1);
	assert_eq!(component.used_custom_tags[0].tag, "example-child");
	assert_eq!(log.error_count(), 0);
	assert_eq!(log.warning_count(), 1);
}

#[test]
fn validates_direct_component_import_usage() {
	let mut log = crate::log::Logger::new();
	let main_component = Component::parse(&mut log,
		"src/build/tests/validate_main.vue",
		"<link rel=\"component\" href=\"known-child.vue\">\n<link rel=\"component\" href=\"unused-child.vue\">\n<div id=\"validate-main\">\n	<known-child></known-child>\n	<missing-child></missing-child>\n</div>\n",
	).unwrap();
	let known_child = Component::parse(&mut log,
		"src/build/tests/known-child.vue",
		"<div id=\"known-child\"></div>\n",
	).unwrap();
	let unused_child = Component::parse(&mut log,
		"src/build/tests/unused-child.vue",
		"<div id=\"unused-child\"></div>\n",
	).unwrap();

	assert_eq!(log.error_count(), 0);
	assert_eq!(log.warning_count(), 0);

	validate_components(&mut log, &[main_component, known_child, unused_child]);

	assert_eq!(log.error_count(), 1);
	assert_eq!(log.warning_count(), 1);
	assert!(log.has_errors());
	assert!(log.has_warnings());
}

#[test]
fn allows_top_level_comments_and_whitespace() {
	let mut log = crate::log::Logger::new();
	let component = Component::parse(&mut log,
		"src/build/tests/allows_top_level_comments_and_whitespace.vue",
		"\n<!-- leading comment -->\n<link rel=\"component\" href=\"components/child.vue\">\n<!-- between blocks -->\n<script>\nimport helper from './helper.js';\nconsole.log(helper);\n</script>\n\n<template><div>Hello</div></template>\n<!-- trailing comment -->\n<style>\ndiv { color: red; }\n</style>\n",
	).unwrap();

	assert_eq!(component.links, vec!["src/build/tests/components/child.vue"]);
	assert_eq!(component.imports, vec!["import helper from './helper.js';\n"]);
	assert!(component.script.as_deref().unwrap().contains("console.log(helper);"));
	assert_eq!(component.template.as_deref(), Some("<template><div>Hello</div></template>"));
	assert!(component.style.as_deref().unwrap().contains("div { color: red; }"));
}

#[test]
fn rejects_non_whitespace_top_level_text() {
	let mut log = crate::log::Logger::new();
	Component::parse(&mut log,
		"src/build/tests/rejects_non_whitespace_top_level_text.vue",
		"hello\n<template><div>Hello</div></template>\n",
	);
	assert!(!log.finished(), "non-whitespace top level text should be reported as an error");
}

#[test]
fn parses_vue_shorthand_attributes_in_template_roots() {
	let mut log = crate::log::Logger::new();
	let component = Component::parse(&mut log,
		"src/build/tests/parses_vue_shorthand_attributes_in_template_roots.vue",
		"<script>\nconsole.log('ok');\n</script>\n<div id=\"app\" class=\"app-main\" @drop=\"dropFile\" @dragenter=\"\">\n\t<app-viewer v-if=\"reader != null\" :reader=\"reader\" @set-status=\"setStatusLine\"></app-viewer>\n\t<div class=\"footer\">{{ status }}</div>\n</div>\n<style>\n.app-main { color: red; }\n</style>\n",
	).unwrap();

	let template = component.template.as_deref().expect("template should be present");
	assert!(template.contains("@drop=\"dropFile\""));
	assert!(template.contains("@dragenter=\"\""));
	assert!(template.contains(":reader=\"reader\""));
	assert!(template.contains("@set-status=\"setStatusLine\""));
	assert!(template.contains("<app-viewer"));
}

#[test]
fn normalizes_component_use_paths() {
	let mut log = crate::log::Logger::new();
	let component = Component::parse(&mut log,
		"src/build/tests/normalizes_component_use_paths.vue",
		include_str!("tests/normalizes_component_use_paths.vue"),
	).unwrap();

	assert_eq!(component.links, vec!["src/build/child.vue"]);
}

#[test]
fn extracts_multiple_import_lines_from_script_contents() {
	let mut log = crate::log::Logger::new();
	let component = Component::parse(&mut log,
		"src/build/tests/extracts_multiple_import_lines_from_script_contents.vue",
		include_str!("tests/extracts_multiple_import_lines_from_script_contents.vue"),
	).unwrap();

	assert_eq!(component.links, vec!["src/build/tests/components/child.vue", "src/build/tests/components/sibling.vue"]);
	assert_eq!(component.imports, vec!["import { createApp } from 'vue';\n", "import helper from './helper.js';\n"]);
	assert_eq!(component.script.as_deref().unwrap().trim(), "console.log(\"ok\");");
}

#[test]
fn rejects_component_use_paths_above_root() {
	let mut log = crate::log::Logger::new();
	let _component = Component::parse(&mut log,
		"src/build/tests/rejects_component_use_paths_above_root.vue",
		include_str!("tests/rejects_component_use_paths_above_root.vue"),
	).unwrap();
	assert!(!log.finished(), "component use paths above root should be reported as an error");
}

#[test]
fn rejects_non_component_links() {
	let mut log = crate::log::Logger::new();
	let _component = Component::parse(&mut log,
		"src/build/tests/ignores_non_component_links.vue",
		include_str!("tests/ignores_non_component_links.vue"),
	).unwrap();
	assert!(!log.finished(), "non-component links should be reported as an error");
}

#[test]
fn rejects_component_links_without_href() {
	let mut log = crate::log::Logger::new();
	let _component = Component::parse(&mut log,
		"src/build/tests/rejects_component_links_without_href.vue",
		include_str!("tests/rejects_component_links_without_href.vue"),
	).unwrap();
	assert!(!log.finished(), "component links without href should be reported as an error");
}

#[test]
fn extracts_import_lines_from_vue_js_helpers() {
	let mut log = crate::log::Logger::new();
	let component = Component::parse(&mut log,
		"src/build/tests/extracts_import_lines_from_vue_js_helpers.vue.js",
		include_str!("tests/extracts_import_lines_from_vue_js_helpers.vue.js"),
	).unwrap();

	assert_eq!(component.imports, vec!["import helper from './helper.js';\n"]);
	assert_eq!(component.script.as_deref().unwrap().trim(), "const answer = 42;");
}

#[test]
fn ignores_non_statement_import_prefixes() {
	let mut log = crate::log::Logger::new();
	let component = Component::parse(&mut log,
		"src/build/tests/ignores_non_statement_import_prefixes.vue.js",
		include_str!("tests/ignores_non_statement_import_prefixes.vue.js"),
	).unwrap();

	assert_eq!(component.imports, vec!["import{ named } from './helper.js';\n"]);
	assert!(component.script.as_deref().unwrap().contains("importedAt = Date.now();"));
	assert!(component.script.as_deref().unwrap().contains("import.meta.env;"));
	assert!(component.script.as_deref().unwrap().contains("const value = import('helper');"));
}

#[test]
fn rejects_multiple_script_tags() {
	let mut log = crate::log::Logger::new();
	let _component = Component::parse(&mut log,
		"src/build/tests/rejects_multiple_script_tags.vue",
		include_str!("tests/rejects_multiple_script_tags.vue"),
	).unwrap();
	assert!(!log.finished(), "multiple script tags should be reported as an error");
}

#[test]
fn rejects_template_and_div_together() {
	let mut log = crate::log::Logger::new();
	let _component = Component::parse(&mut log,
		"src/build/tests/rejects_template_and_div_together.vue",
		include_str!("tests/rejects_template_and_div_together.vue"),
	).unwrap();
	assert!(!log.finished(), "template and div together should be reported as an error");
}

#[test]
fn rejects_full_document() {
	let mut log = crate::log::Logger::new();
	let _component = Component::parse(&mut log,
		"src/build/tests/rejects_full_document.vue",
		include_str!("tests/rejects_full_document.vue"),
	).unwrap();
	assert!(!log.finished(), "full document should be reported as an error");
}

#[test]
fn renders_styles_in_single_tag() {
	let components = vec![
		Component {
			path: "app/one.vue".to_string(),
			source: String::new(),
			links: Vec::new(),
			imports: Vec::new(),
			custom_tag: None,
			used_custom_tags: Vec::new(),
			template: None,
			script: None,
			style: Some(".one { color: red; }".to_string()),
		},
		Component {
			path: "app/two.vue.css".to_string(),
			source: String::new(),
			links: Vec::new(),
			imports: Vec::new(),
			custom_tag: None,
			used_custom_tags: Vec::new(),
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
