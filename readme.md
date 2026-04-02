# Vue Script

Vue 2 Bespoke Single File Components (VB.sfc) without the insanity that comes with the NPM ecosystem.

Vue Script is a small build tool for Vue 2 style single file components. It reads a project description from `vue-script.toml`, pulls templates and component imports together, and writes a final HTML file.

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
vue-script serve
```

Run this in a separate terminal as it will block the current terminal.

## Usage

This repo contains a simple example project.

You will need a [vue-script.toml](vue-script.toml) configuration file. This serves as the entry point for the build process. Run `vue-script` from the directory or a subdirectory where this file is located.

The vue-script.toml declares the location of the target file (the file that will be opened in the browser) and the source files (the .vue/.html/.js files that will be processed). The source files can be located anywhere, but they must be specified relative to the project root.

All paths are relative to the file in which they are written.

📜 License
----------

Licensed under [MIT License](https://opensource.org/licenses/MIT), see [license.txt](license.txt).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, shall be licensed as above, without any additional terms or conditions.
