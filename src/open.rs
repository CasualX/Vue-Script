use std::ffi::OsStr;
use std::io;
use std::path::Path;
use std::process::Command;

use super::*;

fn target(target: &OsStr) -> io::Result<()> {
	#[cfg(target_os = "windows")]
	{
		let status = Command::new("cmd")
			.args(["/C", "start", ""])
			.arg(target)
			.spawn()?
			.wait()?;
		if !status.success() {
			return Err(io::Error::other(format!("Opener exited with status {}", status)));
		}
		return Ok(());
	}

	#[cfg(target_os = "macos")]
	{
		let status = Command::new("open").arg(target).spawn()?.wait()?;
		if !status.success() {
			return Err(io::Error::other(format!("Opener exited with status {}", status)));
		}
		return Ok(());
	}

	#[cfg(all(unix, not(target_os = "macos")))]
	{
		for (program, args) in [
			("xdg-open", &[] as &[&str]),
			("gio", &["open"] as &[&str]),
			("gnome-open", &[] as &[&str]),
			("kde-open", &[] as &[&str]),
			("wslview", &[] as &[&str]),
		] {
			match Command::new(program).args(args).arg(target).spawn() {
				Ok(mut child) => {
					let status = child.wait()?;
					if !status.success() {
						return Err(io::Error::other(format!("Opener exited with status {}", status)));
					}
					return Ok(());
				},
				Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
				Err(err) => return Err(err),
			}
		}

		return Err(io::Error::new(io::ErrorKind::NotFound, "No supported opener command found"));
	}

	#[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
	{
		let _ = target;
		Err(io::Error::new(io::ErrorKind::Unsupported, "Opening files is not supported on this platform"))
	}
}

pub fn path(path: &Path) -> io::Result<()> {
	target(path.as_os_str())
}

pub fn url(url: &str) -> io::Result<()> {
	target(OsStr::new(url))
}

pub fn main(log: &mut log::Logger, config: &Config) {
	// Open the target file in the browser (no serve, just open the file:// URL)
	match config::target_full_path(config) {
		Some(target_full_path) => {
			if let Err(err) = path(&target_full_path) {
				log.log(None, log::LogEntry {
					level: log::LogLevel::Error,
					span: None,
					message: format!("Failed to open generated target \"{}\" in a browser: {}", target_full_path.display(), err),
					note: Some("Open the generated file manually or configure a compatible opener on this system."),
				});
			}
		},
		None => log.log(None, log::LogEntry {
			level: log::LogLevel::Error,
			span: None,
			message: "No target output path is configured.".to_string(),
			note: Some("Set [target].path in vue-script.toml before using the open command."),
		}),
	}
}
