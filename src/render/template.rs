use super::UiTheme;
use include_dir::{include_dir, Dir};
use minijinja::Environment;
use serde::Serialize;
use std::error::Error;

static TEMPLATE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");
static ICONS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/icons");

fn load_icon_svg(icon_name: &str) -> Option<&'static str> {
    ICONS_DIR
        .get_file(icon_name)
        .and_then(|f| f.contents_utf8())
        .or_else(|| ICONS_DIR.get_file("unknown.svg").and_then(|f| f.contents_utf8()))
}

fn extract_attr(opening_tag: &str, attr_name: &str) -> Option<String> {
    let needle = format!("{attr_name}=\"");
    let start = opening_tag.find(&needle)? + needle.len();
    let rest = &opening_tag[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}

fn strip_attr_value(mut input: String, attr_name: &str) -> String {
    let needle = format!(" {attr_name}=\"");
    loop {
        let Some(start) = input.find(&needle) else {
            break;
        };
        let value_start = start + needle.len();
        let Some(value_end_rel) = input[value_start..].find('"') else {
            break;
        };
        let value_end = value_start + value_end_rel;
        input.replace_range(start..=value_end, "");
    }
    input
}

fn normalized_icon_parts(icon_name: &str) -> Option<(String, String)> {
    let raw_svg = load_icon_svg(icon_name)?;

    let svg_start = raw_svg.find("<svg")?;
    let open_end_rel = raw_svg[svg_start..].find('>')?;

    let open_end = svg_start + open_end_rel;
    let opening_tag = &raw_svg[svg_start..=open_end];
    let closing_idx = raw_svg.rfind("</svg>").unwrap_or(raw_svg.len());
    let inner = raw_svg[open_end + 1..closing_idx].trim();
    let inner = strip_attr_value(inner.to_owned(), "sketch:type");
    let view_box = extract_attr(opening_tag, "viewBox").unwrap_or_else(|| "0 0 128 128".to_owned());

    Some((inner, view_box))
}

pub fn icon_markup(icon_name: &str, x: i32, y: i32, width: i32, height: i32) -> String {
    let Some((inner, view_box)) = normalized_icon_parts(icon_name) else {
        return String::new();
    };

    format!(
        "<svg x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" viewBox=\"{view_box}\" preserveAspectRatio=\"xMidYMid meet\">{inner}</svg>"
    )
}

pub fn icon_markup_unsized(icon_name: &str) -> String {
    let Some((inner, view_box)) = normalized_icon_parts(icon_name) else {
        return String::new();
    };

    // Canonical icon size; placement and final sizing should be done in templates.
    format!(
        "<svg width=\"128\" height=\"128\" viewBox=\"{view_box}\" preserveAspectRatio=\"xMidYMid meet\">{inner}</svg>"
    )
}

pub fn render_template_from_ctx<S: Serialize>(template_name: &str, ctx: S) -> Result<Vec<u8>, Box<dyn Error>> {

    let mut env = Environment::new();
    // Register all .svg.j2 templates in the directory
    for file in TEMPLATE_DIR.files() {
        if let Some(path) = file.path().file_name() {
            if let Some(name) = path.to_str() {
                if name.ends_with(".svg.j2") {
                    if let Some(contents) = file.contents_utf8() {
                        env.add_template(name, contents)?;
                    }
                }
            }
        }
    }
    // Register custom truncate filter
    env.add_filter("truncate", |s: String, length: usize, _: bool, end_str: String| {
        if s.len() > length {
            format!("{}{}", &s[..length], end_str)
        } else {
            s
        }
    });
    let template = env.get_template(template_name)?;

    let rendered = template.render(ctx)?;

    return Ok(rendered.into_bytes());
}
