use super::*;

#[test]
fn parses_repeated_watch_entries_and_ignores_target_output() {
	let mut log = crate::log::Logger::new();
	let config = parse_contents(
		&mut log,
		PathBuf::from("vue-script.toml"),
		"[app]\nmain = \"app/main.vue\"\n[target]\npath = \"target/index.html\"\n[serve]\nwatch = \"app/**/*\"\nwatch = \"skills/*.md\"\n",
	)
	.expect("config should parse");

	assert_eq!(config.serve.watch.len(), 4);
	assert_eq!(config.serve.watch[0].pattern.as_str(), "app/**/*");
	assert!(config.serve.watch[0].include);
	assert!(config.serve.watch[0].explicit);
	assert_eq!(config.serve.watch[1].pattern.as_str(), "skills/*.md");
	assert!(config.serve.watch[1].include);
	assert!(config.serve.watch[1].explicit);
	assert_eq!(config.serve.watch[2].pattern.as_str(), CONFIG_FILE);
	assert!(config.serve.watch[2].include);
	assert!(!config.serve.watch[2].explicit);
	assert_eq!(config.serve.watch[3].pattern.as_str(), "target/index.html");
	assert!(!config.serve.watch[3].include);
	assert!(!config.serve.watch[3].explicit);
	assert_eq!(config.serve.explicit_watch_count(), 2);
	assert!(config.serve.explicit_watch_count() > 0);

	assert!(config.serve.matches_relative_path(Path::new("app/main.vue")));
	assert!(config.serve.matches_relative_path(Path::new("skills/vue-script.md")));
	assert!(config.serve.matches_relative_path(Path::new(CONFIG_FILE)));
	assert!(!config.serve.matches_relative_path(Path::new("target/index.html")));
	assert!(!config.serve.matches_relative_path(Path::new("readme.md")));
}

#[test]
fn target_ignore_rule_does_not_count_as_explicit_watch() {
	let mut log = crate::log::Logger::new();
	let config = parse_contents(
		&mut log,
		PathBuf::from("vue-script.toml"),
		"[target]\npath = \"target/index.html\"\n",
	)
	.expect("config should parse");

	assert_eq!(config.serve.explicit_watch_count(), 0);
	assert!(config.serve.explicit_watch_count() == 0);
	assert!(config.serve.matches_relative_path(Path::new(CONFIG_FILE)));
}

#[test]
fn rejects_invalid_watch_patterns() {
	let mut log = crate::log::Logger::new();
	let err = match parse_contents(
		&mut log,
		PathBuf::from("vue-script.toml"),
		"[serve]\nwatch = \"[\"\n",
	) {
		Ok(_) => panic!("invalid watch glob should fail"),
		Err(err) => err,
	};

	assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
	assert!(log.has_errors());
}
