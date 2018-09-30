
use std::{fs};
use std::path::Path;

use clap::ArgMatches;

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

// Markers that identify parts of the file we're interested in.
// Anchor the tags to newlines to avoid false positives.

const SCRIPT_START_TAG: &str = "\n<script";
const SCRIPT_END_TAG: &str = "\n</script>";
const STYLE_START_TAG: &str = "\n<style";
const STYLE_END_TAG: &str = "\n</style>";
const TEMPLATE_START_TAG: &str = "\n<template";
const TEMPLATE_END_TAG: &str = "\n</template>";
const DIV_START_TAG: &str = "\n<div";
const DIV_END_TAG: &str = "\n</div>";

// The input file.

const INPUT_DIR: &str = "app";
const INPUT_FILE: &str = "app/page.html";

pub fn main(_matches: &ArgMatches<'_>) {
	let config_path = match super::config_path() {
		Some(config_path) => config_path,
		None => return,
	};

	let project_path = config_path.parent().unwrap();

	let mut template = String::new();
	let mut script = String::new();
	let mut style = String::new();

	visit_dirs(&project_path.join(INPUT_DIR), &mut |path| {
		dbg!(&path);
		match fs::read_to_string(path) {
			Ok(contents) => {
				match (
					contents.find(SCRIPT_START_TAG), contents.find(SCRIPT_END_TAG)
				) {
					(Some(start), Some(end)) if start < end => {
						script.push_str(&contents[start..end + SCRIPT_END_TAG.len()]);
					},
					(None, None) => {},
					_ => {
						eprintln!("warn: Failed to parse script for {}!", path.display());
					},
				}

				match (
					contents.find(STYLE_START_TAG), contents.find(STYLE_END_TAG)
				) {
					(Some(start), Some(end)) if start < end => {
						style.push_str(&contents[start..end + STYLE_END_TAG.len()]);
					},
					(None, None) => {},
					_ => {
						eprintln!("warn: Failed to parse style for {}!", path.display());
					},
				}

				match (
					contents.find(TEMPLATE_START_TAG), contents.find(TEMPLATE_END_TAG),
					contents.find(DIV_START_TAG), contents.find(DIV_END_TAG)
				) {
					(Some(start), Some(end), None, None) if start < end => {
						template.push_str(&contents[start..end + TEMPLATE_END_TAG.len()]);
					},
					(None, None, Some(start), Some(end)) if start < end => {
						template.push_str(&contents[start..end + DIV_END_TAG.len()]);
					},
					(Some(_), Some(_), Some(_), Some(_)) => {
						eprintln!();
					},
					(None, None, None, None) => {},
					_ => {
						eprintln!("warn: Failed to parse style for {}!", path.display());
					},
				}
			},
			Err(err) => eprintln!("warn: Failed read_to_string(\"{}\"): {}", path.display(), err),
		}
	});

	match fs::read_to_string(&project_path.join(INPUT_FILE)) {
		Ok(contents) => {
			let contents = replace(&contents, "$SCRIPTS$", &script);
			let contents = replace(&contents, "$STYLES$", &style);
			let contents = replace(&contents, "$TEMPLATES$", &template);
			println!("{}", contents);
		},
		Err(err) => {
			eprintln!("warn: Failed to read_to_string(\"{}\"): {}", INPUT_FILE, err);
		}
	}
}

fn visit_dirs(dir: &Path, f: &mut FnMut(&Path)) {
	match fs::read_dir(dir) {
		Ok(read_dir) => {
			// Separate the directories from the files
			let mut dirs = Vec::new();
			let mut files = Vec::new();
			read_dir.for_each(|entry_r| {
				if let Ok(entry) = entry_r {
					let path = entry.path();
					if path.is_dir() {
						dirs.push(path);
					}
					else if path.extension() == Some("vue".as_ref()) {
						files.push(path);
					}
				}
			});
			// Ensure the files are processed before walking the directories
			for path in &files {
				f(path);
			}
			for path in &dirs {
				visit_dirs(path, f);
			}
		},
		Err(err) => eprintln!("warn: Failed read_dir(\"{}\"): {}", dir.display(), err),
	}
}
