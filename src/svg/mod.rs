//! SVG module — parser and renderer for SVG images.
//!
//! Falco can render SVG from:
//! - `<img src="*.svg">` — loaded and rasterized
//! - `data:image/svg+xml;base64,...` — inline data URLs
//! - Inline `<svg>...</svg>` in HTML (coming soon)

pub mod parser;
pub mod renderer;

use parser::SvgDocument;
use crate::paint::Canvas;
use crate::css::Color;

/// Parse an SVG XML string into an SvgDocument.
pub fn parse_svg(xml: &str) -> Option<SvgDocument> {
    // Use the HTML parser to tokenize, then extract SVG elements.
    // This is a simplified parser — it uses regex-like string scanning.
    let lower = xml.to_lowercase();
    if !lower.contains("<svg") { return None; }

    // Extract svg root attributes.
    let svg_start = lower.find("<svg")?;
    let svg_tag_end = lower[svg_start..].find('>')?;
    let svg_tag = &xml[svg_start..svg_start + svg_tag_end];
    
    let width = extract_attr_f32(svg_tag, "width").unwrap_or(300.0);
    let height = extract_attr_f32(svg_tag, "height").unwrap_or(150.0);
    let viewBox = extract_attr_str(svg_tag, "viewbox")
        .and_then(|s| {
            let parts: Vec<f32> = s.split(|c: char| c.is_whitespace() || c == ',')
                .filter_map(|x| x.parse().ok())
                .collect();
            if parts.len() == 4 { Some((parts[0], parts[1], parts[2], parts[3])) } else { None }
        });

    // Extract content between <svg> and </svg>.
    let content_start = svg_start + svg_tag_end + 1;
    let content_end = lower[content_start..].find("</svg>")?;
    let content = &xml[content_start..content_start + content_end];

    // Parse child elements.
    let children = parse_svg_children(content);

    Some(SvgDocument { width, height, viewBox, children })
}

