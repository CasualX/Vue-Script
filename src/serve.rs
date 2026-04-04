use std::{io, path, process, sync::mpsc, thread, time};

use super::*;

const REBUILD_DEBOUNCE: time::Duration = time::Duration::from_secs(1);

struct ServeRuntime {
	config: Config,
	project_root: path::PathBuf,
	server_root: path::PathBuf,
	target_url: String,
}

impl ServeRuntime {
	fn load(log: &mut log::Logger, port: u16) -> io::Result<ServeRuntime> {
		let config = Config::load(log)?;
		ServeRuntime::from_config(config, port)
	}

	fn from_config(config: Config, port: u16) -> io::Result<ServeRuntime> {
		let server_root = config
			.path
			.parent()
			.ok_or_else(|| io::Error::other("Configuration file parent directory not found"))?
			.to_path_buf();
		let project_root = config::project_root(&config)
			.ok_or_else(|| io::Error::other("Project root directory could not be resolved"))?;
		let target_url = target_url(&config, port)?;

		Ok(ServeRuntime {
			config,
			project_root,
			server_root,
			target_url,
		})
	}

	fn explicit_watch_count(&self) -> usize {
		self.config.serve.explicit_watch_count()
	}

	fn matches_relative_path(&self, relative_path: &path::Path) -> bool {
		self.config.serve.matches_relative_path(relative_path)
	}
}

fn target_url(config: &Config, port: u16) -> io::Result<String> {
	let target_path = config.target.path.as_ref().ok_or_else(|| io::Error::other("No target output path is configured"))?;

	let mut url = format!("http://127.0.0.1:{port}");
	for component in path::Path::new(target_path).iter() {
		url.push('/');
		url.push_str(&component.to_string_lossy());
	}
	Ok(url)
}

fn spawn_server(root: &path::Path, server_port: u16) -> io::Result<process::Child> {
	let port = server_port.to_string();

	for (program, args) in [
		("python3", &["-m", "http.server", port.as_str()] as &[&str]),
		("python", &["-m", "http.server", port.as_str()] as &[&str]),
		("py", &["-3", "-m", "http.server", port.as_str()] as &[&str]),
	] {
		match process::Command::new(program).args(args).current_dir(root).spawn() {
			Ok(child) => return Ok(child),
			Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
			Err(err) => return Err(err),
		}
	}

	Err(io::Error::new(io::ErrorKind::NotFound, "No supported Python interpreter found"))
}

fn open_target(log: &mut log::Logger, target_url: &str) {
	// Give the server a moment to start before trying to open the URL
	thread::sleep(time::Duration::from_millis(250));
	if let Err(err) = open::url(target_url) {
		log.log(None, log::LogEntry {
			level: log::LogLevel::Warn,
			span: None,
			message: format!("Development server started, but opening \"{}\" in a browser failed: {}", target_url, err),
			note: Some("Open the URL manually in a browser."),
		});
	}
}

fn start_watcher(project_root: &path::Path) -> io::Result<(notify::RecommendedWatcher, mpsc::Receiver<notify::Result<notify::Event>>)> {
	let (tx, rx) = mpsc::channel();
	use notify::Watcher;
	let mut watcher = notify::RecommendedWatcher::new(
		move |event| {
			let _ = tx.send(event);
		},
		notify::Config::default(),
	)
	.map_err(|err| io::Error::other(format!("Failed to create file watcher: {}", err)))?;
	watcher
		.watch(project_root, notify::RecursiveMode::Recursive)
		.map_err(|err| io::Error::other(format!("Failed to watch project directory \"{}\": {}", project_root.display(), err)))?;
	Ok((watcher, rx))
}

fn print_watch_status(runtime: &ServeRuntime) {
	let explicit_watch_count = runtime.explicit_watch_count();
	if explicit_watch_count > 0 {
		println!("Watching {} path pattern(s) for rebuilds.", explicit_watch_count);
	}
	else {
		println!("No [serve].watch entries are configured. Watching {} for config changes only.", config::CONFIG_FILE);
	}
}

fn relevant_event_kind(event: &notify::Event) -> bool {
	!matches!(event.kind, notify::EventKind::Access(_))
}

fn relative_event_path<'a>(project_root: &path::Path, path: &'a path::Path) -> Option<&'a path::Path> {
	if path.is_relative() {
		return Some(path);
	}
	path.strip_prefix(project_root).ok()
}

fn matching_event_path<'a>(runtime: &ServeRuntime, project_root: &path::Path, event: &'a notify::Event) -> Option<&'a path::Path> {
	if !relevant_event_kind(event) {
		return None;
	}

	for path in &event.paths {
		let Some(relative_path) = relative_event_path(project_root, path) else {
			continue;
		};
		if runtime.matches_relative_path(relative_path) {
			return Some(relative_path);
		}
	}

	None
}

