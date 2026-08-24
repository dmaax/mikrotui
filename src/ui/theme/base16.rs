//! Reading base16 colour schemes.
//!
//! The format is the one used by [tinted-theming/schemes]: a handful of scalar fields and
//! a `palette` map of sixteen hex colours. Parsing it by hand rather than pulling in a
//! YAML crate is a deliberate trade — `serde_yaml` is deprecated and unmaintained, the
//! alternatives are young, and this is seventeen keys in a fixed shape. What it costs is
//! that the parser must be tested against the real corpus rather than a couple of
//! examples: one of the 238 bundled schemes writes `variant: dark` unquoted while the
//! other 237 quote it.
//!
//! [tinted-theming/schemes]: https://github.com/tinted-theming/schemes

use anyhow::{anyhow, Result};
use ratatui::style::Color;

/// The sixteen base16 slots, in order.
///
/// Their meanings are fixed by the [styling guidelines], which is what makes a mapping
/// onto MikroTUI's roles possible at all: base08 is red everywhere, base0B green
/// everywhere.
///
/// [styling guidelines]: https://github.com/tinted-theming/home/blob/main/styling.md
pub const SLOTS: [&str; 16] = [
    "base00", "base01", "base02", "base03", "base04", "base05", "base06", "base07", "base08",
    "base09", "base0A", "base0B", "base0C", "base0D", "base0E", "base0F",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scheme {
    /// Filename stem, e.g. `catppuccin-mocha`. Stable, unlike the display name.
    pub slug: String,
    pub name: String,
    pub author: String,
    pub variant: String,
    pub palette: [Color; 16],
}

impl Scheme {
    pub fn is_dark(&self) -> bool {
        self.variant == "dark"
    }
}

/// Strip a trailing comment, surrounding quotes and whitespace from a scalar.
fn scalar(raw: &str) -> &str {
    let value = raw.trim();
    let value = match value.strip_prefix('"') {
        Some(rest) => rest.split('"').next().unwrap_or(""),
        None => match value.strip_prefix('\'') {
            Some(rest) => rest.split('\'').next().unwrap_or(""),
            // Unquoted: a `#` starts a comment. One bundled scheme relies on this.
            None => value.split('#').next().unwrap_or("").trim(),
        },
    };
    value
}

/// Parse `#RRGGBB`, with or without the leading `#`.
fn hex(value: &str) -> Result<Color> {
    let digits = value.strip_prefix('#').unwrap_or(value);
    if digits.len() != 6 || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(anyhow!("{value:?} is not a #RRGGBB colour"));
    }
    let channel = |from: usize| u8::from_str_radix(&digits[from..from + 2], 16);
    Ok(Color::Rgb(channel(0)?, channel(2)?, channel(4)?))
}

/// Reduce an upstream author field to a name.
///
/// Upstream writes these three ways: a plain name, `Name (https://…)`, and a bare URL.
/// The URL is noise in a picker column, so the first form wins and a bare URL falls back
/// to its last path segment — `https://github.com/catppuccin` becomes `catppuccin`.
fn author_name(raw: &str) -> String {
    let before_url = raw.split(" (").next().unwrap_or("").trim();
    if !before_url.starts_with("http") {
        return before_url.to_string();
    }
    before_url
        .trim_end_matches('/')
        .rsplit('/')
        .find(|segment| !segment.is_empty())
        .unwrap_or(before_url)
        .to_string()
}

/// Parse one scheme document.
pub fn parse(slug: &str, text: &str) -> Result<Scheme> {
    let mut name = String::new();
    let mut author = String::new();
    let mut variant = String::new();
    let mut palette: Vec<Option<Color>> = vec![None; 16];

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        let key = key.trim();

        match key {
            "name" => name = scalar(value).to_string(),
            // Upstream authors are often "Name (https://…)". The URL is not useful in a
            // list and costs the column its width.
            "author" => author = author_name(scalar(value)),
            "variant" => variant = scalar(value).to_string(),
            _ => {
                if let Some(slot) = SLOTS.iter().position(|s| *s == key) {
                    palette[slot] =
                        Some(hex(scalar(value)).map_err(|e| anyhow!("{slug}: {key}: {e}"))?);
                }
            }
        }
    }

    let mut colours = [Color::Reset; 16];
    for (index, slot) in palette.iter().enumerate() {
        colours[index] = slot.ok_or_else(|| anyhow!("{slug}: missing {}", SLOTS[index]))?;
    }

    Ok(Scheme {
        slug: slug.to_string(),
        name: if name.is_empty() {
            slug.to_string()
        } else {
            name
        },
        author,
        variant,
        palette: colours,
    })
}

