use super::*;

#[test]
fn parses_valid_vue_fragment() {
	let component = Component::parse(
		"app/main.vue",
		r#"
<link rel="component" href="components/child.vue">
<script>
	import { createApp } from 'vue';
console.log("ok");
</script>
<template>
<div>Hello</div>
</template>
<style>
div { color: red; }
</style>
"#,
	)
	.expect("component should parse");

	assert_eq!(component.path, "app/main.vue");
	assert_eq!(component.uses, vec!["app/components/child.vue"]);
	assert_eq!(component.imports, vec!["import { createApp } from 'vue';\n"]);
	assert!(component.script.as_deref().unwrap().contains("console.log"));
	assert!(!component.script.as_deref().unwrap().contains("import { createApp } from 'vue';"));
	assert!(component.template.as_deref().unwrap().starts_with("<template>"));
	assert!(component.style.as_deref().unwrap().contains("div { color: red; }"));
	assert!(!component.style.as_deref().unwrap().contains("<style>"));
}

#[test]
fn normalizes_component_use_paths() {
	let component = Component::parse(
		"app/components/main.vue",
		r#"
<link rel="component" href="../shared/../child.vue">
<script>
console.log("ok");
</script>
<div></div>
"#,
	)
	.expect("component should parse");

	assert_eq!(component.uses, vec!["app/child.vue"]);
}

#[test]
fn extracts_multiple_import_lines_from_script_contents() {
	let component = Component::parse(
		"app/main.vue",
		r#"
<link rel="component" href="components/child.vue">
<link rel="component" href="components/sibling.vue">
<script>
	import { createApp } from 'vue';
	import helper from './helper.js';
console.log("ok");
</script>
<div></div>
"#,
	)
	.expect("component should parse");

	assert_eq!(component.uses, vec!["app/components/child.vue", "app/components/sibling.vue"]);
	assert_eq!(component.imports, vec!["import { createApp } from 'vue';\n", "import helper from './helper.js';\n"]);
	assert_eq!(component.script.as_deref().unwrap().trim(), "console.log(\"ok\");");
}

#[test]
fn rejects_component_use_paths_above_root() {
	assert!(Component::parse(
		"main.vue",
		r#"
<link rel="component" href="../child.vue">
<script>
console.log("ok");
</script>
<div></div>
"#,
	)
	.is_none());
}

#[test]
fn ignores_non_component_links() {
	let component = Component::parse(
		"app/main.vue",
		r#"
<link rel="stylesheet" href="ignored.css">
<link rel="component" href="components/child.vue">
<script>
	console.log("ok");
</script>
<div></div>
"#,
	)
	.expect("component should parse");

	assert_eq!(component.uses, vec!["app/components/child.vue"]);
}

#[test]
fn rejects_component_links_without_href() {
	assert!(Component::parse(
		"app/main.vue",
		r#"
<link rel="component">
<script>
	console.log("ok");
</script>
<div></div>
"#,
	)
	.is_none());
}

#[test]
fn extracts_import_lines_from_vue_js_helpers() {
	let component = Component::parse(
		"app/helpers/example.vue.js",
		r#"import helper from './helper.js';
const answer = 42;
"#,
	)
	.expect("helper should parse");

	assert_eq!(component.imports, vec!["import helper from './helper.js';\n"]);
	assert_eq!(component.script.as_deref().unwrap().trim(), "const answer = 42;");
}

#[test]
fn ignores_non_statement_import_prefixes() {
	let component = Component::parse(
		"app/helpers/example.vue.js",
		r#"importedAt = Date.now();
import.meta.env;
const value = import('helper');
import{ named } from './helper.js';
"#,
	)
	.expect("helper should parse");

	assert_eq!(component.imports, vec!["import{ named } from './helper.js';\n"]);
	assert!(component.script.as_deref().unwrap().contains("importedAt = Date.now();"));
	assert!(component.script.as_deref().unwrap().contains("import.meta.env;"));
	assert!(component.script.as_deref().unwrap().contains("const value = import('helper');"));
}

#[test]
fn rejects_multiple_script_tags() {
	assert!(Component::parse(
		"app/main.vue",
		r#"
<script>one</script>
<script>two</script>
"#,
	)
	.is_none());
}

#[test]
fn rejects_template_and_div_together() {
	assert!(Component::parse(
		"app/main.vue",
		r#"
<template><span /></template>
<div></div>
"#,
	)
	.is_none());
}

#[test]
fn rejects_full_document() {
	assert!(Component::parse(
		"app/main.vue",
		r#"
<!doctype html>
<html>
<body>
	<div></div>
</body>
</html>
"#,
	)
	.is_none());
}

#[test]
fn renders_styles_in_single_tag() {
	let components = vec![
		Component {
			path: "app/one.vue".to_string(),
			uses: Vec::new(),
			imports: Vec::new(),
			template: None,
			script: None,
			style: Some(".one { color: red; }".to_string()),
		},
		Component {
			path: "app/two.vue.css".to_string(),
			uses: Vec::new(),
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