fn parse_svg_children(content: &str) -> Vec<parser::SvgElement> {
    let mut children = Vec::new();
    let lower = content.to_lowercase();
    let mut pos = 0;
    
    while pos < content.len() {
        // Find next tag.
        let tag_start = match lower[pos..].find('<') {
            Some(i) => pos + i,
            None => break,
        };
        // Skip closing tags and comments.
        if content[tag_start..].starts_with("</") || content[tag_start..].starts_with("<!--") {
            // Skip to '>'.
            if let Some(end) = lower[tag_start..].find('>') {
                pos = tag_start + end + 1;
            } else { break; }
            continue;
        }
        // Find tag name.
        let tag_end = match lower[tag_start..].find('>') {
            Some(i) => tag_start + i,
            None => break,
        };
        let tag_content = &content[tag_start + 1..tag_end];
        let tag_name = tag_content.split_whitespace().next().unwrap_or("").to_lowercase();
        
        // Parse attributes.
        let attrs = parse_attrs(tag_content);
        
        // Find content (for self-closing tags, no content).
        let self_closing = tag_content.ends_with('/');
        let element = match tag_name.as_str() {
            "rect" => {
                let r = parser::SvgRect {
                    x: extract_attr_f32(tag_content, "x").unwrap_or(0.0),
                    y: extract_attr_f32(tag_content, "y").unwrap_or(0.0),
                    width: extract_attr_f32(tag_content, "width").unwrap_or(0.0),
                    height: extract_attr_f32(tag_content, "height").unwrap_or(0.0),
                    rx: extract_attr_f32(tag_content, "rx").unwrap_or(0.0),
                    ry: extract_attr_f32(tag_content, "ry").unwrap_or(0.0),
                    style: parser::SvgStyle::from_attrs(&attrs),
                };
                Some(parser::SvgElement::Rect(r))
            }
            "circle" => {
                let c = parser::SvgCircle {
                    cx: extract_attr_f32(tag_content, "cx").unwrap_or(0.0),
                    cy: extract_attr_f32(tag_content, "cy").unwrap_or(0.0),
                    r: extract_attr_f32(tag_content, "r").unwrap_or(0.0),
                    style: parser::SvgStyle::from_attrs(&attrs),
                };
                Some(parser::SvgElement::Circle(c))
            }
            "ellipse" => {
                let e = parser::SvgEllipse {
                    cx: extract_attr_f32(tag_content, "cx").unwrap_or(0.0),
                    cy: extract_attr_f32(tag_content, "cy").unwrap_or(0.0),
                    rx: extract_attr_f32(tag_content, "rx").unwrap_or(0.0),
                    ry: extract_attr_f32(tag_content, "ry").unwrap_or(0.0),
                    style: parser::SvgStyle::from_attrs(&attrs),
                };
                Some(parser::SvgElement::Ellipse(e))
            }
            "line" => {
                let l = parser::SvgLine {
                    x1: extract_attr_f32(tag_content, "x1").unwrap_or(0.0),
                    y1: extract_attr_f32(tag_content, "y1").unwrap_or(0.0),
                    x2: extract_attr_f32(tag_content, "x2").unwrap_or(0.0),
                    y2: extract_attr_f32(tag_content, "y2").unwrap_or(0.0),
                    style: parser::SvgStyle::from_attrs(&attrs),
                };
                Some(parser::SvgElement::Line(l))
            }
            "polyline" => {
                let pts_str = extract_attr_str(tag_content, "points").unwrap_or_default();
                Some(parser::SvgElement::Polyline(parser::SvgPolyline {
                    points: parser::parse_points(&pts_str),
                    style: parser::SvgStyle::from_attrs(&attrs),
                }))
            }
            "polygon" => {
                let pts_str = extract_attr_str(tag_content, "points").unwrap_or_default();
                Some(parser::SvgElement::Polygon(parser::SvgPolygon {
                    points: parser::parse_points(&pts_str),
                    style: parser::SvgStyle::from_attrs(&attrs),
                }))
            }
            "path" => {
                let d = extract_attr_str(tag_content, "d").unwrap_or_default();
                Some(parser::SvgElement::Path(parser::SvgPath {
                    commands: parser::parse_path_d(&d),
                    style: parser::SvgStyle::from_attrs(&attrs),
                }))
            }
            "text" => {
                // Extract text content between <text> and </text>.
                let text_content = if self_closing {
                    String::new()
                } else {
                    let close_tag = format!("</{}>", tag_name);
                    if let Some(end) = lower[tag_end + 1..].find(&close_tag.to_lowercase()) {
                        content[tag_end + 1..tag_end + 1 + end].trim().to_string()
                    } else { String::new() }
                };
                Some(parser::SvgElement::Text(parser::SvgText {
                    x: extract_attr_f32(tag_content, "x").unwrap_or(0.0),
                    y: extract_attr_f32(tag_content, "y").unwrap_or(0.0),
                    text: text_content,
                    font_size: extract_attr_f32(tag_content, "font-size").unwrap_or(16.0),
                    font_weight: extract_attr_str(tag_content, "font-weight").and_then(|s| s.parse().ok()).unwrap_or(400),
                    style: parser::SvgStyle::from_attrs(&attrs),
                }))
            }
            "g" => {
                // Parse group children.
                let transform_str = extract_attr_str(tag_content, "transform");
                let transform = transform_str.as_deref().and_then(parser::parse_transform);
                let group_content = if self_closing {
                    ""
                } else {
                    let close_tag = "</g>";
                    if let Some(end) = lower[tag_end + 1..].find(close_tag) {
                        &content[tag_end + 1..tag_end + 1 + end]
                    } else { "" }
                };
                let group_children = parse_svg_children(group_content);
                Some(parser::SvgElement::Group(parser::SvgGroup {
                    transform,
                    style: parser::SvgStyle::from_attrs(&attrs),
                    children: group_children,
                }))
            }
            _ => None,
        };
        
        if let Some(e) = element {
            children.push(e);
        }
        
        // Advance position.
        if self_closing {
            pos = tag_end + 1;
        } else {
            // Find closing tag for this element.
            let close_tag = format!("</{}>", tag_name);
            if let Some(end) = lower[tag_end + 1..].find(&close_tag.to_lowercase()) {
                pos = tag_end + 1 + end + close_tag.len();
            } else {
                pos = tag_end + 1;
            }
        }
    }
    
    children
}

