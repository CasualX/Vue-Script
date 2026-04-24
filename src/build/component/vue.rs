use super::*;

fn join_component_path(base_path: &str, relative_path: &str) -> Option<String> {
	let mut parts: Vec<&str> = base_path.split('/').filter(|part| !part.is_empty()).collect();

	for part in relative_path.split('/') {
		match part {
			"" | "." => (),
			".." => {
				if parts.pop().is_none() {
					return None;
				}
			},
			_ => parts.push(part),
		}
	}

	Some(parts.join("/"))
}

fn parse_component_link(log: &mut log::Logger, component_path: &str, source: &str, component_base_path: &str, element: &tagsoup::Element) -> Option<String> {
	let Some(rel_attr) = element.get_attribute("rel") else {
		log.log(Some(source), log::LogEntry {
			level: log::LogLevel::Error,
			span: Some(log_span(component_path, source, element.span)),
			message: format!("Top-level <link> in component \"{component_path}\" is missing a rel attribute."),
			note: Some("Add rel=\"component\" to the link element."),
		});
		return None;
	};
	let Some(rel_value) = rel_attr.value.as_ref() else {
		log.log(Some(source), log::LogEntry {
			level: log::LogLevel::Error,
			span: Some(log_span(component_path, source, rel_attr.span)),
			message: format!("Top-level <link> in component \"{component_path}\" has an empty rel attribute."),
			note: Some("Set rel=\"component\" on the link element."),
		});
		return None;
	};

	let rel = rel_value.value_raw();
	if rel != "component" {
		log.log(Some(source), log::LogEntry {
			level: log::LogLevel::Error,
			span: Some(log_span(component_path, source, rel_value.span)),
			message: format!("Top-level <link> in component \"{component_path}\" must use rel=\"component\"."),
			note: Some("Remove the link or change it to <link rel=\"component\" href=\"...\">."),
		});
		return None;
	}

	let Some(href_attr) = element.get_attribute("href") else {
		log.log(Some(source), log::LogEntry {
			level: log::LogLevel::Error,
			span: Some(log_span(component_path, source, element.span)),
			message: format!("Component import in \"{component_path}\" is missing an href value."),
			note: Some("Add href=\"...\" to the component import link."),
		});
		return None;
	};

	let Some(href_value) = href_attr.value.as_ref() else {
		log.log(Some(source), log::LogEntry {
			level: log::LogLevel::Error,
			span: Some(log_span(component_path, source, href_attr.span)),
			message: format!("Component import in \"{component_path}\" has an empty href value."),
			note: Some("Set href=\"...\" on the component import link."),
		});
		return None;
	};

	let Some(href) = join_component_path(component_base_path, &href_value.value_raw()) else {
		log.log(Some(source), log::LogEntry {
			level: log::LogLevel::Error,
			span: Some(log_span(component_path, source, href_value.span)),
			message: format!("Component import in \"{component_path}\" resolves outside the project root."),
			note: Some("Use a relative href that stays within the project directory."),
		});
		return None;
	};

	Some(href)
}

fn collect_used_custom_tags(element: &tagsoup::Element, used: &mut HashMap<String, tagsoup::Span>) {
	if element.tag.contains('-') {
		used.entry(element.tag.to_string()).or_insert(element.tag_span);
	}

	for child in &element.children {
		if let tagsoup::Node::Element(child) = child {
			collect_used_custom_tags(child, used);
		}
	}
}

