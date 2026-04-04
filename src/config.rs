use std::{env, io};
use std::path::PathBuf;

use crate::log;

// Helpers

const CONFIG_FILE: &str = "vue-script.toml";

pub struct ConfigApp {
	pub page: String,
	pub main: String,
}

pub struct ConfigTarget {
	pub path: Option<String>,
}

pub struct Config {
	pub path: PathBuf,
	pub app: ConfigApp,
	pub target: ConfigTarget,
}

fn parse_str(s: &str) -> String {
	s.trim_matches(|c| c == '"' || c == '\'').to_string()
}

pub fn load(log: &mut log::Logger) -> io::Result<Config> {
	let path = match config_path(log) {
		Some(path) => path,
		None => return Err(io::Error::new(io::ErrorKind::NotFound, "Configuration file not found")),
	};

	let mut app = ConfigApp {
		page: "app/page.html".to_string(),
		main: "app/main.vue".to_string(),
	};

	let mut target = ConfigTarget {
		path: None,
	};

	let contents = match std::fs::read_to_string(&path) {
		Ok(contents) => contents,
		Err(err) => {
			log.log(None, log::LogEntry {
				level: log::LogLevel::Error,
				span: None,
				message: format!("Failed to read config \"{}\": {}", path.display(), err),
				note: Some("Check that vue-script.toml exists and is readable."),
			});
			return Err(err);
		}
	};
	let mut current_section = None;

	for line in ini_core::Parser::new(&contents).auto_trim(true) {
		match line {
			ini_core::Item::Section(section) => {
				current_section = Some(section);
				if section != "app" && section != "target" {
					log.log(None, log::LogEntry {
						level: log::LogLevel::Warn,
						span: None,
						message: format!("Unknown config section [{}].", section),
						note: Some("Remove the section or rename it to a supported section."),
					});
				}
			}
			ini_core::Item::Property(key, Some(value)) => {
				match current_section {
					Some("app") => match key {
						"page" => app.page = parse_str(&value),
						"main" => app.main = parse_str(&value),
						_ => log.log(None, log::LogEntry {
							level: log::LogLevel::Warn,
							span: None,
							message: format!("Unknown config key [{}].{}.", "app", key),
							note: Some("Remove the key or rename it to a supported setting."),
						}),
					}
					Some("target") => match key {
						"path" => target.path = Some(parse_str(&value)),
						_ => log.log(None, log::LogEntry {
							level: log::LogLevel::Warn,
							span: None,
							message: format!("Unknown config key [{}].{}.", "target", key),
							note: Some("Remove the key or rename it to a supported setting."),
						}),
					}
					_ => (),
				}

			}
			_ => (),
		}
	}

	Ok(Config { path, app, target })
}

fn config_path(log: &mut log::Logger) -> Option<PathBuf> {
	let current_dir = match env::current_dir() {
		Ok(path) => path,
		Err(err) => {
			log.log(None, log::LogEntry {
				level: log::LogLevel::Error,
				span: None,
				message: format!("Current working directory is unavailable: {}", err),
				note: Some("Run vue-script from inside the project directory when possible."),
			});
			return None;
		},
	};

	let mut path = current_dir.clone();
	loop {
		path.push(CONFIG_FILE);
		if path.exists() {
			return Some(path);
		}
		if !path.pop() || !path.pop() {
			log.log(None, log::LogEntry {
				level: log::LogLevel::Error,
				span: None,
				message: format!("Could not find {} in {} or any parent directory.", CONFIG_FILE, current_dir.display()),
				note: Some("Create vue-script.toml in the project root or run the command from inside the project."),
			});
			return None;
		}
	}
}

pub fn project_root(config: &Config) -> Option<PathBuf> {
	let mut project_file = config.path.canonicalize().ok()?;
	project_file.pop();
	Some(project_file)
}

pub fn target_full_path(config: &Config) -> Option<PathBuf> {
	let project_root = project_root(config)?;
	let target_path = config.target.path.as_ref()?;
	Some(project_root.join(target_path))
}