fn parse_attrs(tag_content: &str) -> std::collections::HashMap<String, String> {
    let mut attrs = std::collections::HashMap::new();
    // Simple attribute parser: key="value" or key='value'
    let mut chars = tag_content.chars().peekable();
    let mut key = String::new();
    let mut in_value = false;
    let mut quote = '"';
    let mut value = String::new();
    let mut reading_key = false;
    
    while let Some(c) = chars.next() {
        if !in_value {
            if c == '=' && !key.is_empty() {
                // Start of value.
                if let Some(&q) = chars.peek() {
                    if q == '"' || q == '\'' {
                        quote = q;
                        chars.next();
                        in_value = true;
                        value.clear();
                    }
                }
            } else if c.is_whitespace() {
                if reading_key && !key.is_empty() {
                    // Boolean attribute (no value).
                    attrs.insert(key.clone(), String::new());
                    key.clear();
                    reading_key = false;
                }
            } else {
                if !reading_key {
                    key.clear();
                    reading_key = true;
                }
                // Skip the tag name (first word).
                if !key.is_empty() || c != ' ' {
                    key.push(c);
                }
            }
        } else {
            if c == quote {
                attrs.insert(key.trim().to_lowercase(), value.clone());
                key.clear();
                in_value = false;
                reading_key = false;
            } else {
                value.push(c);
            }
        }
    }
    
    // Remove the tag name from attrs (it gets added as the first "attribute").
    // The first entry will be like "rect" with empty value — remove it.
    let tag_name = tag_content.split_whitespace().next().unwrap_or("");
    attrs.remove(tag_name);
    
    attrs
}

fn extract_attr_f32(tag: &str, name: &str) -> Option<f32> {
    extract_attr_str(tag, name)?.trim_end_matches("px").parse().ok()
}

fn extract_attr_str(tag: &str, name: &str) -> Option<String> {
    let pattern = format!("{}=\"", name);
    if let Some(start) = tag.to_lowercase().find(&pattern.to_lowercase()) {
        let rest = &tag[start + pattern.len()..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    let pattern = format!("{}='", name);
    if let Some(start) = tag.to_lowercase().find(&pattern.to_lowercase()) {
        let rest = &tag[start + pattern.len()..];
        if let Some(end) = rest.find('\'') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

/// Render an SVG string onto a canvas.
pub fn render_svg_to_canvas(svg_xml: &str, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32) -> bool {
    if let Some(doc) = parse_svg(svg_xml) {
        renderer::render_svg(canvas, &doc, x, y, w, h);
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_svg() {
        let svg = r##"<svg width="100" height="100" viewBox="0 0 100 100">
            <rect x="10" y="10" width="80" height="80" fill="#ff0000" stroke="#000000" stroke-width="2"/>
            <circle cx="50" cy="50" r="30" fill="#0000ff"/>
        </svg>"##;
        let doc = parse_svg(svg);
        assert!(doc.is_some());
        let doc = doc.unwrap();
        assert_eq!(doc.width, 100.0);
        assert_eq!(doc.height, 100.0);
        assert_eq!(doc.children.len(), 2);
    }

    #[test]
    fn parses_path() {
        let svg = r##"<svg width="100" height="100"><path d="M 10 10 L 50 50 Z" fill="red"/></svg>"##;
        let doc = parse_svg(svg).unwrap();
        assert_eq!(doc.children.len(), 1);
    }

    #[test]
    fn parses_group() {
        let svg = r##"<svg width="100" height="100"><g transform="translate(10,20)"><rect width="50" height="50" fill="blue"/></g></svg>"##;
        let doc = parse_svg(svg).unwrap();
        assert_eq!(doc.children.len(), 1);
    }
}
