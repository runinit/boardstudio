//! Zero-dependency `Inflector` replacement for the CAD trees.
//!
//! `step-p21` and `step-p21-derive` call only [`to_pascal_case`] and
//! [`to_screaming_snake_case`] (cold-build ticket 07), so this crate implements
//! exactly those two conversions with the upstream `0.11.4` word-splitting
//! behavior and no `regex`/`lazy_static` dependency.
//!
//! [`to_pascal_case`]: cases::pascalcase::to_pascal_case
//! [`to_screaming_snake_case`]: cases::screamingsnakecase::to_screaming_snake_case

/// STEP/Rust identifier case conversions.
pub mod cases {
    /// Converts identifiers to `PascalCase` (`foo_bar` -> `FooBar`).
    pub mod pascalcase {
        /// Converts `value` to `PascalCase`.
        pub fn to_pascal_case(value: &str) -> String {
            super::to_case_camel_like(
                value,
                super::CamelOptions {
                    new_word: true,
                    last_char: ' ',
                    first_word: false,
                    injectable_char: ' ',
                    has_seperator: false,
                    inverted: false,
                },
            )
        }

        /// Tests whether `value` is already `PascalCase`.
        pub fn is_pascal_case(value: &str) -> bool {
            to_pascal_case(value) == value
        }
    }

    /// Converts identifiers to `SCREAMING_SNAKE_CASE` (`fooBar` -> `FOO_BAR`).
    pub mod screamingsnakecase {
        /// Converts `value` to `SCREAMING_SNAKE_CASE`.
        pub fn to_screaming_snake_case(value: &str) -> String {
            super::to_case_snake_like(value, "_", "upper")
        }

        /// Tests whether `value` is already `SCREAMING_SNAKE_CASE`.
        pub fn is_screaming_snake_case(value: &str) -> bool {
            value == to_screaming_snake_case(value)
        }
    }

    pub(crate) struct CamelOptions {
        pub(crate) new_word: bool,
        pub(crate) last_char: char,
        pub(crate) first_word: bool,
        pub(crate) injectable_char: char,
        pub(crate) has_seperator: bool,
        pub(crate) inverted: bool,
    }

    pub(crate) fn to_case_snake_like(
        convertable_string: &str,
        replace_with: &str,
        case: &str,
    ) -> String {
        let mut first_character: bool = true;
        let mut result: String = String::with_capacity(convertable_string.len() * 2);
        for char_with_index in trim_right(convertable_string).char_indices() {
            if char_is_seperator(&char_with_index.1) {
                if !first_character {
                    first_character = true;
                    result.push(replace_with.chars().nth(0).unwrap_or('_'));
                }
            } else if requires_seperator(char_with_index, first_character, convertable_string) {
                first_character = false;
                result = snake_like_with_seperator(result, replace_with, &char_with_index.1, case)
            } else {
                first_character = false;
                result = snake_like_no_seperator(result, &char_with_index.1, case)
            }
        }
        result
    }

    pub(crate) fn to_case_camel_like(
        convertable_string: &str,
        camel_options: CamelOptions,
    ) -> String {
        let mut new_word: bool = camel_options.new_word;
        let mut first_word: bool = camel_options.first_word;
        let mut last_char: char = camel_options.last_char;
        let mut found_real_char: bool = false;
        let mut result: String = String::with_capacity(convertable_string.len() * 2);
        for character in trim_right(convertable_string).chars() {
            if char_is_seperator(&character) && found_real_char {
                new_word = true;
            } else if !found_real_char && is_not_alphanumeric(character) {
                continue;
            } else if character.is_numeric() {
                found_real_char = true;
                new_word = true;
                result.push(character);
            } else if last_char_lower_current_is_upper_or_new_word(new_word, last_char, character) {
                found_real_char = true;
                new_word = false;
                result = append_on_new_word(result, first_word, character, &camel_options);
                first_word = false;
            } else {
                found_real_char = true;
                last_char = character;
                result.push(character.to_ascii_lowercase());
            }
        }
        result
    }

