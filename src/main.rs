mod build;
mod config;
mod open;
mod serve;
mod log;

use config::Config;

fn with_config(log: &mut log::Logger, action: impl FnOnce(&mut log::Logger, &Config)) {
	match config::load(log) {
		Ok(config) => action(log, &config),
		Err(_) => (),
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
			.about("Opens the HTML file in a local browser")
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


	let success = match matches.subcommand() {
		Some(("build", _build_matches)) => {
			let mut log = log::Logger::new();
			with_config(&mut log, |log, config| build::main(log, config));
			log.finished()
		},
		Some(("open", _open_matches)) => {
			let mut log = log::Logger::new();
			with_config(&mut log, |log, config| {
				build::main(log, config);
				open::main(log, config);
			});
			log.finished()
		},
		Some(("serve", serve_matches)) => {
			let mut log = log::Logger::new();
			with_config(&mut log, |log, config| {
				build::main(log, config);
				let detached = serve_matches.get_flag("detached");
				serve::main(log, config, detached);
			});
			log.finished()
		},
		Some((command, _)) => unreachable!("Unknown command: {}", command),
		None => {
			println!("Welcome!");
			std::process::exit(0);
		},
	};

	let code = if success { 0 } else { 1 };
	std::process::exit(code)
}
