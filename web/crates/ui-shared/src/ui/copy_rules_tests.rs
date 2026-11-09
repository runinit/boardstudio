//! Repository check for the copied rust-ui components (ticket 003): every copy records its provenance, carries
//! `data-slot`s, and contains no inline script, `document::eval`, document listener, `icons` crate import or
//! `dark:` border class.

const COPIES: &[(&str, &str)] = &[
    ("accordion", include_str!("accordion.rs")),
    ("alert", include_str!("alert.rs")),
    ("alert_dialog", include_str!("alert_dialog.rs")),
    ("badge", include_str!("badge.rs")),
    ("button", include_str!("button.rs")),
    ("callout", include_str!("callout.rs")),
    ("card", include_str!("card.rs")),
    ("checkbox", include_str!("checkbox.rs")),
    ("collapsible", include_str!("collapsible.rs")),
    ("combobox", include_str!("combobox.rs")),
    ("dialog", include_str!("dialog.rs")),
    ("dropdown_menu", include_str!("dropdown_menu.rs")),
    ("empty", include_str!("empty.rs")),
    ("field", include_str!("field.rs")),
    ("input", include_str!("input.rs")),
    ("input_group", include_str!("input_group.rs")),
    ("item", include_str!("item.rs")),
    ("kbd", include_str!("kbd.rs")),
    ("label", include_str!("label.rs")),
    ("popover", include_str!("popover.rs")),
    ("select", include_str!("select.rs")),
    ("separator", include_str!("separator.rs")),
    ("sheet", include_str!("sheet.rs")),
    ("sidenav", include_str!("sidenav.rs")),
    ("skeleton", include_str!("skeleton.rs")),
    ("slider", include_str!("slider.rs")),
    ("spinner", include_str!("spinner.rs")),
    ("stepper", include_str!("stepper.rs")),
    ("table", include_str!("table.rs")),
    ("tabs", include_str!("tabs.rs")),
    ("toast", include_str!("toast.rs")),
    ("toggle_group", include_str!("toggle_group.rs")),
    ("toolbar", include_str!("toolbar.rs")),
    ("tooltip", include_str!("tooltip.rs")),
];

const FORBIDDEN: &[&str] = &[
    "script {",
    "dangerous_inner_html",
    "document::eval",
    "eval(",
    "addEventListener",
    "use icons::",
    "dark:border-",
    " border ",
    "border-transparent",
    "border-border",
];

#[test]
fn every_ui_file_is_listed() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui");
    let mut on_disk: Vec<String> = std::fs::read_dir(dir)
        .expect("src/ui is readable")
        .map(|entry| {
            entry
                .expect("dir entry")
                .file_name()
                .to_string_lossy()
                .trim_end_matches(".rs")
                .to_owned()
        })
        .filter(|name| {
            !matches!(
                name.as_str(),
                "mod" | "icons" | "command" | "copy_rules_tests" | "primitives_mount_tests"
            )
        })
        .collect();
    on_disk.sort();
    let listed: Vec<&str> = COPIES.iter().map(|(name, _)| *name).collect();
    assert_eq!(
        on_disk, listed,
        "add a new copy to COPIES so the rules below cover it"
    );
}

#[test]
fn copies_record_provenance_and_data_slots() {
    for (name, source) in COPIES {
        assert!(
            source.contains("7120636e68556ddd023a4ec8e083663c7b745318"),
            "{name}: missing upstream commit"
        );
        assert!(
            source.contains("rust-ui-MIT.txt"),
            "{name}: missing MIT notice pointer"
        );
        // Spinner's root is a shared icon, which carries `data-slot="icon"` itself.
        if *name != "spinner" {
            assert!(source.contains("\"data-slot\""), "{name}: no data-slot");
        }
    }
}

#[test]
fn copies_have_no_script_listener_icons_crate_or_dark_border() {
    for (name, source) in COPIES {
        let code: String = source
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        for needle in FORBIDDEN {
            assert!(!code.contains(needle), "{name}: contains `{needle}`");
        }
    }
}
