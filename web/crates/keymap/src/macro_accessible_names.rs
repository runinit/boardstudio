pub(crate) fn step_kind_control_name(accepted_macro_name: &str, index: usize) -> String {
    format!("{accepted_macro_name} step {}", index + 1)
}

pub(crate) fn step_value_control_name(is_wait: bool) -> &'static str {
    if is_wait { "Delay (ms)" } else { "Keycode" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_kind_name_uses_the_accepted_macro_display_name_and_one_based_index() {
        assert_eq!(step_kind_control_name("Macro 1", 0), "Macro 1 step 1");
        assert_eq!(
            step_kind_control_name("Kitchen layer", 2),
            "Kitchen layer step 3"
        );
    }

    #[test]
    fn step_value_names_match_the_visible_field_labels() {
        assert_eq!(step_value_control_name(false), "Keycode");
        assert_eq!(step_value_control_name(true), "Delay (ms)");
    }
}
