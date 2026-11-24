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
    let mut sole: Option<&str> = None;
    let mut non_empty = 0;
    for part in parts {
        if part.split_whitespace().next().is_none() {
            continue;
        }
        non_empty += 1;
        if non_empty > 1 {
            break;
        }
        sole = Some(part);
    }
    if non_empty <= 1 {
        return sole.unwrap_or("").to_owned();
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

/// Whether `(group, detail)` supersedes an earlier `(old_group, old_detail)` token.
fn evicts(group: &str, detail: &str, old_group: &str, old_detail: &str) -> bool {
    if group == old_group {
        return detail == old_detail;
    }
    match group {
        "size" => matches!(old_group, "w" | "h" | "size"),
        "w" => matches!(old_group, "w" | "size"),
        "h" => matches!(old_group, "h" | "size"),
        "p" => matches!(old_group, "p" | "px" | "py" | "pt" | "pr" | "pb" | "pl"),
        "px" => matches!(old_group, "px" | "pl" | "pr"),
        "py" => matches!(old_group, "py" | "pt" | "pb"),
        "m" => matches!(old_group, "m" | "mx" | "my" | "mt" | "mr" | "mb" | "ml"),
        "mx" => matches!(old_group, "mx" | "ml" | "mr"),
        "my" => matches!(old_group, "my" | "mt" | "mb"),
        "gap" => matches!(old_group, "gap" | "gap-x" | "gap-y"),
        "rounded" => matches!(old_group, "rounded" | "rounded-dir"),
        "inset" => matches!(
            old_group,
            "inset" | "inset-x" | "inset-y" | "top" | "right" | "bottom" | "left"
        ),
        "inset-x" => matches!(old_group, "inset-x" | "left" | "right"),
        "inset-y" => matches!(old_group, "inset-y" | "top" | "bottom"),
        _ => false,
    }
}

const PREFIX_GROUPS: &[(&str, &str)] = &[
    ("whitespace-", "whitespace"),
    ("tracking-", "tracking"),
    ("leading-", "leading"),
    ("justify-items-", "justify-items"),
    ("justify-self-", "justify-self"),
    ("justify-", "justify"),
    ("items-", "items"),
    ("content-", "content"),
    ("self-", "self"),
    ("order-", "order"),
    ("opacity-", "opacity"),
    ("z-", "z"),
    ("cursor-", "cursor"),
    ("basis-", "basis"),
    ("pointer-events-", "pointer-events"),
    ("duration-", "duration"),
    ("aspect-", "aspect"),
    ("line-clamp-", "line-clamp"),
    ("select-", "select"),
    ("overscroll-", "overscroll"),
    ("overflow-x-", "overflow-x"),
    ("overflow-y-", "overflow-y"),
    ("overflow-", "overflow"),
    ("gap-x-", "gap-x"),
    ("gap-y-", "gap-y"),
    ("min-w-", "min-w"),
    ("max-w-", "max-w"),
    ("min-h-", "min-h"),
    ("max-h-", "max-h"),
    ("px-", "px"),
    ("py-", "py"),
    ("pt-", "pt"),
    ("pr-", "pr"),
    ("pb-", "pb"),
    ("pl-", "pl"),
    ("ps-", "ps"),
    ("pe-", "pe"),
    ("mx-", "mx"),
    ("my-", "my"),
    ("mt-", "mt"),
    ("mr-", "mr"),
    ("mb-", "mb"),
    ("ml-", "ml"),
    ("ms-", "ms"),
    ("me-", "me"),
    ("inset-x-", "inset-x"),
    ("inset-y-", "inset-y"),
    ("top-", "top"),
    ("right-", "right"),
    ("bottom-", "bottom"),
    ("left-", "left"),
    ("rotate-", "rotate"),
    ("scale-", "scale"),
];

const EXACT_OR_PREFIX_GROUPS: &[(&str, &str)] = &[
    ("transition", "transition"),
    ("shadow", "shadow"),
    ("outline", "outline"),
    ("shrink", "shrink"),
    ("grow", "grow"),
    ("gap", "gap"),
    ("size", "size"),
    ("w", "w"),
    ("h", "h"),
    ("p", "p"),
    ("m", "m"),
    ("inset", "inset"),
];

fn starts_with_bracket_digit(rest: &str) -> bool {
    rest.starts_with('[') && rest.as_bytes().get(1).is_some_and(u8::is_ascii_digit)
}

/// Classify `base` into a conflict group + detail slice. Unknown utilities dedup exactly.
fn conflict_group(base: &str) -> (&'static str, &str) {
    let core = base.strip_prefix('-').unwrap_or(base);
    match core {
        "block" | "inline-block" | "inline" | "flex" | "inline-flex" | "table" | "inline-table"
        | "grid" | "inline-grid" | "contents" | "list-item" | "hidden" => return ("display", ""),
        "flex-row" | "flex-row-reverse" | "flex-col" | "flex-col-reverse" => {
            return ("flex-direction", "");
        }
        "flex-wrap" | "flex-wrap-reverse" | "flex-nowrap" => return ("flex-wrap", ""),
        "static" | "fixed" | "absolute" | "relative" | "sticky" => return ("position", ""),
        "truncate" => return ("text-overflow", ""),
        "ring" => return ("ring-width", ""),
        "rounded" => return ("rounded", ""),
        "border" => return ("border-width", ""),
        _ => {}
    }
    if let Some(&(_, group)) = PREFIX_GROUPS
        .iter()
        .find(|&&(prefix, _)| core.starts_with(prefix))
    {
        return (group, "");
    }
    if let Some(&(_, group)) = EXACT_OR_PREFIX_GROUPS.iter().find(|&&(stem, _)| {
        core == stem || (core.starts_with(stem) && core.as_bytes().get(stem.len()) == Some(&b'-'))
    }) {
        return (group, "");
    }
    if let Some(rest) = core.strip_prefix("font-") {
        if matches!(
            rest,
            "thin"
                | "extralight"
                | "light"
                | "normal"
                | "medium"
                | "semibold"
                | "bold"
                | "extrabold"
                | "black"
        ) || rest.starts_with('[')
        {
            return ("font-weight", "");
        }
        if matches!(rest, "sans" | "serif" | "mono") || rest.starts_with('(') {
            return ("font-family", "");
        }
        return ("", base);
    }
    if let Some(rest) = core.strip_prefix("text-") {
        if matches!(
            rest,
            "xs" | "sm"
                | "base"
                | "lg"
                | "xl"
                | "2xl"
                | "3xl"
                | "4xl"
                | "5xl"
                | "6xl"
                | "7xl"
                | "8xl"
                | "9xl"
        ) || starts_with_bracket_digit(rest)
        {
            return ("text-size", "");
        }
        if matches!(
            rest,
            "left" | "center" | "right" | "justify" | "start" | "end"
        ) {
            return ("text-align", "");
        }
        if matches!(rest, "wrap" | "nowrap" | "balance" | "pretty") {
            return ("text-wrap", "");
        }
        if matches!(rest, "ellipsis" | "clip") {
            return ("text-overflow", "");
        }
        return ("text-color", "");
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
        if !rest.starts_with('[')
            && matches!(
                segment,
                "t" | "r"
                    | "b"
                    | "l"
                    | "tl"
                    | "tr"
                    | "bl"
                    | "br"
                    | "s"
                    | "e"
                    | "ss"
                    | "se"
                    | "es"
                    | "ee"
                    | "start"
                    | "end"
            )
        {
            return ("rounded-dir", segment);
        }
        return ("rounded", "");
    }
    if let Some(rest) = core.strip_prefix("border-") {
        if matches!(
            rest,
            "solid" | "dashed" | "dotted" | "double" | "hidden" | "none"
        ) {
            return ("border-style", "");
        }
        if let Some(direction) = ["t", "r", "b", "l", "x", "y"]
            .into_iter()
            .find(|dir| rest == *dir || rest.strip_prefix(dir).is_some_and(|s| s.starts_with('-')))
        {
            return ("border-dir", direction);
        }
        if matches!(rest, "0" | "2" | "4" | "8") || starts_with_bracket_digit(rest) {
            return ("border-width", "");
        }
        return ("border-color", "");
    }
    if let Some(rest) = core.strip_prefix("bg-") {
        if matches!(
            rest,
            "bottom"
                | "center"
                | "left"
                | "left-bottom"
                | "left-top"
                | "right"
                | "right-bottom"
                | "right-top"
                | "top"
        ) {
            return ("bg-position", "");
        }
        if matches!(rest, "auto" | "cover" | "contain") {
            return ("bg-size", "");
        }
        if rest == "repeat" || rest.starts_with("repeat-") || rest == "no-repeat" {
            return ("bg-repeat", "");
        }
        if matches!(rest, "fixed" | "local" | "scroll") {
            return ("bg-attachment", "");
        }
        if rest.starts_with("clip-") {
            return ("bg-clip", "");
        }
        if rest.starts_with("origin-") {
            return ("bg-origin", "");
        }
        if rest == "none"
            || rest.starts_with("gradient-")
            || rest.starts_with("linear-")
            || rest.starts_with("[url(")
            || rest.starts_with("[linear-")
            || rest.starts_with("[radial-")
            || rest.starts_with("[conic-")
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
        let merged_card = merge_classes(&[
            "bg-card text-card-foreground flex flex-col rounded-xl shadow-sm",
            "py-6 gap-4",
            "block bg-canvas p-0 text-lg",
        ]);
        let card: Vec<&str> = merged_card.split_whitespace().collect();
        for kept in [
            "block",
            "bg-canvas",
            "p-0",
            "text-lg",
            "text-card-foreground",
            "flex-col",
            "rounded-xl",
            "shadow-sm",
            "gap-4",
        ] {
            assert!(card.contains(&kept), "card missing {kept}: {card:?}");
        }
        for dropped in ["flex", "bg-card", "py-6"] {
            assert!(!card.contains(&dropped), "card kept {dropped}: {card:?}");
        }

        let merged_btn = merge_classes(&[
            "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm",
            "bg-primary text-primary-foreground shadow-xs",
            "h-control px-4 py-2",
            "w-fit h-9 px-2 bg-red-500 text-white gap-1",
        ]);
        let btn: Vec<&str> = merged_btn.split_whitespace().collect();
        for kept in [
            "inline-flex",
            "items-center",
            "justify-center",
            "whitespace-nowrap",
            "rounded-md",
            "text-sm",
            "shadow-xs",
            "py-2",
            "w-fit",
            "h-9",
            "px-2",
            "bg-red-500",
            "text-white",
            "gap-1",
        ] {
            assert!(btn.contains(&kept), "btn missing {kept}: {btn:?}");
        }
        for dropped in [
            "h-control",
            "px-4",
            "bg-primary",
            "text-primary-foreground",
            "gap-2",
        ] {
            assert!(!btn.contains(&dropped), "btn kept {dropped}: {btn:?}");
        }
    }

    #[test]
    fn groups_modifiers_and_edge_cases() {
        for (parts, expected) in [
            (&["size-4", "size-3.5"][..], "size-3.5"),
            (&["size-4 animate-spin", "size-6"], "animate-spin size-6"),
            (&["size-4", "w-2"], "w-2"),
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