    fn append_on_new_word(
        mut result: String,
        first_word: bool,
        character: char,
        camel_options: &CamelOptions,
    ) -> String {
        if not_first_word_and_has_seperator(first_word, camel_options.has_seperator) {
            result.push(camel_options.injectable_char);
        }
        if first_word_or_not_inverted(first_word, camel_options.inverted) {
            result.push(character.to_ascii_uppercase());
        } else {
            result.push(character.to_ascii_lowercase());
        }
        result
    }

    fn not_first_word_and_has_seperator(first_word: bool, has_seperator: bool) -> bool {
        has_seperator && !first_word
    }

    fn first_word_or_not_inverted(first_word: bool, inverted: bool) -> bool {
        !inverted || first_word
    }

    fn last_char_lower_current_is_upper_or_new_word(
        new_word: bool,
        last_char: char,
        character: char,
    ) -> bool {
        new_word || ((last_char.is_lowercase() && character.is_uppercase()) && (last_char != ' '))
    }

    fn char_is_seperator(character: &char) -> bool {
        is_not_alphanumeric(*character)
    }

    fn trim_right(convertable_string: &str) -> &str {
        convertable_string.trim_end_matches(is_not_alphanumeric)
    }

    fn is_not_alphanumeric(character: char) -> bool {
        !character.is_alphanumeric()
    }

    fn requires_seperator(
        char_with_index: (usize, char),
        first_character: bool,
        convertable_string: &str,
    ) -> bool {
        !first_character
            && char_is_uppercase(char_with_index.1)
            && next_or_previous_char_is_lowercase(convertable_string, char_with_index.0)
    }

    fn snake_like_no_seperator(mut accumulator: String, current_char: &char, case: &str) -> String {
        if case == "lower" {
            accumulator.push(current_char.to_ascii_lowercase());
        } else {
            accumulator.push(current_char.to_ascii_uppercase());
        }
        accumulator
    }

    fn snake_like_with_seperator(
        mut accumulator: String,
        replace_with: &str,
        current_char: &char,
        case: &str,
    ) -> String {
        if case == "lower" {
            accumulator.push(replace_with.chars().nth(0).unwrap_or('_'));
            accumulator.push(current_char.to_ascii_lowercase());
        } else {
            accumulator.push(replace_with.chars().nth(0).unwrap_or('_'));
            accumulator.push(current_char.to_ascii_uppercase());
        }
        accumulator
    }

    fn next_or_previous_char_is_lowercase(
        convertable_string: &str,
        char_with_index: usize,
    ) -> bool {
        convertable_string.chars().nth(char_with_index + 1).unwrap_or('A').is_lowercase()
            || convertable_string.chars().nth(char_with_index - 1).unwrap_or('A').is_lowercase()
    }

    fn char_is_uppercase(test_char: char) -> bool {
        test_char == test_char.to_ascii_uppercase()
    }
}

/// Case conversions for STEP and Rust identifiers.
pub trait Inflector {
    /// Converts the string to `PascalCase`.
    fn to_pascal_case(&self) -> String;
    /// Tests whether the string is in `PascalCase`.
    fn is_pascal_case(&self) -> bool;
    /// Converts the string to `SCREAMING_SNAKE_CASE`.
    fn to_screaming_snake_case(&self) -> String;
    /// Tests whether the string is in `SCREAMING_SNAKE_CASE`.
    fn is_screaming_snake_case(&self) -> bool;
}

impl Inflector for str {
    fn to_pascal_case(&self) -> String {
        cases::pascalcase::to_pascal_case(self)
    }

    fn is_pascal_case(&self) -> bool {
        cases::pascalcase::is_pascal_case(self)
    }

    fn to_screaming_snake_case(&self) -> String {
        cases::screamingsnakecase::to_screaming_snake_case(self)
    }

    fn is_screaming_snake_case(&self) -> bool {
        cases::screamingsnakecase::is_screaming_snake_case(self)
    }
}

impl Inflector for String {
    fn to_pascal_case(&self) -> String {
        cases::pascalcase::to_pascal_case(self)
    }

    fn is_pascal_case(&self) -> bool {
        cases::pascalcase::is_pascal_case(self)
    }

    fn to_screaming_snake_case(&self) -> String {
        cases::screamingsnakecase::to_screaming_snake_case(self)
    }

