---
name: vue-script
description: 'Build and edit apps that use Vue-Script, a custom Vue build tool. Use when working with vue-script.toml, configured page HTML placeholders, .vue components, .vue.js helpers, .vue.css styles, or when wiring component link dependencies and import lines in Vue-Script projects.'
argument-hint: 'Describe the app, components, and any Vue-Script files to create or update.'
---

# Vue-Script

Use this skill when building or modifying apps that use Vue-Script.

Vue-Script is not Vite, webpack, or Vue SFC tooling. It is a custom Rust build script that assembles Vue apps into one HTML file by:

1. Loading configuration from `vue-script.toml`.
2. Starting from the configured main component.
3. Recursively following component dependencies declared by top-level `<link rel="component" href="...">` elements.
4. Collecting templates, scripts, imports, and styles.
5. Injecting them into the configured page template.

The default authoring model in this repo is Vue 3 loaded from the page HTML, not a bundler-managed runtime.

## When To Use

- Creating a new Vue-Script app or component tree.
- Editing `vue-script.toml`, page shell HTML, or `.vue` component files.
- Adding shared helper code in `.vue.js` files.
- Adding shared styles in `.vue.css` files.
- Fixing build issues caused by bad dependency paths, bad component ordering, or malformed top-level `.vue` structure.

## Core Model

Author apps around these pieces:

- `vue-script.toml`: project config.
- The page HTML configured by `[app].page`, containing placeholder comments.
- The root component configured by `[app].main`.
- `.vue`: Vue-Script component fragments.
- `.vue.js`: plain JavaScript helper modules that are inlined into the final module script.
- `.vue.css`: plain CSS files that are merged into the final style block.

The builder reads these config keys:

```toml
[app]
page = "path/to/page.html"
main = "path/to/main.vue"

[target]
path = "path/to/output.html"
```

Paths in `vue-script.toml` are relative to the project root, which is the directory that contains `vue-script.toml`.

`[target].path` is optional. If it is omitted, the builder prints the assembled HTML to stdout instead of writing a file.

## Vue 3 Runtime Assumption

The configured page HTML is responsible for loading Vue 3. Build component scripts assuming `Vue` is available globally.

The page shell should load Vue 3 before the injected app code and preserve the three build placeholders, for example:

```html
<!DOCTYPE html>
<html lang="en">
<head>
	<meta charset="UTF-8">
	<meta name="viewport" content="width=device-width, initial-scale=1.0">
	<title>Vue-Script App</title>
	<script src="https://cdn.jsdelivr.net/npm/vue@3/dist/vue.global.js"></script>
<!-- STYLES -->
</head>
<body>
<!-- TEMPLATES -->
<!-- SCRIPTS -->
</body>
</html>
```

Prefer Vue 3 global-runtime patterns such as `Vue.createApp(...)`, `app.component(...)`, and component objects exported through the assembled script order.

## Supported File Types

Vue-Script recognizes exactly these file name suffixes:

- `.vue`: full component fragment.
- `.vue.js`: script-only helper file.
- `.vue.css`: style-only helper file.

Any other component-like extension is unsupported by the builder.

## `.vue` File Structure

A `.vue` file must be an HTML fragment, not a full HTML document. Top-level content must be ordered as:

1. Zero or more `<link>` elements
2. Optional `<script>`
3. Optional `<template>` xor `<div>`
4. Optional `<style>`

Important constraints enforced by the parser:

- No extra top-level text nodes.
- No extra top-level comments.
- No duplicate top-level sections.
- Do not include both `<template>` and `<div>` in the same file.
- `<script>` and `<style>` must use explicit closing tags.
- Keep `<script>` and `<style>` bodies plain text so they can be extracted cleanly.

Use this shape for a parent or root component file that renders two child components:

