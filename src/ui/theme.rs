use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeKind {
    WinBoxDark,
    NordSlate,
    HighContrast,
}

impl ThemeKind {
    pub fn name(&self) -> &'static str {
        match self {
            ThemeKind::WinBoxDark => "WinBox Dark (Padrão)",
            ThemeKind::NordSlate => "Nord Slate",
            ThemeKind::HighContrast => "Alto Contraste",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            ThemeKind::WinBoxDark => ThemeKind::NordSlate,
            ThemeKind::NordSlate => ThemeKind::HighContrast,
            ThemeKind::HighContrast => ThemeKind::WinBoxDark,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub kind: ThemeKind,
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
    pub safe_mode_active: Style,
    pub safe_mode_inactive: Style,
    pub read_only_badge: Style,
}

impl Theme {
    pub fn from_kind(kind: ThemeKind) -> Self {
        match kind {
            ThemeKind::WinBoxDark => Self::winbox_dark(),
            ThemeKind::NordSlate => Self::nord_slate(),
            ThemeKind::HighContrast => Self::high_contrast(),
        }
    }

    pub fn winbox_dark() -> Self {
        Self {
            kind: ThemeKind::WinBoxDark,
            border: Style::default().fg(Color::Rgb(70, 82, 100)),
            border_focus: Style::default().fg(Color::Rgb(90, 165, 230)),
            title: Style::default().fg(Color::Rgb(105, 190, 245)).add_modifier(Modifier::BOLD),
            header_cell: Style::default().fg(Color::Rgb(220, 228, 235)).bg(Color::Rgb(32, 42, 58)).add_modifier(Modifier::BOLD),
            selected_row: Style::default().fg(Color::White).bg(Color::Rgb(45, 90, 150)).add_modifier(Modifier::BOLD),
            normal_text: Style::default().fg(Color::Rgb(210, 218, 225)),
            muted_text: Style::default().fg(Color::Rgb(125, 138, 155)),
            accent: Style::default().fg(Color::Rgb(90, 180, 235)),
            success: Style::default().fg(Color::Rgb(70, 190, 120)),
            warning: Style::default().fg(Color::Rgb(235, 180, 70)),
            danger: Style::default().fg(Color::Rgb(230, 80, 80)),
            safe_mode_active: Style::default().fg(Color::Black).bg(Color::Rgb(70, 190, 120)).add_modifier(Modifier::BOLD),
            safe_mode_inactive: Style::default().fg(Color::White).bg(Color::Rgb(210, 60, 60)).add_modifier(Modifier::BOLD),
            read_only_badge: Style::default().fg(Color::White).bg(Color::Rgb(50, 110, 180)).add_modifier(Modifier::BOLD),
        }
    }

    pub fn nord_slate() -> Self {
        Self {
            kind: ThemeKind::NordSlate,
            border: Style::default().fg(Color::Rgb(76, 86, 106)),
            border_focus: Style::default().fg(Color::Rgb(136, 192, 208)),
            title: Style::default().fg(Color::Rgb(143, 188, 187)).add_modifier(Modifier::BOLD),
            header_cell: Style::default().fg(Color::Rgb(236, 239, 244)).bg(Color::Rgb(59, 69, 89)).add_modifier(Modifier::BOLD),
            selected_row: Style::default().fg(Color::Rgb(236, 239, 244)).bg(Color::Rgb(67, 76, 94)).add_modifier(Modifier::BOLD),
            normal_text: Style::default().fg(Color::Rgb(216, 222, 233)),
            muted_text: Style::default().fg(Color::Rgb(129, 161, 193)),
            accent: Style::default().fg(Color::Rgb(136, 192, 208)),
            success: Style::default().fg(Color::Rgb(163, 190, 140)),
            warning: Style::default().fg(Color::Rgb(235, 203, 139)),
            danger: Style::default().fg(Color::Rgb(191, 97, 106)),
            safe_mode_active: Style::default().fg(Color::Rgb(46, 52, 64)).bg(Color::Rgb(163, 190, 140)).add_modifier(Modifier::BOLD),
            safe_mode_inactive: Style::default().fg(Color::Rgb(236, 239, 244)).bg(Color::Rgb(191, 97, 106)).add_modifier(Modifier::BOLD),
            read_only_badge: Style::default().fg(Color::Rgb(236, 239, 244)).bg(Color::Rgb(94, 129, 172)).add_modifier(Modifier::BOLD),
        }
    }

    pub fn high_contrast() -> Self {
        Self {
            kind: ThemeKind::HighContrast,
            border: Style::default().fg(Color::White),
            border_focus: Style::default().fg(Color::Yellow),
            title: Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            header_cell: Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD),
            selected_row: Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD),
            normal_text: Style::default().fg(Color::White),
            muted_text: Style::default().fg(Color::Gray),
            accent: Style::default().fg(Color::Yellow),
            success: Style::default().fg(Color::Green),
            warning: Style::default().fg(Color::Yellow),
            danger: Style::default().fg(Color::Red),
            safe_mode_active: Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD),
            safe_mode_inactive: Style::default().fg(Color::White).bg(Color::Red).add_modifier(Modifier::BOLD),
            read_only_badge: Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD),
        }
    }
}
