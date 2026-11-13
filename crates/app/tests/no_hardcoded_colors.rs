//! Guard test ensuring no hardcoded Color32 or tiny_skia::Color literals exist outside `tokens.rs`.

const APP_SOURCES_EXCEPT_TOKENS: &[(&str, &str)] = &[
    ("src/app.rs", include_str!("../src/app.rs")),
    ("src/canvas.rs", include_str!("../src/canvas.rs")),
    (
        "src/command_palette.rs",
        include_str!("../src/command_palette.rs"),
    ),
    ("src/lib.rs", include_str!("../src/lib.rs")),
    ("src/main.rs", include_str!("../src/main.rs")),
    ("src/screenshot.rs", include_str!("../src/screenshot.rs")),
    ("src/theme.rs", include_str!("../src/theme.rs")),
    ("src/workspace.rs", include_str!("../src/workspace.rs")),
    ("src/panels/mod.rs", include_str!("../src/panels/mod.rs")),
    (
        "src/panels/align.rs",
        include_str!("../src/panels/align.rs"),
    ),
    (
        "src/panels/boolean_panel.rs",
        include_str!("../src/panels/boolean_panel.rs"),
    ),
    ("src/panels/dock.rs", include_str!("../src/panels/dock.rs")),
    (
        "src/panels/fill_stroke.rs",
        include_str!("../src/panels/fill_stroke.rs"),
    ),
    (
        "src/panels/laser_panel.rs",
        include_str!("../src/panels/laser_panel.rs"),
    ),
    (
        "src/panels/object_manager.rs",
        include_str!("../src/panels/object_manager.rs"),
    ),
    (
        "src/panels/properties.rs",
        include_str!("../src/panels/properties.rs"),
    ),
    (
        "src/panels/properties_panel.rs",
        include_str!("../src/panels/properties_panel.rs"),
    ),
    (
        "src/panels/statusbar.rs",
        include_str!("../src/panels/statusbar.rs"),
    ),
    (
        "src/panels/toast.rs",
        include_str!("../src/panels/toast.rs"),
    ),
    (
        "src/panels/toolbar.rs",
        include_str!("../src/panels/toolbar.rs"),
    ),
    (
        "src/panels/welcome.rs",
        include_str!("../src/panels/welcome.rs"),
    ),
];

const FORBIDDEN_COLOR_PATTERNS: &[&str] = &[
    "Color32::from",
    "egui::Color32::",
    "Color32::WHITE",
    "Color32::BLACK",
    "Color32::TRANSPARENT",
    "Color32::RED",
    "Color32::GREEN",
    "Color32::BLUE",
    "Color32::YELLOW",
    "Color32::GRAY",
    "Color32::LIGHT_",
    "Color32::DARK_",
    "tiny_skia::Color::from_rgba8",
    "Color::from_rgba8",
    "Color::WHITE",
    "Color::BLACK",
];

#[test]
fn test_no_hardcoded_colors_outside_tokens() {
    let mut violations = Vec::new();
    for (file, contents) in APP_SOURCES_EXCEPT_TOKENS {
        for (line_idx, line) in contents.lines().enumerate() {
            for pat in FORBIDDEN_COLOR_PATTERNS {
                if line.contains(pat) {
                    violations.push(format!(
                        "{file}:{}: matched `{pat}` -> {}",
                        line_idx + 1,
                        line.trim()
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Hardcoded colors found outside tokens.rs:\n{}",
        violations.join("\n")
    );
}