    fn is_screaming_snake_case(&self) -> bool {
        cases::screamingsnakecase::is_screaming_snake_case(self)
    }
}

#[cfg(test)]
mod tests {
    use super::cases::{pascalcase, screamingsnakecase};
    use super::Inflector;

    /// (input, PascalCase, SCREAMING_SNAKE_CASE) parity table covering STEP/Rust
    /// identifiers, acronyms, digits, separators, and mixed-case inputs.
    const PARITY: &[(&str, &str, &str)] = &[
        ("", "", ""),
        ("a", "A", "A"),
        ("foo", "Foo", "FOO"),
        ("fooBar", "FooBar", "FOO_BAR"),
        ("FooBar", "FooBar", "FOO_BAR"),
        ("FOO_BAR", "FooBar", "FOO_BAR"),
        ("foo_bar", "FooBar", "FOO_BAR"),
        ("foo-bar", "FooBar", "FOO_BAR"),
        ("Foo-Bar", "FooBar", "FOO_BAR"),
        ("Foo Bar", "FooBar", "FOO_BAR"),
        ("Foo bar", "FooBar", "FOO_BAR"),
        ("fooBar3", "FooBar3", "FOO_BAR_3"),
        ("FooBar3", "FooBar3", "FOO_BAR_3"),
        ("HTTPServer", "Httpserver", "HTTP_SERVER"),
        ("HttpFooBar", "HttpFooBar", "HTTP_FOO_BAR"),
        ("CartesianPoint", "CartesianPoint", "CARTESIAN_POINT"),
        ("cartesian_point", "CartesianPoint", "CARTESIAN_POINT"),
        ("AdvancedBrep", "AdvancedBrep", "ADVANCED_BREP"),
        ("advanced_brep_shape", "AdvancedBrepShape", "ADVANCED_BREP_SHAPE"),
        ("my_entity", "MyEntity", "MY_ENTITY"),
        ("edge_curve", "EdgeCurve", "EDGE_CURVE"),
        ("a_b_c", "ABC", "A_B_C"),
        ("foo  bar", "FooBar", "FOO_BAR"),
        (" foo_bar ", "FooBar", "FOO_BAR"),
        ("foo_bar_", "FooBar", "FOO_BAR"),
        ("_foo_bar", "FooBar", "FOO_BAR"),
        ("-foo-bar-", "FooBar", "FOO_BAR"),
        ("Robert C. Martin", "RobertCMartin", "ROBERT_C_MARTIN"),
        ("has a + sign", "HasASign", "HAS_A_SIGN"),
        ("foo123Bar", "Foo123Bar", "FOO_123_BAR"),
        ("FOO123_BAR", "Foo123Bar", "FOO123_BAR"),
        ("a1B2c3", "A1B2C3", "A_1B_2C_3"),
        ("personID", "PersonID", "PERSON_ID"),
        ("PERSON_ID", "PersonId", "PERSON_ID"),
        ("MeasureValue", "MeasureValue", "MEASURE_VALUE"),
        (".T.", "T", "T"),
        ("TRUE", "True", "TRUE"),
        ("T", "T", "T"),
    ];

    #[test]
    fn case_conversions_match_upstream() {
        for (input, pascal, screaming) in PARITY {
            assert_eq!(pascalcase::to_pascal_case(input), *pascal, "{input:?}");
            assert_eq!(
                screamingsnakecase::to_screaming_snake_case(input),
                *screaming,
                "{input:?}"
            );
            assert_eq!(input.to_pascal_case(), *pascal, "trait {input:?}");
            assert_eq!(
                input.to_screaming_snake_case(),
                *screaming,
                "trait {input:?}"
            );
            assert_eq!(
                (*input).to_string().to_pascal_case(),
                *pascal,
                "String {input:?}"
            );
        }
    }

    #[test]
    fn predicates_match_conversions() {
        assert!("FooBar".is_pascal_case());
        assert!(!"fooBar".is_pascal_case());
        assert!(!"FOO_BAR".is_pascal_case());
        assert!("FOO_BAR".is_screaming_snake_case());
        assert!(!"FooBar".is_screaming_snake_case());
        assert!(!"foo_bar".is_screaming_snake_case());
    }
}
