//! Colour themes.
//!
//! MikroTUI ships one hand-built theme, WinBox Dark, and 238 base16 dark schemes bundled
//! from [tinted-theming/schemes]. Users can drop further base16 files into
//! `~/.config/mikrotui/themes/` and they appear alongside them.
//!
//! Only dark variants are bundled. Several elements — the badges, `muted_text` — assume a
//! dark background, and shipping light schemes that look wrong would be worse than not
//! shipping them.
//!
//! [tinted-theming/schemes]: https://github.com/tinted-theming/schemes

pub mod base16;

use ratatui::style::{Color, Modifier, Style};
use std::path::PathBuf;

use base16::Scheme;

/// The bundled schemes, vendored at build time. See `themes/CREDITS.md`.
const BUNDLED: &str = include_str!("../../../themes/base16-dark.yaml");

/// Identifies a theme across runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeId {
    /// The built-in default, not derived from a base16 palette.
    WinBoxDark,
    /// A base16 scheme, bundled or loaded from disk, by slug.
    Base16(String),
}

impl ThemeId {
    pub fn slug(&self) -> &str {
        match self {
            ThemeId::WinBoxDark => "winbox-dark",
            ThemeId::Base16(slug) => slug,
        }
    }

    pub fn from_slug(slug: &str) -> Self {
        if slug == "winbox-dark" {
            ThemeId::WinBoxDark
        } else {
            ThemeId::Base16(slug.to_string())
        }
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub id: ThemeId,
    pub name: String,
    pub border: Style,
    pub border_focus: Style,
    pub title: Style,
    pub header_cell: Style,
    pub selected_row: Style,
    pub normal_text: Style,
    pub muted_text: Style,
    pub accent: Style,
    pub success: Style,
    pub warning: Style,
    pub danger: Style,
    pub host_key_verified: Style,
    pub host_key_unverified: Style,
    pub read_only_badge: Style,
}

impl Theme {
    /// The default, kept hand-tuned rather than derived: it is the look the project is
    /// named after, and a generated palette would not reproduce it.
    pub fn winbox_dark() -> Self {
        Self {
            id: ThemeId::WinBoxDark,
            name: "WinBox Dark".to_string(),
            border: Style::default().fg(Color::Rgb(70, 82, 100)),
            border_focus: Style::default().fg(Color::Rgb(90, 165, 230)),
            title: Style::default()
                .fg(Color::Rgb(105, 190, 245))
                .add_modifier(Modifier::BOLD),
            header_cell: Style::default()
                .fg(Color::Rgb(220, 228, 235))
                .bg(Color::Rgb(32, 42, 58))
                .add_modifier(Modifier::BOLD),
            selected_row: Style::default()
                .fg(Color::White)
                .bg(Color::Rgb(45, 90, 150))
                .add_modifier(Modifier::BOLD),
            normal_text: Style::default().fg(Color::Rgb(210, 218, 225)),
            muted_text: Style::default().fg(Color::Rgb(125, 138, 155)),
            accent: Style::default().fg(Color::Rgb(90, 180, 235)),
            success: Style::default().fg(Color::Rgb(70, 190, 120)),
            warning: Style::default().fg(Color::Rgb(235, 180, 70)),
            danger: Style::default().fg(Color::Rgb(230, 80, 80)),
            host_key_verified: Style::default()
                .fg(Color::Black)
                .bg(Color::Rgb(70, 190, 120))
                .add_modifier(Modifier::BOLD),
            host_key_unverified: Style::default()
                .fg(Color::White)
                .bg(Color::Rgb(210, 60, 60))
                .add_modifier(Modifier::BOLD),
            read_only_badge: Style::default()
                .fg(Color::White)
                .bg(Color::Rgb(50, 110, 180))
                .add_modifier(Modifier::BOLD),
        }
    }

