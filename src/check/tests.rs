use super::*;

fn compilation_with_check(check: crate::config::ConfigCheck) -> build::Compilation {
	build::Compilation {
		config: crate::config::Config {
			path: path::PathBuf::from("/work/project/vue-script.toml"),
			app: crate::config::ConfigApp {
				page: "app/page.html".to_string(),
				main: "app/main.vue".to_string(),
			},
			target: crate::config::ConfigTarget { path: Some("public/index.html".to_string()) },
			check,
			serve: crate::config::ConfigServe::default(),
		},
		javascript: build::MappedJavaScript { source: String::new(), mappings: Vec::new() },
		output: String::new(),
	}
}

#[test]
fn resolves_configured_typescript_paths_from_the_config_file() {
	let config_path = path::Path::new("/work/project/vue-script.toml");
	#[cfg(windows)]
	assert_eq!(typescript_command(config_path, None), path::PathBuf::from("tsc.cmd"));
	#[cfg(not(windows))]
	assert_eq!(typescript_command(config_path, None), path::PathBuf::from("tsc"));

	assert_eq!(
		typescript_command(config_path, Some("node_modules/.bin/tsc")),
		path::PathBuf::from("/work/project/node_modules/.bin/tsc"),
	);

	let absolute = if cfg!(windows) { path::Path::new("C:\\tools\\tsc.cmd") } else { path::Path::new("/tools/tsc") };
	assert_eq!(typescript_command(config_path, absolute.to_str()), absolute);
}

#[test]
fn parses_typescript_diagnostics_with_and_without_locations() {
	assert_eq!(
		parse_typescript_diagnostic("/tmp/check.js(12,7): error TS2322: Type 'number' is not assignable to type 'string'."),
		Some(TypeScriptDiagnostic {
			file: Some("/tmp/check.js".to_string()),
			line: Some(12),
			column: Some(7),
			level: log::LogLevel::Error,
			code: Some("TS2322".to_string()),
			message: "Type 'number' is not assignable to type 'string'.".to_string(),
		})
	);
	assert_eq!(
		parse_typescript_diagnostic("error TS5058: The specified path does not exist."),
		Some(TypeScriptDiagnostic {
			file: None,
			line: None,
			column: None,
			level: log::LogLevel::Error,
			code: Some("TS5058".to_string()),
			message: "The specified path does not exist.".to_string(),
		})
	);
}

#[test]
fn parses_paths_containing_parentheses() {
	let diagnostic = parse_typescript_diagnostic("/tmp/my (project)/check.js(2,4): warning TS1234: Message").unwrap();
	assert_eq!(diagnostic.file.as_deref(), Some("/tmp/my (project)/check.js"));
	assert_eq!(diagnostic.line, Some(2));
	assert_eq!(diagnostic.column, Some(4));
}

#[test]
fn resolves_relative_typescript_diagnostic_paths_from_the_checker_directory() {
	let displayed = display_source_path(
		path::Path::new("feedback.js"),
		path::Path::new("/work/project"),
		path::Path::new("/work/project/public"),
	);

	assert_eq!(displayed, path::PathBuf::from("public/feedback.js"));
}

#[test]
fn requires_an_explicit_typescript_target() {
	let compilation = compilation_with_check(crate::config::ConfigCheck::default());
	let mut log = log::Logger::with_format(log::MessageFormat::Json);

	assert!(check_typescript(&mut log, &compilation).is_err());
	assert_eq!(log.error_count(), 1);
}

#[test]
fn checker_prelude_models_vue_instance_globals_and_constructor_props() {
	assert!(PRELUDE.contains("$el: any;"));
	assert!(PRELUDE.contains("$refs: Record<string, any>;"));
	assert!(PRELUDE.contains("$nextTick(callback?: () => void): Promise<void>;"));
	assert!(PRELUDE.contains("abstract new (...args: any[]) => infer I"));
}
