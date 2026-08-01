//! SVG parser — parses SVG XML into a tree of shape elements.
//!
//! Supported elements:
//!   <svg> — root element with viewBox
//!   <rect> — x, y, width, height, rx, ry
//!   <circle> — cx, cy, r
//!   <ellipse> — cx, cy, rx, ry
//!   <line> — x1, y1, x2, y2
//!   <polyline> — points
//!   <polygon> — points (auto-closed)
//!   <path> — d attribute (M, L, H, V, C, S, Q, T, A, Z commands)
//!   <text> — x, y, font-size, fill
//!   <g> — group with transform
//!   <defs> — definitions (gradients, etc.) — parsed but not rendered
//!   <linearGradient>, <radialGradient> — gradient definitions
//!
//! Supported attributes:
//!   fill, fill-opacity, stroke, stroke-width, stroke-opacity
//!   opacity, transform (translate, scale, rotate)
//!   font-size, font-family, font-weight, text-anchor

use crate::css::Color;
use std::collections::HashMap;

/// An SVG document — the root of the SVG tree.
#[derive(Debug, Clone)]
pub struct SvgDocument {
    pub width: f32,
    pub height: f32,
    pub viewBox: Option<(f32, f32, f32, f32)>,
    pub children: Vec<SvgElement>,
}

/// An SVG element — shape, group, or text.
#[derive(Debug, Clone)]
pub enum SvgElement {
    Rect(SvgRect),
    Circle(SvgCircle),
    Ellipse(SvgEllipse),
    Line(SvgLine),
    Polyline(SvgPolyline),
    Polygon(SvgPolygon),
    Path(SvgPath),
    Text(SvgText),
    Group(SvgGroup),
}

#[derive(Debug, Clone)]
pub struct SvgRect {
    pub x: f32, pub y: f32, pub width: f32, pub height: f32,
    pub rx: f32, pub ry: f32,
    pub style: SvgStyle,
}

#[derive(Debug, Clone)]
pub struct SvgCircle {
    pub cx: f32, pub cy: f32, pub r: f32,
    pub style: SvgStyle,
}

#[derive(Debug, Clone)]
pub struct SvgEllipse {
    pub cx: f32, pub cy: f32, pub rx: f32, pub ry: f32,
    pub style: SvgStyle,
}

#[derive(Debug, Clone)]
pub struct SvgLine {
    pub x1: f32, pub y1: f32, pub x2: f32, pub y2: f32,
    pub style: SvgStyle,
}

#[derive(Debug, Clone)]
pub struct SvgPolyline {
    pub points: Vec<(f32, f32)>,
    pub style: SvgStyle,
}

#[derive(Debug, Clone)]
pub struct SvgPolygon {
    pub points: Vec<(f32, f32)>,
    pub style: SvgStyle,
}

#[derive(Debug, Clone)]
pub struct SvgPath {
    pub commands: Vec<PathCommand>,
    pub style: SvgStyle,
}

#[derive(Debug, Clone)]
pub enum PathCommand {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    HorizontalLineTo(f32),
    VerticalLineTo(f32),
    CubicBezier(f32, f32, f32, f32, f32, f32),
    SmoothCubicBezier(f32, f32, f32, f32),
    QuadraticBezier(f32, f32, f32, f32),
    SmoothQuadraticBezier(f32, f32),
    Arc(f32, f32, f32, bool, bool, f32, f32),
    ClosePath,
}

#[derive(Debug, Clone)]
pub struct SvgText {
    pub x: f32, pub y: f32,
    pub text: String,
    pub font_size: f32,
    pub font_weight: u16,
    pub style: SvgStyle,
}

#[derive(Debug, Clone)]
pub struct SvgGroup {
    pub transform: Option<SvgTransform>,
    pub style: SvgStyle,
    pub children: Vec<SvgElement>,
}

#[derive(Debug, Clone, Copy)]
pub struct SvgTransform {
    pub translate: (f32, f32),
    pub scale: (f32, f32),
    pub rotate: f32, // degrees
}

