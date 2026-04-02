mod build;
mod config;
mod open;
mod serve;

fn with_config(action: impl FnOnce(&config::Config)) {
	match config::load() {
		Ok(config) => action(&config),
		Err(e) => eprintln!("Error loading config: {}", e),
	}
}

fn main() {
	let app = clap::Command::new("Vue Script")
		.version(clap::crate_version!())
		.author("Casper <CasualX@users.noreply.github.com>")
		.about("Vue Single File Components without the insanity that comes with the NPM ecosystem")
		.subcommand(clap::Command::new("build")
			.about("Compiles the Vue Single File Components for distribution")
		)
		.subcommand(clap::Command::new("open")
			.about("Opens the html in a local browser")
		)
		.subcommand(clap::Command::new("serve")
			.about("Serves the project and opens the target in a local browser")
			.arg(clap::Arg::new("detached")
				.long("detached")
				.action(clap::ArgAction::SetTrue)
				.help("Start the server and return immediately, leaving the process running")
			)
		);
	let matches = app.get_matches();

	match matches.subcommand() {
		Some(("build", _build_matches)) => {
			with_config(|config| build::main(config));
		},
		Some(("open", _open_matches)) => {
			with_config(|config| {
				build::main(config);
				open::main(config);
			});
		},
		Some(("serve", serve_matches)) => {
			with_config(|config| {
				build::main(config);
				let detached = serve_matches.get_flag("detached");
				serve::main(config, detached);
			});
		},
		Some((command, _)) => unreachable!("Unknown command: {}", command),
		None => println!("Welcome!"),
	}
}