    /// Build a theme from a base16 palette.
    ///
    /// The slots have fixed meanings, so this mapping is not arbitrary: base08 is the red
    /// slot in every scheme, base0B the green, base02 the one the spec calls "Selection
    /// Background". Roles that carry meaning — success, danger — take the colour the spec
    /// assigns to that meaning, so a scheme cannot make an error look calm.
    pub fn from_scheme(scheme: &Scheme) -> Self {
        let p = scheme.palette;
        let (bg, surface, selection, comment) = (p[0], p[1], p[2], p[3]);
        let (text, red, yellow) = (p[5], p[8], p[10]);
        let (green, cyan, blue, magenta) = (p[11], p[12], p[13], p[14]);

        Self {
            id: ThemeId::Base16(scheme.slug.clone()),
            name: scheme.name.clone(),
            border: Style::default().fg(comment),
            border_focus: Style::default().fg(blue),
            title: Style::default().fg(cyan).add_modifier(Modifier::BOLD),
            header_cell: Style::default()
                .fg(text)
                .bg(surface)
                .add_modifier(Modifier::BOLD),
            selected_row: Style::default()
                .fg(text)
                .bg(selection)
                .add_modifier(Modifier::BOLD),
            normal_text: Style::default().fg(text),
            muted_text: Style::default().fg(comment),
            accent: Style::default().fg(blue),
            success: Style::default().fg(green),
            warning: Style::default().fg(yellow),
            danger: Style::default().fg(red),
            // Badges invert: the background carries the state, so the foreground takes
            // the scheme's own background rather than a guessed black or white.
            host_key_verified: Style::default()
                .fg(bg)
                .bg(green)
                .add_modifier(Modifier::BOLD),
            host_key_unverified: Style::default().fg(bg).bg(red).add_modifier(Modifier::BOLD),
            read_only_badge: Style::default()
                .fg(bg)
                .bg(magenta)
                .add_modifier(Modifier::BOLD),
        }
    }
}

/// A theme the user can pick, without the cost of building its styles.
#[derive(Debug, Clone)]
pub struct Entry {
    pub id: ThemeId,
    pub name: String,
    pub author: String,
    /// Where it came from, for the picker to show.
    pub source: Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    BuiltIn,
    Bundled,
    UserFile,
}

impl Source {
    pub fn label(&self) -> &'static str {
        match self {
            Source::BuiltIn => "built-in",
            Source::Bundled => "base16",
            Source::UserFile => "custom",
        }
    }
}

/// Where user-supplied schemes are read from.
pub fn user_theme_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("mikrotui").join("themes"))
}

/// Every theme available, in display order: the default first, then everything else by
/// name so the picker's list is stable between runs.
pub fn catalogue() -> Vec<Entry> {
    let mut entries = vec![Entry {
        id: ThemeId::WinBoxDark,
        name: "WinBox Dark".to_string(),
        author: "MikroTUI".to_string(),
        source: Source::BuiltIn,
    }];

    let mut rest: Vec<Entry> = schemes()
        .into_iter()
        .map(|(scheme, source)| Entry {
            id: ThemeId::Base16(scheme.slug),
            name: scheme.name,
            author: scheme.author,
            source,
        })
        .collect();
    rest.sort_by_key(|e| e.name.to_lowercase());

    entries.extend(rest);
    entries
}

/// Bundled schemes plus anything in the user's theme directory.
///
/// A user file with the same slug as a bundled scheme replaces it, so a scheme can be
/// corrected locally without waiting for a release.
fn schemes() -> Vec<(Scheme, Source)> {
    let mut found: Vec<(Scheme, Source)> = base16::parse_bundle(BUNDLED)
        .unwrap_or_default()
        .into_iter()
        .map(|s| (s, Source::Bundled))
        .collect();

    for (scheme, source) in user_schemes() {
        match found.iter().position(|(s, _)| s.slug == scheme.slug) {
            Some(index) => found[index] = (scheme, source),
            None => found.push((scheme, source)),
        }
    }

    found
}

/// Read `~/.config/mikrotui/themes/*.yaml`.
///
/// Unreadable or malformed files are skipped rather than fatal: a broken theme file
/// should not stop the tool from starting.
fn user_schemes() -> Vec<(Scheme, Source)> {
    let Some(dir) = user_theme_dir() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };

    let mut schemes = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let is_yaml = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e == "yaml" || e == "yml");
        if !is_yaml {
            continue;
        }
        let Some(slug) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        if let Ok(scheme) = base16::parse(slug, &text) {
            // Light schemes are not bundled and are not accepted from disk either: the
            // badges and muted text assume a dark background.
            if scheme.is_dark() {
                schemes.push((scheme, Source::UserFile));
            }
        }
    }
    schemes
}