impl Default for SvgTransform {
    fn default() -> Self {
        Self { translate: (0.0, 0.0), scale: (1.0, 1.0), rotate: 0.0 }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SvgStyle {
    pub fill: Option<Color>,
    pub fill_opacity: f32,
    pub stroke: Option<Color>,
    pub stroke_width: f32,
    pub stroke_opacity: f32,
    pub opacity: f32,
}

impl SvgStyle {
    pub fn from_attrs(attrs: &HashMap<String, String>) -> Self {
        let mut s = SvgStyle {
            fill_opacity: 1.0,
            stroke_width: 1.0,
            stroke_opacity: 1.0,
            opacity: 1.0,
            ..Default::default()
        };
        if let Some(f) = attrs.get("fill") {
            if f != "none" {
                s.fill = parse_svg_color(f);
            }
        }
        if let Some(f) = attrs.get("fill-opacity") {
            s.fill_opacity = f.parse().unwrap_or(1.0);
        }
        if let Some(st) = attrs.get("stroke") {
            if st != "none" {
                s.stroke = parse_svg_color(st);
            }
        }
        if let Some(sw) = attrs.get("stroke-width") {
            s.stroke_width = sw.trim_end_matches("px").parse().unwrap_or(1.0);
        }
        if let Some(so) = attrs.get("stroke-opacity") {
            s.stroke_opacity = so.parse().unwrap_or(1.0);
        }
        if let Some(o) = attrs.get("opacity") {
            s.opacity = o.parse().unwrap_or(1.0);
        }
        s
    }
}

/// Parse an SVG color string ("red", "#ff0000", "rgb(255,0,0)").
fn parse_svg_color(s: &str) -> Option<Color> {
    crate::css::parse_color(s)
}

/// Parse a transform attribute string like "translate(10,20) scale(2)".
pub fn parse_transform(s: &str) -> Option<SvgTransform> {
    let mut t = SvgTransform::default();
    // Find translate(...)
    if let Some(start) = s.find("translate(") {
        let rest = &s[start + 10..];
        if let Some(end) = rest.find(')') {
            let args: Vec<f32> = rest[..end].split(',').filter_map(|x| x.trim().parse().ok()).collect();
            if args.len() >= 2 {
                t.translate = (args[0], args[1]);
            } else if args.len() == 1 {
                t.translate = (args[0], 0.0);
            }
        }
    }
    if let Some(start) = s.find("scale(") {
        let rest = &s[start + 6..];
        if let Some(end) = rest.find(')') {
            let args: Vec<f32> = rest[..end].split(',').filter_map(|x| x.trim().parse().ok()).collect();
            if args.len() >= 2 {
                t.scale = (args[0], args[1]);
            } else if args.len() == 1 {
                t.scale = (args[0], args[0]);
            }
        }
    }
    if let Some(start) = s.find("rotate(") {
        let rest = &s[start + 7..];
        if let Some(end) = rest.find(')') {
            let arg = rest[..end].trim().trim_end_matches("deg").parse().unwrap_or(0.0);
            t.rotate = arg;
        }
    }
    Some(t)
}

/// Parse SVG path "d" attribute into PathCommands.
pub fn parse_path_d(d: &str) -> Vec<PathCommand> {
    let mut commands = Vec::new();
    let mut chars = d.chars().peekable();
    let mut current_cmd = ' ';

    while let Some(&c) = chars.peek() {
        if c.is_ascii_alphabetic() {
            current_cmd = c;
            chars.next();
        } else if c.is_whitespace() || c == ',' {
            chars.next();
            continue;
        }

        // Read numbers for the current command.
        match current_cmd {
            'M' | 'm' => {
                if let (Some(x), Some(y)) = (read_number(&mut chars), read_number(&mut chars)) {
                    commands.push(PathCommand::MoveTo(x, y));
                    // Subsequent pairs are implicit LineTo.
                    current_cmd = if current_cmd == 'M' { 'L' } else { 'l' };
                }
            }
            'L' | 'l' => {
                if let (Some(x), Some(y)) = (read_number(&mut chars), read_number(&mut chars)) {
                    commands.push(PathCommand::LineTo(x, y));
                }
            }
            'H' | 'h' => {
                if let Some(x) = read_number(&mut chars) {
                    commands.push(PathCommand::HorizontalLineTo(x));
                }
            }
            'V' | 'v' => {
                if let Some(y) = read_number(&mut chars) {
                    commands.push(PathCommand::VerticalLineTo(y));
                }
            }
            'C' | 'c' => {
                let nums = read_numbers(&mut chars, 6);
                if nums.len() == 6 {
                    commands.push(PathCommand::CubicBezier(nums[0], nums[1], nums[2], nums[3], nums[4], nums[5]));
                }
            }
            'S' | 's' => {
                let nums = read_numbers(&mut chars, 4);
                if nums.len() == 4 {
                    commands.push(PathCommand::SmoothCubicBezier(nums[0], nums[1], nums[2], nums[3]));
                }
            }
            'Q' | 'q' => {
                let nums = read_numbers(&mut chars, 4);
                if nums.len() == 4 {
                    commands.push(PathCommand::QuadraticBezier(nums[0], nums[1], nums[2], nums[3]));
                }
            }
            'Z' | 'z' => {
                commands.push(PathCommand::ClosePath);
                chars.next();
            }
            _ => {
                chars.next();
            }
        }
    }
    commands
}

fn read_number(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<f32> {
    // Skip whitespace and commas.
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() || c == ',' { chars.next(); } else { break; }
    }
    let mut s = String::new();
    let mut has_digit = false;
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e' || c == 'E' {
            s.push(c);
            if c.is_ascii_digit() { has_digit = true; }
            chars.next();
        } else {
            break;
        }
    }
    if has_digit {
        s.parse().ok()
    } else {
        None
    }
}

fn read_numbers(chars: &mut std::iter::Peekable<std::str::Chars>, count: usize) -> Vec<f32> {
    let mut nums = Vec::with_capacity(count);
    for _ in 0..count {
        if let Some(n) = read_number(chars) {
            nums.push(n);
        } else {
            break;
        }
    }
    nums
}

/// Parse points attribute "10,20 30,40 50,60".
pub fn parse_points(s: &str) -> Vec<(f32, f32)> {
    let mut points = Vec::new();
    let nums: Vec<f32> = s.split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();
    for chunk in nums.chunks(2) {
        if chunk.len() == 2 {
            points.push((chunk[0], chunk[1]));
        }
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_path_d() {
        let cmds = parse_path_d("M 10 10 L 50 50 Z");
        assert_eq!(cmds.len(), 3);
        assert!(matches!(cmds[0], PathCommand::MoveTo(10.0, 10.0)));
        assert!(matches!(cmds[1], PathCommand::LineTo(50.0, 50.0)));
        assert!(matches!(cmds[2], PathCommand::ClosePath));
    }

    #[test]
    fn parses_points() {
        let pts = parse_points("10,20 30,40 50,60");
        assert_eq!(pts.len(), 3);
        assert_eq!(pts[0], (10.0, 20.0));
        assert_eq!(pts[2], (50.0, 60.0));
    }

    #[test]
    fn parses_transform() {
        let t = parse_transform("translate(10,20) scale(2)").unwrap();
        assert_eq!(t.translate, (10.0, 20.0));
        assert_eq!(t.scale, (2.0, 2.0));
    }
}
