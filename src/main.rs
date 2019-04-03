
extern crate clap;

use std::{env};
use std::path::{PathBuf};
use clap::{App, SubCommand};

fn main() {
	let app = App::new("Vue Script")
		.version("0.1")
		.author("Casper <CasualX@users.noreply.github.com>")
		.about("Vue Single File Components without the insanity that comes with the NPM ecosystem")
		.subcommand(SubCommand::with_name("build")
			.about("Compiles the Vue Single File Components for distribution")
		)
		.subcommand(SubCommand::with_name("open")
			.about("Opens the Vue Single File Components in a local browser")
		);
	let matches = app.get_matches();

	match matches.subcommand() {
		("build", Some(build_matches)) => build::main(build_matches),
		("open", Some(_open_matches)) => (),
		(command, Some(_)) => unreachable!("Unknown command: {}", command),
		(_, None) => println!("Welcome!"),
	}
}

// Helpers

const CONFIG_FILE: &str = "vue-script.toml";

fn config_path() -> Option<PathBuf> {
	let current_dir = match env::current_dir() {
		Ok(path) => path,
		Err(err) => {
			eprintln!("warn: Current directory not available: {}", err);
			eprintln!("warn: Falling back to the executable path");
			env::args_os().next().unwrap().into()
		},
	};

	let mut path = current_dir.clone();
	loop {
		path.push(CONFIG_FILE);
		if path.exists() {
			return Some(path);
		}
		if !path.pop() || !path.pop() {
			eprintln!("error: Could not find `{}` in `{}` or any parent directory", CONFIG_FILE, current_dir.display());
			return None;
		}
	}
}

// The subcommands

mod build;