/// Build the theme for `id`, falling back to the default if it no longer exists.
pub fn load(id: &ThemeId) -> Theme {
    match id {
        ThemeId::WinBoxDark => Theme::winbox_dark(),
        ThemeId::Base16(slug) => schemes()
            .into_iter()
            .find(|(s, _)| &s.slug == slug)
            .map(|(s, _)| Theme::from_scheme(&s))
            .unwrap_or_else(Theme::winbox_dark),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bundle must parse in full. A scheme that fails here would silently vanish from
    /// the picker, and the corpus is exactly what a hand-written parser needs testing on.
    #[test]
    fn every_bundled_scheme_parses() {
        let schemes = base16::parse_bundle(BUNDLED).expect("the bundle should parse");
        assert!(
            schemes.len() > 200,
            "expected the full bundle, got {}",
            schemes.len()
        );

        for scheme in &schemes {
            assert!(!scheme.slug.is_empty());
            assert!(!scheme.name.is_empty(), "{} has no name", scheme.slug);
            assert!(
                scheme.is_dark(),
                "{} is {:?}, only dark schemes are bundled",
                scheme.slug,
                scheme.variant
            );
            for (index, colour) in scheme.palette.iter().enumerate() {
                assert!(
                    matches!(colour, Color::Rgb(..)),
                    "{} {} did not parse to an RGB colour",
                    scheme.slug,
                    base16::SLOTS[index]
                );
            }
        }
    }

    #[test]
    fn the_famous_schemes_are_present() {
        let slugs: Vec<String> = base16::parse_bundle(BUNDLED)
            .unwrap()
            .into_iter()
            .map(|s| s.slug)
            .collect();

        for expected in [
            "dracula",
            "nord",
            "catppuccin-mocha",
            "gruvbox-dark-medium",
            "tokyo-night-dark",
            "solarized-dark",
            "onedark",
            "rose-pine",
        ] {
            assert!(slugs.contains(&expected.to_string()), "{expected} missing");
        }
    }

    /// Roles that carry meaning take the slot the spec assigns to that meaning, so no
    /// scheme can make an error green.
    #[test]
    fn semantic_roles_follow_the_spec() {
        let scheme = base16::parse_bundle(BUNDLED)
            .unwrap()
            .into_iter()
            .find(|s| s.slug == "nord")
            .expect("nord is bundled");
        let theme = Theme::from_scheme(&scheme);

        assert_eq!(theme.danger.fg, Some(scheme.palette[8]), "danger is base08");
        assert_eq!(theme.warning.fg, Some(scheme.palette[10]), "warning base0A");
        assert_eq!(theme.success.fg, Some(scheme.palette[11]), "success base0B");
        assert_eq!(theme.accent.fg, Some(scheme.palette[13]), "accent base0D");
        assert_eq!(
            theme.selected_row.bg,
            Some(scheme.palette[2]),
            "selection is base02"
        );
    }

    #[test]
    fn the_catalogue_leads_with_the_default() {
        let catalogue = catalogue();
        assert_eq!(catalogue[0].id, ThemeId::WinBoxDark);
        assert_eq!(catalogue[0].source, Source::BuiltIn);
        assert!(catalogue.len() > 200);

        // The rest is sorted, so the list does not reshuffle between runs.
        let names: Vec<String> = catalogue[1..]
            .iter()
            .map(|e| e.name.to_lowercase())
            .collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[test]
    fn an_unknown_slug_falls_back_to_the_default() {
        let theme = load(&ThemeId::Base16("no-such-scheme".to_string()));
        assert_eq!(theme.id, ThemeId::WinBoxDark);
    }

    #[test]
    fn a_slug_round_trips() {
        assert_eq!(ThemeId::from_slug("winbox-dark"), ThemeId::WinBoxDark);
        assert_eq!(
            ThemeId::from_slug("dracula"),
            ThemeId::Base16("dracula".to_string())
        );
        assert_eq!(ThemeId::WinBoxDark.slug(), "winbox-dark");
    }
}
