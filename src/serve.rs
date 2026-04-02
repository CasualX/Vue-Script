use crate::{config, open};

use std::io;
use std::path::Path;
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

const SERVER_PORT: u16 = 8000;

fn target_url(config: &config::Config) -> io::Result<String> {
	let target_path = config.target.path.as_ref().ok_or_else(|| io::Error::other("No target path specified in config"))?;
	let path = Path::new(target_path)
		.iter()
		.map(|component| component.to_string_lossy())
		.collect::<Vec<_>>()
		.join("/");
	Ok(format!("http://127.0.0.1:{}/{}", SERVER_PORT, path))
}

fn spawn_server(root: &Path) -> io::Result<Child> {
	let port = SERVER_PORT.to_string();

	for (program, args) in [
		("python3", &["-m", "http.server", port.as_str()] as &[&str]),
		("python", &["-m", "http.server", port.as_str()] as &[&str]),
		("py", &["-3", "-m", "http.server", port.as_str()] as &[&str]),
	] {
		match Command::new(program).args(args).current_dir(root).spawn() {
			Ok(child) => return Ok(child),
			Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
			Err(err) => return Err(err),
		}
	}

	Err(io::Error::new(io::ErrorKind::NotFound, "No supported Python interpreter found"))
}

pub fn run(config: &config::Config, detached: bool) -> io::Result<()> {
	let root = config.path.parent().ok_or_else(|| io::Error::other("Configuration file parent directory not found"))?;
	let target_url = target_url(config)?;
	let mut server = spawn_server(root)?;

	thread::sleep(Duration::from_millis(250));
	if let Err(err) = open::url(&target_url) {
		eprintln!("Error opening URL \"{}\": {}", target_url, err);
	}

	if detached {
		println!("Server started in the background at {}", target_url);
		println!("The Python process will keep running until you stop it manually.");
		println!("Recommendation: for development, prefer running `vue-script serve` in a separate terminal.");
	}
	else {
		println!("Serving {}", target_url);
		println!("Press Ctrl+C to stop the Python server.");
		server.wait()?;
	}
	Ok(())
}

pub fn main(config: &config::Config, detached: bool) {
	if let Err(err) = run(config, detached) {
		eprintln!("Error running server: {}", err);
	}
}
