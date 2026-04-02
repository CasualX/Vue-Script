use super::*;

#[test]
fn parses_valid_vue_fragment() {
	let component = Component::parse(
		"app/main.vue",
		r#"
<script use="components/child.vue" import="{ createApp } from 'vue'">
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
	assert!(component.template.as_deref().unwrap().starts_with("<template>"));
	assert!(component.style.as_deref().unwrap().contains("div { color: red; }"));
	assert!(!component.style.as_deref().unwrap().contains("<style>"));
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