/// Split the bundled blob into its schemes.
///
/// Each is preceded by a `### slug: <name>` line added at vendoring time; everything
/// after it is the upstream file verbatim.
pub fn parse_bundle(blob: &str) -> Result<Vec<Scheme>> {
    let mut schemes = Vec::new();
    let mut slug: Option<&str> = None;
    let mut body = String::new();

    for line in blob.lines() {
        if let Some(next) = line.strip_prefix("### slug:") {
            if let Some(current) = slug.take() {
                schemes.push(parse(current, &body)?);
            }
            slug = Some(next.trim());
            body.clear();
        } else if slug.is_some() {
            body.push_str(line);
            body.push('\n');
        }
    }
    if let Some(current) = slug {
        schemes.push(parse(current, &body)?);
    }

    Ok(schemes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NORD: &str = r##"
system: "base16"
name: "Nord"
author: "arcticicestudio"
variant: "dark"
palette:
  base00: "#2E3440"
  base01: "#3B4252"
  base02: "#434C5E"
  base03: "#4C566A"
  base04: "#D8DEE9"
  base05: "#E5E9F0"
  base06: "#ECEFF4"
  base07: "#8FBCBB"
  base08: "#BF616A"
  base09: "#D08770"
  base0A: "#EBCB8B"
  base0B: "#A3BE8C"
  base0C: "#88C0D0"
  base0D: "#81A1C1"
  base0E: "#B48EAD"
  base0F: "#5E81AC"
"##;

    #[test]
    fn parses_a_scheme() {
        let scheme = parse("nord", NORD).unwrap();
        assert_eq!(scheme.slug, "nord");
        assert_eq!(scheme.name, "Nord");
        assert_eq!(scheme.author, "arcticicestudio");
        assert!(scheme.is_dark());
        assert_eq!(scheme.palette[0], Color::Rgb(0x2E, 0x34, 0x40));
        assert_eq!(scheme.palette[11], Color::Rgb(0xA3, 0xBE, 0x8C));
        assert_eq!(scheme.palette[15], Color::Rgb(0x5E, 0x81, 0xAC));
    }

    /// One bundled scheme writes `variant: dark` unquoted. A parser that assumed quotes
    /// would classify it as light and drop it.
    #[test]
    fn unquoted_scalars_are_accepted() {
        let text = NORD.replace(r##"variant: "dark""##, "variant: dark");
        assert!(parse("x", &text).unwrap().is_dark());

        let text = NORD.replace(r##"base00: "#2E3440""##, "base00: 2E3440");
        assert_eq!(
            parse("x", &text).unwrap().palette[0],
            Color::Rgb(46, 52, 64)
        );
    }

    /// Upstream author fields often carry a URL, which is noise in a picker column.
    #[test]
    fn a_url_is_stripped_from_the_author() {
        let text = NORD.replace(
            r##"author: "arcticicestudio""##,
            r##"author: "Jannik Siebert (https://github.com/janniks)""##,
        );
        assert_eq!(parse("x", &text).unwrap().author, "Jannik Siebert");
    }

    #[test]
    fn a_bare_url_author_becomes_its_last_segment() {
        assert_eq!(author_name("https://github.com/catppuccin"), "catppuccin");
        assert_eq!(author_name("https://github.com/catppuccin/"), "catppuccin");
        assert_eq!(
            author_name("Jannik Siebert (https://github.com/x)"),
            "Jannik Siebert"
        );
        assert_eq!(author_name("romainl"), "romainl");
        assert_eq!(author_name(""), "");
    }

    #[test]
    fn a_trailing_comment_is_not_part_of_the_value() {
        let text = NORD.replace(r##"variant: "dark""##, "variant: dark # the good one");
        assert_eq!(parse("x", &text).unwrap().variant, "dark");
    }

    #[test]
    fn a_missing_slot_is_an_error_rather_than_a_default_colour() {
        let text = NORD.replace(r##"  base0C: "#88C0D0""##, "  # base0C removed");
        let err = parse("nord", &text).unwrap_err().to_string();
        assert!(err.contains("base0C"), "got: {err}");
    }

    #[test]
    fn a_malformed_colour_is_rejected() {
        for bad in ["#12345", "#GGGGGG", "blue", ""] {
            let text = NORD.replace(r##""#2E3440""##, &format!("{bad:?}"));
            assert!(parse("x", &text).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn a_bundle_splits_into_its_schemes() {
        let blob = format!("### slug: nord\n{NORD}\n### slug: nord-two\n{NORD}");
        let schemes = parse_bundle(&blob).unwrap();
        assert_eq!(schemes.len(), 2);
        assert_eq!(schemes[0].slug, "nord");
        assert_eq!(schemes[1].slug, "nord-two");
    }
}