fn drain_debounced_path(
	runtime: &ServeRuntime,
	project_root: &path::Path,
	rx: &mpsc::Receiver<notify::Result<notify::Event>>,
	mut matched_path: std::path::PathBuf,
) -> io::Result<std::path::PathBuf> {
	let mut debounce_started = time::Instant::now();

	loop {
		let remaining = REBUILD_DEBOUNCE.saturating_sub(debounce_started.elapsed());
		if remaining.is_zero() {
			return Ok(matched_path);
		}

		match rx.recv_timeout(remaining) {
			Ok(Ok(event)) => {
				if let Some(next_path) = matching_event_path(runtime, project_root, &event) {
					matched_path = next_path.to_path_buf();
					debounce_started = time::Instant::now();
				}
			}
			Ok(Err(err)) => {
				return Err(io::Error::other(format!("File watcher reported an error: {}", err)));
			}
			Err(mpsc::RecvTimeoutError::Timeout) => return Ok(matched_path),
			Err(mpsc::RecvTimeoutError::Disconnected) => {
				return Err(io::Error::other("File watcher disconnected unexpectedly"));
			}
		}
	}
}

fn is_config_path(relative_path: &path::Path) -> bool {
	relative_path == path::Path::new(config::CONFIG_FILE)
}

fn reload_runtime(log: &mut log::Logger, runtime: &mut ServeRuntime, port: u16) {
	let previous_watch_count = runtime.explicit_watch_count();
	let previous_target_url = runtime.target_url.clone();

	match ServeRuntime::load(log, port) {
		Ok(updated_runtime) => {
			let updated_watch_count = updated_runtime.explicit_watch_count();
			let updated_target_url = updated_runtime.target_url.clone();
			*runtime = updated_runtime;

			if updated_watch_count != previous_watch_count {
				print_watch_status(runtime);
			}
			if updated_target_url != previous_target_url {
				println!("Updated target URL to {}", updated_target_url);
			}
		}
		Err(err) => {
			log.log(None, log::LogEntry {
				level: log::LogLevel::Warn,
				span: None,
				message: format!("Keeping previous serve configuration after reload failed: {}", err),
				note: Some("Fix vue-script.toml and save again; the watcher will retry on the next config change."),
			});
		}
	}
}

fn watch_and_rebuild(
	log: &mut log::Logger,
	runtime: &mut ServeRuntime,
	rx: mpsc::Receiver<notify::Result<notify::Event>>,
	server: &mut process::Child,
	port: u16,
) -> io::Result<()> {
	println!("Serving {}", runtime.target_url);
	print_watch_status(runtime);
	println!("Press Ctrl+C to stop the Python server.");

	loop {
		if let Some(status) = server.try_wait()? {
			return Err(io::Error::other(format!("Python server exited with status {}", status)));
		}

		match rx.recv_timeout(time::Duration::from_millis(250)) {
			Ok(Ok(event)) => {
				if let Some(relative_path) = matching_event_path(runtime, &runtime.project_root, &event) {
					let relative_path = drain_debounced_path(runtime, &runtime.project_root, &rx, relative_path.to_path_buf())?;
					println!("Rebuilding after change to {}", relative_path.display());
					if is_config_path(&relative_path) {
						reload_runtime(log, runtime, port);
					}
					build::main(log);
				}
			}
			Ok(Err(err)) => {
				log.log(None, log::LogEntry {
					level: log::LogLevel::Warn,
					span: None,
					message: format!("File watcher reported an error: {}", err),
					note: Some("Check filesystem permissions and watch paths, then save again to retry."),
				});
			}
			Err(mpsc::RecvTimeoutError::Timeout) => (),
			Err(mpsc::RecvTimeoutError::Disconnected) => {
				return Err(io::Error::other("File watcher disconnected unexpectedly"));
			}
		}
	}
}

fn run(log: &mut log::Logger, detached: bool, port: u16) -> io::Result<()> {
	let mut runtime = ServeRuntime::load(log, port)?;

	if detached {
		let mut server = spawn_server(&runtime.server_root, port)?;
		open_target(log, &runtime.target_url);
		println!("Server started in the background at {}", runtime.target_url);
		println!("The Python process will keep running until you stop it manually.");
		println!("Recommendation: for development, prefer running `vue-script serve` in a separate terminal.");
		let _ = &mut server;
	}
	else {
		let (_watcher, rx) = start_watcher(&runtime.project_root)?;
		let mut server = spawn_server(&runtime.server_root, port)?;
		open_target(log, &runtime.target_url);
		watch_and_rebuild(log, &mut runtime, rx, &mut server, port)?;
	}
	Ok(())
}

pub fn main(log: &mut log::Logger, detached: bool, port: u16) {
	if let Err(err) = run(log, detached, port) {
		log.log(None, log::LogEntry {
			level: log::LogLevel::Error,
			span: None,
			message: format!("Failed to start the development server: {}", err),
			note: Some("Ensure Python is installed, the selected port is available, [target].path is configured, and the project directory is accessible."),
		});
	}
}