```html
<link rel="component" href="components/hero-banner.vue">
<link rel="component" href="components/feature-list.vue">

<script>
const app = Vue.createApp({
	data() {
		return {
			title: 'Hello Vue-Script',
			features: ['Small build tool', 'Vue 3 runtime', 'Single HTML output'],
		};
	},
});

app.component('hero-banner', HeroBanner);
app.component('feature-list', FeatureList);
app.mount('#app');
</script>

<div id="app">
	<hero-banner :title="title"></hero-banner>
	<feature-list :items="features"></feature-list>
</div>
```

Any child component or helper used by this file should be declared with a top-level `<link rel="component" href="...">` element.

Use this shape for a child component file that uses `<template>`.
A good convention is to use the same string for the template id and the root element class name so the relationship stays obvious,
then use BEM-style class names such as `hero-banner__title` for internal elements:

```html
<script>
const HeroBanner = {
	template: '#hero-banner',
	props: {
		title: String,
	},
};
</script>

<template id="hero-banner">
	<section class="hero-banner">
		<h1 class="hero-banner__title">{{ title }}</h1>
	</section>
</template>

<style>
.hero-banner {
	padding: 24px;
	border-bottom: 1px solid #ddd;
}

.hero-banner__title {
	margin: 0;
}
</style>
```

## Dependency Rules

There are two different script inputs that affect build output.

### Component links

Use one top-level `<link rel="component" href="...">` element per Vue-Script file dependency required by the current file.

Example:

```html
<link rel="component" href="components/hero-banner.vue">
<link rel="component" href="components/feature-list.vue">
<link rel="component" href="helpers/formatters.vue.js">

<script>
const app = Vue.createApp({});

app.component('hero-banner', HeroBanner);
app.component('feature-list', FeatureList);
app.mount('#app');
</script>
```

Behavior:

- Each `<link rel="component">` adds another file to the build graph.
- Paths are resolved relative to the current component's directory.
- Dependencies are traversed recursively from `app.main`.
- Dependency scripts are emitted before the component that depends on them.
- A parent file declares component links for its children and helpers; a child component does not depend on itself.
- Declare every child component or helper that the current file depends on.
- `rel="component"` is the supported dependency declaration.
- Non-component link relations are ignored with a warning.
- Each component link must provide an `href` attribute.

Use component links for:

- Child `.vue` components.
- Shared `.vue.js` helpers.
- Shared `.vue.css` styles.

Treat the dependency graph as a directed acyclic graph. The builder orders scripts by walking dependencies first, so circular dependency chains break the intended ordering and produce warnings.

### `import` lines in script source

Use normal JavaScript `import` lines directly inside `.vue` `<script>` blocks or `.vue.js` helper files when you need external module imports.

Example:

```html
<script>
	import { something } from './module.js';

	Vue.createApp({}).mount('#app');
</script>
```

Behavior:

- The builder looks for lines that start with `import` after trimming leading whitespace.
- Matching lines are collected and emitted before the remaining component script bodies in the final `<script type="module">` block.
- Collected import lines do not add Vue-Script files to the dependency graph.
- Import extraction is ad hoc and line-based, not a full JavaScript parse.

Use import lines for standard JavaScript module imports. Use component links for Vue-Script source files.

Keep the distinction clear:

- `<link rel="component">` participates in Vue-Script dependency discovery and ordering.
- `import` lines only emit JavaScript import statements into the final module script.

## Path Conventions

When authoring `<link rel="component" href="...">` entries, treat the `href` value as relative to the file that contains it.

Examples:

- From `src/main.vue`, `<link rel="component" href="components/hero-banner.vue">` resolves to `src/components/hero-banner.vue`.
- From `src/main.vue`, `<link rel="component" href="helpers/formatters.vue.js">` resolves to `src/helpers/formatters.vue.js`.
- From `src/components/feature-list.vue`, `<link rel="component" href="item-pill.vue">` resolves to `src/components/item-pill.vue`.

Do not treat `href` values as project-root relative unless the current file is already at the root.

## Build Output Assembly

The builder reads the page HTML and replaces three placeholder comments:

- `<!-- STYLES -->`
- `<!-- TEMPLATES -->`
- `<!-- SCRIPTS -->`

Assembly behavior:

