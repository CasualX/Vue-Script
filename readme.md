# Vue Script

[![MIT License](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![crates.io](https://img.shields.io/crates/v/vue-script.svg)](https://crates.io/crates/vue-script)
[![Build status](https://github.com/CasualX/Vue-Script/actions/workflows/check.yml/badge.svg)](https://github.com/CasualX/Vue-Script/actions/workflows/check.yml)

Vue 3 Bespoke Single File Components without the insanity that comes with the NPM ecosystem.

Vue Script is a small build tool for global-runtime Vue single file components. The bundled example uses Vue 3 loaded from the page HTML, while the builder itself focuses on assembling templates, dependency-ordered scripts, imports, and styles into one final HTML file.

## Install

Various installation methods are available:

```bash
# Install from crates.io (recommended for most users)
cargo install vue-script
# Install from a local checkout (for development)
cargo install --path .
# Or, if you want to run it without installing:
cargo run -- [COMMAND]
```

## Commands

The project root is the directory that contains `vue-script.toml`.

Build the configured target:

```bash
vue-script build
```

Check the project without writing the configured target:

```bash
vue-script check
```

The check command runs Vue-Script's component and HTML-fragment validation, assembles the final JavaScript module in memory, and checks that module with TypeScript.
It requires `[check].target` so the ECMAScript environment is explicit; use a TypeScript target such as `"es2020"`, `"es2022"`, or `"esnext"` that matches the browsers supported by the project.
It uses `tsc` on `PATH` by default. Set `[check].typescript` to select a specific compiler;
absolute paths are used as-is and relative paths are resolved from the directory containing `vue-script.toml`.
TypeScript is an optional development dependency and is not used by `build` or required by the browser runtime.

Vue-Script injects a small checker-only declaration for the global-runtime `Vue` object. Wrap Options API component objects in `Vue.defineComponent({ ... })` to infer data, prop, computed, and method properties on `this` and validate `$emit` event names without installing the Vue type package. The declaration intentionally models only this lightweight subset; event payloads and other Vue global APIs remain dynamically typed, and implicit function parameter types are allowed for ordinary JavaScript.

VS Code includes TypeScript language support, but it does not install the `tsc` command-line compiler. TypeScript is normally installed through npm, which is included with [Node.js](https://nodejs.org/). A project-local installation is recommended because it pins the checker version for the project:

```bash
# Only needed when the project does not have a package.json yet.
npm init -y
npm install --save-dev typescript

# Confirm that the local compiler works.
npx tsc --version
```

Configure Vue-Script to use the local compiler:

```toml
[check]
target = "es2022"
typescript = "node_modules/.bin/tsc"
```

Alternatively, omit `[check].typescript` and install `tsc` once for the whole machine:

```bash
npm install --global typescript
tsc --version
```

See the official [TypeScript installation guide](https://www.typescriptlang.org/download/) for other package managers and platforms.

Diagnostics default to the normal human-readable format. Tools can request a JSON array instead:

```bash
vue-script check --message-format json
```

Each JSON diagnostic contains a level, message, optional TypeScript error code, optional source span, and optional help note. Lines and columns in JSON output are one-indexed. TypeScript diagnostics from the assembled module are mapped back to the originating `.vue` or `.vue.js` source lines.

Build and open the configured target file:

```bash
vue-script open
```

Build, start a loopback-only Python HTTP server from the project root, and open the configured target URL (requires Python to be installed and available in PATH):

```bash
vue-script serve --port 8123
```

Run this in a separate terminal as it will block the current terminal.
In blocking mode, `serve` also watches the configured `[serve].watch` globs and rebuilds when matching files change. The default port is 8000 if not specified.

## Usage

This repo contains a simple example project.

You will need a [vue-script.toml](vue-script.toml) configuration file. This serves as the entry point for the build process. Run `vue-script` from the directory or a subdirectory where this file is located.

The vue-script.toml declares the location of the target file (the file that will be opened in the browser) and the source files (the .vue/.html/.js files that will be processed). The source files can be located anywhere, but they must be specified relative to the project root.

When using `serve`, you can provide repeated `watch = "..."` entries under `[serve]` to define which project-relative paths should trigger rebuilds. These entries use glob syntax.

```toml
[serve]
watch = "app/**/*"
```

The configured `[target].path` is always ignored implicitly so writes to the generated output do not trigger rebuild loops.

The project configuration file `vue-script.toml` is also watched implicitly so configuration edits trigger a debounced rebuild when file watching is enabled.

Vue-Script `.vue` files are HTML fragments with zero or more top-level `<link rel="component" href="...">` elements, an optional `<script>`, an optional `<template>` or `<div>`, and an optional `<style>`. Put one component link at the top level for each child component or helper file dependency.

Vue-Script treats every `is` value as opaque. When using `<component :is="...">`, mark each corresponding link as dynamic:

```html
<link rel="component" href="pages/home-page.vue" dynamic>
<link rel="component" href="pages/account-page.vue" dynamic>

<script>
const routes = [
	{ path: '/', component: HomePage },
	{ path: '/account', component: AccountPage },
];
</script>

<template id="route-outlet">
	<component :is="activeRoute.component"></component>
</template>
```

The `dynamic` attribute suppresses the unused-component warning only for the link on which it appears. Other unused component links continue to produce warnings.

JavaScript module imports should be written as normal `import ...;` lines inside the `<script>` block or a `.vue.js` helper file. Vue-Script collects lines that start with `import` after trimming leading whitespace and emits them before the remaining script bodies in the final module script.
Module specifiers are preserved verbatim, so relative imports are resolved by the browser relative to the generated target HTML. This is intended for JavaScript modules placed alongside the target and its other vendored static resources, not relative to the source `.vue` file.

Component link paths are relative to the `.vue` file in which they are written.

📜 License
----------

Licensed under [MIT License](https://opensource.org/licenses/MIT), see [license.txt](license.txt).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, shall be licensed as above, without any additional terms or conditions.
