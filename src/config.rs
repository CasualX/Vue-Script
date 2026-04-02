use std::{env, io};
use std::path::PathBuf;

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

pub fn load() -> io::Result<Config> {
	let path = match config_path() {
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

	let contents = std::fs::read_to_string(&path)?;
	let mut current_section = None;

	for line in ini_core::Parser::new(&contents).auto_trim(true) {
		match line {
			ini_core::Item::Section(section) => {
				current_section = Some(section);
				if section != "app" && section != "target" {
					eprintln!("warn: Unknown config section: [{}]", section);
				}
			}
			ini_core::Item::Property(key, Some(value)) => {
				match current_section {
					Some("app") => match key {
						"page" => app.page = parse_str(&value),
						"main" => app.main = parse_str(&value),
						_ => eprintln!("warn: Unknown config key in [app]: {}", key),
					}
					Some("target") => match key {
						"path" => target.path = Some(parse_str(&value)),
						_ => eprintln!("warn: Unknown config key in [target]: {}", key),
					}
					_ => (),
				}

			}
			_ => (),
		}
	}

	Ok(Config { path, app, target })
}

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
