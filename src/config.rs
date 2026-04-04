use std::{env, io};
use std::path::{Component, Path, PathBuf};

use glob::Pattern;

use crate::log;

// Helpers

pub const CONFIG_FILE: &str = "vue-script.toml";

#[derive(Clone)]
pub struct ConfigApp {
	pub page: String,
	pub main: String,
}

#[derive(Clone)]
pub struct ConfigTarget {
	pub path: Option<String>,
}

#[derive(Clone)]
pub struct ServeWatchRule {
	pattern: Pattern,
	include: bool,
	explicit: bool,
}

impl ServeWatchRule {
	fn new(raw_pattern: &str, include: bool, explicit: bool) -> io::Result<ServeWatchRule> {
		let pattern = Pattern::new(raw_pattern).map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err.msg.to_string()))?;
		Ok(ServeWatchRule { pattern, include, explicit })
	}

	fn matches_relative_path(&self, relative_path: &str) -> bool {
		self.pattern.matches(relative_path)
	}

	fn ignored_target(target_path: &str) -> io::Result<ServeWatchRule> {
		ServeWatchRule::new(&Pattern::escape(target_path), false, false)
	}

	fn included_config_file() -> io::Result<ServeWatchRule> {
		ServeWatchRule::new(&Pattern::escape(CONFIG_FILE), true, false)
	}

	fn invalid_pattern_error(raw_pattern: &str, err: io::Error) -> io::Error {
		io::Error::new(io::ErrorKind::InvalidInput, format!("Invalid [serve].watch pattern \"{}\": {}", raw_pattern, err))
	}

	fn parse_include(raw_pattern: &str) -> io::Result<ServeWatchRule> {
		ServeWatchRule::new(raw_pattern, true, true).map_err(|err| ServeWatchRule::invalid_pattern_error(raw_pattern, err))
	}

	fn parse_ignored_target(target_path: &str) -> io::Result<ServeWatchRule> {
		ServeWatchRule::ignored_target(target_path).map_err(|err| ServeWatchRule::invalid_pattern_error(target_path, err))
	}

	fn parse_included_config_file() -> io::Result<ServeWatchRule> {
		ServeWatchRule::included_config_file().map_err(|err| ServeWatchRule::invalid_pattern_error(CONFIG_FILE, err))
	}

	fn log_invalid_pattern(log: &mut log::Logger, error: &io::Error) {
		log.log(None, log::LogEntry {
			level: log::LogLevel::Error,
			span: None,
			message: error.to_string(),
			note: Some("Fix the [serve].watch glob pattern in vue-script.toml."),
		});
	}

	fn from_include(log: &mut log::Logger, raw_pattern: &str) -> io::Result<ServeWatchRule> {
		match ServeWatchRule::parse_include(raw_pattern) {
			Ok(rule) => Ok(rule),
			Err(err) => {
				ServeWatchRule::log_invalid_pattern(log, &err);
				Err(err)
			}
		}
	}

	fn from_ignored_target(log: &mut log::Logger, target_path: &str) -> io::Result<ServeWatchRule> {
		match ServeWatchRule::parse_ignored_target(target_path) {
			Ok(rule) => Ok(rule),
			Err(err) => {
				ServeWatchRule::log_invalid_pattern(log, &err);
				Err(err)
			}
		}
	}

	fn from_included_config_file(log: &mut log::Logger) -> io::Result<ServeWatchRule> {
		match ServeWatchRule::parse_included_config_file() {
			Ok(rule) => Ok(rule),
			Err(err) => {
				ServeWatchRule::log_invalid_pattern(log, &err);
				Err(err)
			}
		}
	}
}

#[derive(Clone, Default)]
pub struct ConfigServe {
	pub watch: Vec<ServeWatchRule>,
}

impl ConfigServe {
	pub fn explicit_watch_count(&self) -> usize {
		self.watch.iter().filter(|rule| rule.include && rule.explicit).count()
	}

	pub fn matches_relative_path(&self, path: &Path) -> bool {
		let Some(relative_path) = normalize_relative_path(path) else {
			return false;
		};

		let mut should_rebuild = false;
		for rule in &self.watch {
			if rule.matches_relative_path(&relative_path) {
				should_rebuild = rule.include;
			}
		}
		should_rebuild
	}
}

#[derive(Clone)]
pub struct Config {
	pub path: PathBuf,
	pub app: ConfigApp,
	pub target: ConfigTarget,
	pub serve: ConfigServe,
}

fn parse_str(s: &str) -> String {
	s.trim_matches(|c| c == '"' || c == '\'').to_string()
}

fn normalize_relative_path(path: &Path) -> Option<String> {
	let mut components = Vec::new();
	for component in path.components() {
		match component {
			Component::CurDir => (),
			Component::Normal(component) => components.push(component.to_string_lossy().into_owned()),
			Component::ParentDir => components.push("..".to_string()),
			Component::RootDir | Component::Prefix(_) => return None,
		}
	}
	Some(components.join("/"))
}

fn parse_contents(log: &mut log::Logger, path: PathBuf, contents: &str) -> io::Result<Config> {
	let mut app = ConfigApp {
		page: "app/page.html".to_string(),
		main: "app/main.vue".to_string(),
	};

	let mut target = ConfigTarget {
		path: None,
	};

	let mut serve = ConfigServe::default();
	let mut current_section = None;

	for line in ini_core::Parser::new(contents).auto_trim(true) {
		match line {
			ini_core::Item::Section(section) => {
				current_section = Some(section);
				if section != "app" && section != "target" && section != "serve" {
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
					},
					Some("target") => match key {
						"path" => target.path = Some(parse_str(&value)),
						_ => log.log(None, log::LogEntry {
							level: log::LogLevel::Warn,
							span: None,
							message: format!("Unknown config key [{}].{}.", "target", key),
							note: Some("Remove the key or rename it to a supported setting."),
						}),
					},
					Some("serve") => match key {
						"watch" => {
							let pattern = parse_str(&value);
							serve.watch.push(ServeWatchRule::from_include(log, &pattern)?);
						}
						_ => log.log(None, log::LogEntry {
							level: log::LogLevel::Warn,
							span: None,
							message: format!("Unknown config key [{}].{}.", "serve", key),
							note: Some("Remove the key or rename it to a supported setting."),
						}),
					},
					_ => (),
				}

			}
			_ => (),
		}
	}

	serve.watch.push(ServeWatchRule::from_included_config_file(log)?);

	if let Some(target_path) = &target.path {
		serve.watch.push(ServeWatchRule::from_ignored_target(log, target_path)?);
	}

	Ok(Config { path, app, target, serve })
}

impl Config {
	pub fn load(log: &mut log::Logger) -> io::Result<Config> {
		let path = match config_path(log) {
			Some(path) => path,
			None => return Err(io::Error::new(io::ErrorKind::NotFound, "Configuration file not found")),
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

		parse_contents(log, path, &contents)
	}
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

#[cfg(test)]
mod tests;