- All discovered styles are merged into one `<style>` block.
- All discovered templates are concatenated and inserted as raw markup.
- All discovered scripts are placed into one `<script type="module">` block.
- Collected `import` statements are emitted before component script bodies.
- Script bodies are ordered so dependencies come before dependents.

If a placeholder is missing, the builder warns and leaves the source HTML unchanged at that location.

## Practical Authoring Guidance

When asked to build an app with Vue-Script, follow this workflow:

1. Inspect `vue-script.toml` to find the real page and main entry paths.
2. Inspect the page HTML and preserve the placeholder comments.
3. Keep Vue 3 runtime loading in the page shell.
4. Declare every child component and helper dependency with top-level `<link rel="component" href="...">` elements on the file that needs it.
5. Keep JavaScript imports as normal `import ...;` lines in the script body, one import per line if you want Vue-Script to extract them.
6. Put reusable helpers in `.vue.js` files when they do not need a template.
7. Put reusable global CSS in `.vue.css` files when it is shared across multiple components.
8. Use BEM-style class naming to keep component styles local in practice, for example `.hero-banner`, `.hero-banner__title`, and `.hero-banner--compact`.
9. Keep component names, template ids, custom element tags, and registered component objects aligned.
10. Prefer small dependency graphs with clear one-way dependency relationships.
11. Avoid circular dependency chains; the builder only warns and may skip part of the cycle.

## Typical Build Result

For a project where a root component depends on child components and helpers:

- Child template markup is inserted into the configured page output.
- Helper JavaScript is emitted before the component code that calls it.
- All collected `import` lines are emitted before the remaining script bodies.
- All discovered styles are merged into one style block.

## What To Avoid

- Do not write Vue 2 `new Vue(...)` or global `Vue.component(...)` patterns in new example code unless the existing project already depends on them.
- Do not assume `.vue` files are compiled by npm tooling.
- Do not add unsupported top-level blocks such as `<script setup>`.
- Do not remove or rename the HTML placeholder comments.
- Do not introduce file extensions that the Vue-Script tool does not parse.
- Do not use scoped styles; Vue-Script merges styles globally and does not provide SFC-style style scoping.
- Do not rely on multi-line or dynamically constructed import statements being extracted correctly; the current import handling is intentionally line-based.
- Do not use generic class names that are likely to leak across components; prefer BEM-style naming to keep selectors component-specific.

## Build And Validation

Agents should only run the build command themselves:

```bash
vue-script build
```

Other supported commands exist, but the agent should not run them automatically:

```bash
vue-script open
vue-script serve
```

Command behavior:

- `vue-script build` assembles the configured output and is the only Vue-Script CLI command the agent should run directly.
- `vue-script open` builds and opens the generated target as a local file in the browser, using the built output path directly rather than serving it over HTTP.
- `vue-script serve` builds, starts a Python `http.server` from the project root on port `8000`, opens the resulting `http://127.0.0.1:8000/...` URL, and blocks the terminal until stopped unless the user explicitly uses detached mode.

Agent rule:

- Do not run `vue-script open`.
- Do not run `vue-script serve`.
- If the user wants to preview the built app in a browser, ask them to run `vue-script open` themselves.
- If the user wants a local HTTP preview, ask them to run `vue-script serve` themselves in a separate terminal because it is blocking.

For validation, prefer checking that:

- `vue-script.toml` points to real files.
- Each component link `href` resolves correctly relative to its component.
- The page HTML still contains all three placeholders.
- `.vue` files use the expected top-level order.
- Generated app behavior matches Vue 3 global-runtime assumptions.

## Implementation Notes For Agents

When generating Vue-Script code:

- Follow the file naming and placement conventions already used by the target project.
- Keep changes minimal and consistent with the capabilities of the Vue-Script tool.
- Prefer Vue 3 global-runtime patterns unless the target codebase already uses older Vue APIs.
- If the user asks for features that the Vue-Script tool does not support, explain the limitation and work within the supported workflow instead of inventing unsupported behavior.
