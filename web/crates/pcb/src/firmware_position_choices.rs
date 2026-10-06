//! Legacy PCB firmware-position choices mirrored from the existing TypeScript editor.

pub fn choices() -> Vec<(String, String)> {
    let mut choices = vec![
        ("&none".into(), "Unassigned".into()),
        ("&trans".into(), "Transparent".into()),
    ];
    choices.extend((b'A'..=b'Z').map(|key| {
        let key = char::from(key).to_string();
        (format!("&kp {key}"), key)
    }));
    choices.extend((0..10).map(|digit| (format!("&kp N{digit}"), digit.to_string())));
    choices.extend((1..=12).map(|function| (format!("&kp F{function}"), format!("F{function}"))));
    choices.extend([
        ("&kp SPACE".into(), "Space".into()),
        ("&kp ENTER".into(), "Enter".into()),
        ("&kp ESC".into(), "Esc".into()),
        ("&kp TAB".into(), "Tab".into()),
        ("&kp BSPC".into(), "Backspace".into()),
        ("&kp LSHFT".into(), "Shift".into()),
        ("&kp LCTRL".into(), "Ctrl".into()),
        ("&kp LALT".into(), "Alt".into()),
        ("&kp LGUI".into(), "Super".into()),
        ("&kp UP".into(), "Up".into()),
        ("&kp DOWN".into(), "Down".into()),
        ("&kp LEFT".into(), "Left".into()),
        ("&kp RIGHT".into(), "Right".into()),
        ("&kp MINUS".into(), "-".into()),
        ("&kp EQUAL".into(), "=".into()),
        ("&kp LBKT".into(), "[".into()),
        ("&kp RBKT".into(), "]".into()),
        ("&kp BSLH".into(), "\\".into()),
        ("&kp SEMI".into(), ";".into()),
        ("&kp SQT".into(), "'".into()),
        ("&kp COMMA".into(), ",".into()),
        ("&kp DOT".into(), ".".into()),
        ("&kp FSLH".into(), "/".into()),
        ("&kp GRAVE".into(), "`".into()),
    ]);
    choices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choices_match_reference_defaults_labels_and_order() {
        let choices = choices();
        assert_eq!(
            choices.first().unwrap(),
            &("&none".into(), "Unassigned".into())
        );
        assert_eq!(
            choices.get(1).unwrap(),
            &("&trans".into(), "Transparent".into())
        );
        assert_eq!(choices.get(2).unwrap(), &("&kp A".into(), "A".into()));
        assert_eq!(choices.get(27).unwrap(), &("&kp Z".into(), "Z".into()));
        assert_eq!(choices.get(28).unwrap(), &("&kp N0".into(), "0".into()));
        assert_eq!(choices.get(37).unwrap(), &("&kp N9".into(), "9".into()));
        assert!(choices.contains(&("&kp F12".into(), "F12".into())));
        assert!(choices.contains(&("&kp LSHFT".into(), "Shift".into())));
        assert!(choices.contains(&("&kp GRAVE".into(), "`".into())));
        assert_eq!(choices.len(), 2 + 26 + 10 + 12 + 24);
    }
}
