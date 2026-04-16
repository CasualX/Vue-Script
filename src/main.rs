mod build;
mod config;
mod open;
mod serve;
mod log;

use config::Config;

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
			.arg(clap::Arg::new("port")
				.long("port")
				.value_name("PORT")
				.value_parser(clap::value_parser!(u16))
				.default_value("8000")
				.help("Port for the Python HTTP server")
			)
			.arg(clap::Arg::new("detached")
				.long("detached")
				.action(clap::ArgAction::SetTrue)
				.help("Start the server and return immediately, leaving the process running")
			)
		);

	let success = match app.get_matches().subcommand() {
		Some(("build", _matches)) => {
			let mut log = log::Logger::new();
			build::main(&mut log);
			log.finished()
		},
		Some(("open", _matches)) => {
			let mut log = log::Logger::new();
			build::main(&mut log);
			open::main(&mut log);
			log.finished()
		},
		Some(("serve", matches)) => {
			let mut log = log::Logger::new();
			build::main(&mut log);
			let detached = matches.get_flag("detached");
			let port = *matches.get_one::<u16>("port").expect("serve port should have a default value");
			serve::main(&mut log, detached, port);
			log.finished()
		},
		Some((command, _)) => unreachable!("Unknown command: {}", command),
		None => {
			println!("Welcome!");
			true
		},
	};

	let code = if success { 0 } else { 1 };
	std::process::exit(code)
}
