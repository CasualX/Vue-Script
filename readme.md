# Vue Script

Vue 3 Bespoke Single File Components (VB.sfc) without the insanity that comes with the NPM ecosystem.

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

Build and open the configured target file:

```bash
vue-script open
```

Build, start a Python HTTP server from the project root, and open the configured target URL (requires Python to be installed and available in PATH):

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

JavaScript module imports should be written as normal `import ...;` lines inside the `<script>` block or a `.vue.js` helper file. Vue-Script collects lines that start with `import` after trimming leading whitespace and emits them before the remaining script bodies in the final module script.

All paths are relative to the file in which they are written.

📜 License
----------

Licensed under [MIT License](https://opensource.org/licenses/MIT), see [license.txt](license.txt).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, shall be licensed as above, without any additional terms or conditions.