pub fn parse(log: &mut log::Logger, component_path: &str, source: &str) -> Option<Component> {
	let component_base_path = component_path.rfind("/").map_or("", |index| &component_path[..index]);

	let path = component_path.to_string();
	let mut links = Vec::new();
	let mut imports = Vec::new();
	let mut custom_tag = None;
	let mut used_custom_tags = Vec::new();
	let mut script = None;
	let mut template = None;
	let mut style = None;

	let html = tagsoup::Document::parse(source);
	for error in &html.errors {
		log.log(Some(source), log::LogEntry {
			level: log::LogLevel::Warn,
			span: Some(log_span(component_path, source, error.span)),
			message: format!("Malformed HTML in component \"{}\": {}.", component_path, error.kind.as_str()),
			note: Some("The component may still be processed, but consider fixing the HTML syntax."),
		});
	}

	for node in &html.children {
		match node {
			tagsoup::Node::Text(text_node) => {
				if text_node.text.trim_ascii().is_empty() {
					continue;
				}

				log.log(Some(source), log::LogEntry {
					level: log::LogLevel::Error,
					span: Some(log_span(component_path, source, text_node.span)),
					message: format!("Component \"{}\" has top-level text content outside of elements.", component_path),
					note: Some("Consider moving any top-level text into the template or removing it."),
				});
			}
			tagsoup::Node::Doctype(doctype_node) => {
				log.log(Some(source), log::LogEntry {
					level: log::LogLevel::Error,
					span: Some(log_span(component_path, source, doctype_node.span)),
					message: format!("Component \"{}\" must not include a doctype declaration.", component_path),
					note: Some("Remove the doctype declaration from the component file."),
				});
			}
			tagsoup::Node::ProcessingInstruction(pi_node) => {
				log.log(Some(source), log::LogEntry {
					level: log::LogLevel::Error,
					span: Some(log_span(component_path, source, pi_node.span)),
					message: format!("Component \"{}\" must not include processing instructions.", component_path),
					note: Some("Remove any processing instructions from the component file."),
				});
			}
			tagsoup::Node::Comment(_) => (),
			tagsoup::Node::Element(el) => {
				if el.tag.eq_ignore_ascii_case("LINK") {
					if let Some(href) = parse_component_link(log, component_path, source, component_base_path, el) {
						links.push(href);
					}
				}
				else if el.tag.eq_ignore_ascii_case("SCRIPT") {
					if script.is_some() {
						log.log(Some(source), log::LogEntry {
							level: log::LogLevel::Error,
							span: Some(log_span(component_path, source, el.span)),
							message: format!("Component \"{}\" has more than one top-level <script> element.", component_path),
							note: Some("Keep a single top-level <script> block in each component."),
						});
					}
					else {
						let script_contents = el.text_content();
						let (script_imports, script_contents) = js::get_imports(&script_contents);
						imports = script_imports;
						script = Some(script_contents);
					}
				}
				else if el.tag.eq_ignore_ascii_case("STYLE") {
					if style.is_some() {
						log.log(Some(source), log::LogEntry {
							level: log::LogLevel::Error,
							span: Some(log_span(component_path, source, el.span)),
							message: format!("Component \"{}\" has more than one top-level <style> element.", component_path),
							note: Some("Keep a single top-level <style> block in each component."),
						});
					}
					else {
						style = Some(el.text_content());
					}
				}
				else if el.tag.eq_ignore_ascii_case("TEMPLATE") || el.tag.eq_ignore_ascii_case("DIV") {
					if template.is_some() {
						log.log(Some(source), log::LogEntry {
							level: log::LogLevel::Error,
							span: Some(log_span(component_path, source, el.span)),
							message: format!("Component \"{}\" has more than one top-level template root.", component_path),
							note: Some("Use a single top-level <template> or <div> element."),
						});
					}
					else {
						if let Some(id) = el.id {
							custom_tag = Some(id.to_ascii_lowercase());
						}
						else {
							log.log(Some(source), log::LogEntry {
								level: log::LogLevel::Warn,
								span: Some(log_span(component_path, source, el.tag_span)),
								message: format!("Component \"{}\" is missing an id on its top-level <{}> element.", component_path, el.tag),
								note: Some("Set the top-level template or div id to the component's custom tag name."),
							});
						}

						let mut used = HashMap::new();
						collect_used_custom_tags(el, &mut used);
						used_custom_tags = used.into_iter().map(|(tag, span)| UsedCustomTag { tag, span }).collect();
						used_custom_tags.sort_by(|left, right| left.tag.cmp(&right.tag));
						template = Some(outer_html(source, el.span).to_string());
					}
				}
				else {
					log.log(Some(source), log::LogEntry {
						level: log::LogLevel::Error,
						span: Some(log_span(component_path, source, el.span)),
						message: format!("Component \"{}\" has unsupported top-level <{}> content.", component_path, el.tag),
						note: Some("Use only top-level <link>, <script>, <template>/<div>, and <style> elements."),
					});
				}
			}
		}
	}

	Some(Component {
		path,
		source: source.to_string(),
		links,
		imports,
		custom_tag,
		used_custom_tags,
		template,
		script,
		style,
	})
}
