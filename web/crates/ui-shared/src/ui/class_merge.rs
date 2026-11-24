//! Conflict-aware Tailwind class merger (zero dependencies).
//!
//! Tailwind CSS v4 applies styles in stylesheet declaration order, not HTML `class`
//! attribute order, so callers that override e.g. `Card`'s `flex` with `block` or its
//! `bg-card` with `bg-canvas` need later tokens to supersede earlier conflicting ones.
//! Simple string concatenation would keep both and break the visual layout. This module
//! is the zero-dependency replacement for the retired external class-merge crate:
//! later parts win over earlier ones within the same modifier scope and conflict group,
//! and surviving tokens keep their last-occurrence order.

/// Merge Tailwind class lists, letting later conflicting tokens supersede earlier ones.
///
/// Tokens are split on whitespace across `parts` in order. Each token is divided into
/// `(modifiers, base)` at the last `:` outside `[...]` brackets, so stacked variants
/// such as `dark:aria-invalid:ring-destructive/40` or `[&>svg]:size-4` keep their exact
/// scope, and only tokens sharing identical modifiers can conflict. Conflicts follow the
/// group table in [`conflict_group`]; anything else dedups on the exact token.
///
/// Fast path: when at most one part carries any token (the common case of a base string
/// plus an empty `class` prop), that part is returned verbatim without tokenizing.
pub(crate) fn merge_classes(parts: &[&str]) -> String {
    let mut non_empty = parts
        .iter()
        .copied()
        .filter(|p| p.split_whitespace().next().is_some());
    if let (first, None) = (non_empty.next(), non_empty.next()) {
        return first.unwrap_or("").to_owned();
    }
    let mut surviving: Vec<(&str, &'static str, &str, &str)> = Vec::new();
    for part in parts {
        for token in part.split_whitespace() {
            let (modifiers, base) = split_modifiers(token);
            let (group, detail) = conflict_group(base);
            surviving.retain(|&(m, g, d, _)| m != modifiers || !evicts(group, detail, g, d));
            surviving.push((modifiers, group, detail, token));
        }
    }
    let mut merged = String::new();
    for (_, _, _, token) in surviving {
        if !merged.is_empty() {
            merged.push(' ');
        }
        merged.push_str(token);
    }
    merged
}

/// Thin call-site shorthand so component files keep the shape of the retired macro calls.
///
/// `macro_rules!` takes no visibility qualifier and is only path-addressable through a
/// `use` bridge; the alias below exposes it at this module's path so sibling `ui::*`
/// modules can `use crate::ui::class_merge::merge_classes;` for both macro and function.
macro_rules! merge_classes_impl {
    ($($part:expr),+ $(,)?) => {
        $crate::ui::class_merge::merge_classes(&[$($part),+])
    };
}
pub(crate) use merge_classes_impl as merge_classes;

/// Split `token` into `(modifiers, base)` at the last `:` outside `[...]` brackets.
fn split_modifiers(token: &str) -> (&str, &str) {
    let mut depth = 0_usize;
    let mut split = None;
    for (index, byte) in token.bytes().enumerate() {
        match byte {
            b'[' => depth += 1,
            b']' => depth = depth.saturating_sub(1),
            b':' if depth == 0 => split = Some(index),
            _ => {}
        }
    }
    match split {
        Some(index) => (&token[..index], &token[index + 1..]),
        None => ("", token),
    }
}

fn has_word(words: &str, token: &str) -> bool {
    words.split(' ').any(|word| word == token)
}

const EVICT_SUPERSET: &[(&str, &str)] = &[
    ("size", "w h"),
    ("text-size", "leading"),
    ("p", "px py pt pr pb pl"),
    ("px", "pl pr"),
    ("py", "pt pb"),
    ("m", "mx my mt mr mb ml"),
    ("mx", "ml mr"),
    ("my", "mt mb"),
    ("gap", "gap-x gap-y"),
    ("rounded", "rounded-dir"),
    ("inset", "inset-x inset-y top right bottom left"),
    ("inset-x", "left right"),
    ("inset-y", "top bottom"),
];

/// Whether `(group, detail)` supersedes an earlier `(old_group, old_detail)` token.
fn evicts(group: &str, detail: &str, old_group: &str, old_detail: &str) -> bool {
    (group == old_group && detail == old_detail)
        || EVICT_SUPERSET
            .iter()
            .any(|&(g, olds)| g == group && has_word(olds, old_group))
}

// Ordered longest-prefix-first so e.g. `justify-items-` beats `justify-` and `overflow-x-` beats `overflow-`.
const PREFIX_GROUPS: &str = "\
    whitespace tracking leading justify-items justify-self justify items content self order \
    opacity z cursor basis pointer-events duration aspect line-clamp select overscroll \
    overflow-x overflow-y overflow gap-x gap-y min-w max-w min-h max-h px py pt pr pb pl ps \
    pe mx my mt mr mb ml ms me inset-x inset-y top right bottom left rotate scale";

const EXACT_OR_PREFIX_GROUPS: &str = "transition shadow outline shrink grow gap size w h p m inset";

const EXACT_GROUPS: &[(&str, &str)] = &[
    (
        "block inline-block inline flex inline-flex table inline-table grid inline-grid contents list-item hidden",
        "display",
    ),
    (
        "flex-row flex-row-reverse flex-col flex-col-reverse",
        "flex-direction",
    ),
    ("flex-wrap flex-wrap-reverse flex-nowrap", "flex-wrap"),
    ("static fixed absolute relative sticky", "position"),
    ("truncate", "text-overflow"),
    ("ring", "ring-width"),
    ("rounded", "rounded"),
    ("border", "border-width"),
];

const TEXT_SIZES: &str = "xs sm base lg xl 2xl 3xl 4xl 5xl 6xl 7xl 8xl 9xl";
const ROUNDED_DIRS: &str = "t r b l tl tr bl br s e ss se es ee start end";
const BG_POSITIONS: &str =
    "bottom center left left-bottom left-top right right-bottom right-top top";

fn starts_with_bracket_digit(rest: &str) -> bool {
    rest.starts_with('[') && rest.as_bytes().get(1).is_some_and(u8::is_ascii_digit)
}

/// Classify `base` into a conflict group + detail slice. Unknown utilities dedup exactly.
fn conflict_group(base: &str) -> (&'static str, &str) {
    let core = base.strip_prefix('-').unwrap_or(base);
    if let Some(&(_, group)) = EXACT_GROUPS
        .iter()
        .find(|&&(tokens, _)| has_word(tokens, core))
    {
        return (group, "");
    }
    if let Some(group) = PREFIX_GROUPS
        .split(' ')
        .find(|&g| core.strip_prefix(g).is_some_and(|r| r.starts_with('-')))
    {
        return (group, "");
    }
    if let Some(group) = EXACT_OR_PREFIX_GROUPS
        .split(' ')
        .find(|&g| core == g || core.strip_prefix(g).is_some_and(|r| r.starts_with('-')))
    {
        return (group, "");
    }
    if let Some(rest) = core.strip_prefix("font-") {
        if has_word(
            "thin extralight light normal medium semibold bold extrabold black",
            rest,
        ) || rest.starts_with('[')
        {
            return ("font-weight", "");
        }
        if has_word("sans serif mono", rest) || rest.starts_with('(') {
            return ("font-family", "");
        }
        return ("", base);
    }
    if let Some(rest) = core.strip_prefix("text-") {
        if has_word(TEXT_SIZES, rest) || starts_with_bracket_digit(rest) {
            return ("text-size", "");
        }
        return [
            ("left center right justify start end", "text-align"),
            ("wrap nowrap balance pretty", "text-wrap"),
            ("ellipsis clip", "text-overflow"),
        ]
        .into_iter()
        .find_map(|(words, group)| has_word(words, rest).then_some((group, "")))
        .unwrap_or(("text-color", ""));
    }
    if let Some(rest) = core.strip_prefix("ring-") {
        if rest.starts_with("offset-") {
            return ("ring-offset", "");
        }
        if rest == "inset" {
            return ("ring-inset", "");
        }
        if starts_with_bracket_digit(rest) || rest.bytes().all(|b| b.is_ascii_digit()) {
            return ("ring-width", "");
        }
        return ("ring-color", "");
    }
    if let Some(rest) = core.strip_prefix("rounded-") {
        let segment = rest.split('-').next().unwrap_or(rest);
        if !rest.starts_with('[') && has_word(ROUNDED_DIRS, segment) {
            return ("rounded-dir", segment);
        }
        return ("rounded", "");
    }
    if let Some(rest) = core.strip_prefix("border-") {
        if has_word("solid dashed dotted double hidden none", rest) {
            return ("border-style", "");
        }
        if let Some(dir) = ["t", "r", "b", "l", "x", "y"]
            .into_iter()
            .find(|dir| rest == *dir || rest.strip_prefix(dir).is_some_and(|s| s.starts_with('-')))
        {
            return ("border-dir", dir);
        }
        if has_word("0 2 4 8", rest) || starts_with_bracket_digit(rest) {
            return ("border-width", "");
        }
        return ("border-color", "");
    }
    if let Some(rest) = core.strip_prefix("bg-") {
        if let Some(hit) = [
            (BG_POSITIONS, "bg-position"),
            ("auto cover contain", "bg-size"),
            ("fixed local scroll", "bg-attachment"),
        ]
        .into_iter()
        .find_map(|(words, group)| has_word(words, rest).then_some((group, "")))
        {
            return hit;
        }
        if rest == "repeat" || rest.starts_with("repeat-") || rest == "no-repeat" {
            return ("bg-repeat", "");
        }
        if rest.starts_with("clip-") {
            return ("bg-clip", "");
        }
        if rest.starts_with("origin-") {
            return ("bg-origin", "");
        }
        if rest == "none"
            || "gradient- linear- [url( [linear- [radial- [conic-"
                .split(' ')
                .any(|p| rest.starts_with(p))
        {
            return ("bg-image", "");
        }
        return ("bg-color", "");
    }
    ("", base)
}

#[cfg(test)]
mod tests {
    use super::merge_classes;

    #[test]
    fn fast_path_returns_single_slice_verbatim() {
        for (parts, expected) in [
            (&["flex flex-col"][..], "flex flex-col"),
            (&["flex", ""], "flex"),
            (&["", "gap-4"], "gap-4"),
            (&[""], ""),
            (&[], ""),
        ] {
            assert_eq!(merge_classes(parts), expected);
        }
    }

    #[test]
    fn card_and_button_real_caller_overrides() {
        let check = |parts: &[&str], kept: &str, dropped: &str| {
            let merged = merge_classes(parts);
            let tokens: Vec<&str> = merged.split_whitespace().collect();
            for token in kept.split_whitespace() {
                assert!(tokens.contains(&token), "missing {token}: {tokens:?}");
            }
            for token in dropped.split_whitespace() {
                assert!(!tokens.contains(&token), "kept {token}: {tokens:?}");
            }
        };
        check(
            &[
                "bg-card text-card-foreground flex flex-col rounded-xl shadow-sm",
                "py-6 gap-4",
                "block bg-canvas p-0 text-lg",
            ],
            "block bg-canvas p-0 text-lg text-card-foreground flex-col rounded-xl shadow-sm gap-4",
            "flex bg-card py-6",
        );
        check(
            &[
                "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm",
                "bg-primary text-primary-foreground shadow-xs",
                "h-control px-4 py-2",
                "w-fit h-9 px-2 bg-red-500 text-white gap-1",
            ],
            "inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm shadow-xs py-2 w-fit h-9 px-2 bg-red-500 text-white gap-1",
            "h-control px-4 bg-primary text-primary-foreground gap-2",
        );
    }

    #[test]
    fn groups_modifiers_and_edge_cases() {
        for (parts, expected) in [
            (&["size-4", "size-3.5"][..], "size-3.5"),
            (&["size-4 animate-spin", "size-6"], "animate-spin size-6"),
            (&["size-4", "w-2"], "size-4 w-2"),
            (
                &["text-sm leading-none", "text-xs text-foreground"],
                "text-xs text-foreground",
            ),
            (&["w-2 h-2", "size-4"], "size-4"),
            (
                &["focus-visible:ring-2", "focus-visible:ring-[3px]"],
                "focus-visible:ring-[3px]",
            ),
            (
                &["focus-visible:ring-2", "ring-2"],
                "focus-visible:ring-2 ring-2",
            ),
            (
                &["text-sm shadow-none flex gap-2 items-center", "gap-1"],
                "text-sm shadow-none flex items-center gap-1",
            ),
            (
                &["flex flex-col", "max-[760px]:flex"],
                "flex flex-col max-[760px]:flex",
            ),
            (
                &["text-sm text-foreground whitespace-nowrap", "text-xs"],
                "text-foreground whitespace-nowrap text-xs",
            ),
            (
                &["text-muted-foreground", "text-foreground"],
                "text-foreground",
            ),
            (&["[&>svg]:size-4", "[&>svg]:size-3"], "[&>svg]:size-3"),
            (
                &["[&_tr:last-child]:border-0", "border"],
                "[&_tr:last-child]:border-0 border",
            ),
            (&["px-6 py-6", "p-0"], "p-0"),
            (&["p-4", "px-2"], "p-4 px-2"),
            (&["pl-4 pr-4", "px-2"], "px-2"),
            (&["my-0", "mt-2"], "my-0 mt-2"),
            (
                &["-mx-1 my-1 h-px bg-border", "-mx-2"],
                "my-1 h-px bg-border -mx-2",
            ),
            (&["rounded-xl", "rounded-md"], "rounded-md"),
            (&["rounded-xl", "rounded-t-lg"], "rounded-xl rounded-t-lg"),
            (&["rounded-t-lg", "rounded"], "rounded"),
            (&["border", "border-2"], "border-2"),
            (
                &["border-t border-border", "border-t-2"],
                "border-border border-t-2",
            ),
            (&["bg-card", "bg-cover"], "bg-card bg-cover"),
            (&["bg-card", "bg-canvas"], "bg-canvas"),
            (&["bg-card", "bg-[#123456]"], "bg-[#123456]"),
            (&["bg-card", "bg-none"], "bg-card bg-none"),
            (
                &["group/item flex", "group/item flex-none"],
                "flex group/item flex-none",
            ),
            (&["px-6 text-sm flex", "block"], "px-6 text-sm block"),
        ] {
            assert_eq!(merge_classes(parts), expected, "{parts:?}");
        }
    }
}
