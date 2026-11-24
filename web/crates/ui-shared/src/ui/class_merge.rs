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
    let mut surviving: Vec<Slot<'_>> = Vec::new();
    for part in parts {
        for token in part.split_whitespace() {
            let (modifiers, base) = split_modifiers(token);
            let (group, detail) = conflict_group(base);
            let mut index = 0;
            while index < surviving.len() {
                let kept = &surviving[index];
                if kept.modifiers == modifiers && evicts(group, detail, kept.group, kept.detail) {
                    surviving.remove(index);
                } else {
                    index += 1;
                }
            }
            surviving.push(Slot {
                modifiers,
                group,
                detail,
                token,
            });
        }
    }
    let mut merged = String::new();
    for slot in surviving {
        if !merged.is_empty() {
            merged.push(' ');
        }
        merged.push_str(slot.token);
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

/// One surviving token plus the key deciding which later token supersedes it.
struct Slot<'a> {
    modifiers: &'a str,
    group: &'static str,
    detail: &'a str,
    token: &'a str,
}

/// Split `token` into `(modifiers, base)` at the last `:` outside `[...]` brackets.
/// Tokens without such a colon (including arbitrary properties like
/// `[-webkit-tap-highlight-color:transparent]`) have empty modifiers.
fn split_modifiers(token: &str) -> (&str, &str) {
    let bytes = token.as_bytes();
    let mut depth = 0_usize;
    let mut split = None;
    for (index, byte) in bytes.iter().enumerate() {
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

/// Whether a token in `(group, detail)` supersedes a surviving `(old_group, old_detail)`
/// token. Same-group tokens collide only on equal detail (directional `rounded-t-*`
/// overrides just `rounded-t-*`; unknown tokens dedup exactly); the match arms below
/// list the cross-group supersessions (`p-*` over `px-*`, `size-*` over `w-*`, ...).
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

/// Classify `base` (the token without modifiers, minus one leading `-`) into a conflict
/// group plus a detail slice. Unknown utilities return `("", base)` for exact dedup.
fn conflict_group(base: &str) -> (&'static str, &str) {
    let core = base.strip_prefix('-').unwrap_or(base);
    if matches!(
        core,
        "block"
            | "inline-block"
            | "inline"
            | "flex"
            | "inline-flex"
            | "table"
            | "inline-table"
            | "grid"
            | "inline-grid"
            | "contents"
            | "list-item"
            | "hidden"
    ) {
        return ("display", "");
    }
    if matches!(
        core,
        "flex-row" | "flex-row-reverse" | "flex-col" | "flex-col-reverse"
    ) {
        return ("flex-direction", "");
    }
    if matches!(core, "flex-wrap" | "flex-wrap-reverse" | "flex-nowrap") {
        return ("flex-wrap", "");
    }
    if matches!(
        core,
        "static" | "fixed" | "absolute" | "relative" | "sticky"
    ) {
        return ("position", "");
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
        ) {
            return ("text-size", "");
        }
        if rest.starts_with('[') {
            return match rest.as_bytes().get(1) {
                Some(first) if first.is_ascii_digit() => ("text-size", ""),
                _ => ("text-color", ""),
            };
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
    if core == "truncate" {
        return ("text-overflow", "");
    }
    if core == "transition" || core.starts_with("transition-") {
        return ("transition", "");
    }
    if core == "shadow" || core.starts_with("shadow-") {
        return ("shadow", "");
    }
    if core == "ring" {
        return ("ring-width", "");
    }
    if let Some(rest) = core.strip_prefix("ring-") {
        if rest.starts_with("offset-") {
            return ("ring-offset", "");
        }
        if rest == "inset" {
            return ("ring-inset", "");
        }
        if rest.starts_with('[') {
            return match rest.as_bytes().get(1) {
                Some(first) if first.is_ascii_digit() => ("ring-width", ""),
                _ => ("ring-color", ""),
            };
        }
        if rest.bytes().all(|byte| byte.is_ascii_digit()) {
            return ("ring-width", "");
        }
        return ("ring-color", "");
    }
    if core == "outline" || core.starts_with("outline-") {
        return ("outline", "");
    }
    if core == "shrink" || core.starts_with("shrink-") {
        return ("shrink", "");
    }
    if core == "grow" || core.starts_with("grow-") {
        return ("grow", "");
    }
    if core.starts_with("whitespace-") {
        return ("whitespace", "");
    }
    if core.starts_with("tracking-") {
        return ("tracking", "");
    }
    if core.starts_with("leading-") {
        return ("leading", "");
    }
    if core.starts_with("justify-items-") {
        return ("justify-items", "");
    }
    if core.starts_with("justify-self-") {
        return ("justify-self", "");
    }
    if core.starts_with("justify-") {
        return ("justify", "");
    }
    if core.starts_with("items-") {
        return ("items", "");
    }
    if core.starts_with("content-") {
        return ("content", "");
    }
    if core.starts_with("self-") {
        return ("self", "");
    }
    if core.starts_with("order-") {
        return ("order", "");
    }
    if core.starts_with("opacity-") {
        return ("opacity", "");
    }
    if core.starts_with("z-") {
        return ("z", "");
    }
    if core.starts_with("cursor-") {
        return ("cursor", "");
    }
    if core.starts_with("basis-") {
        return ("basis", "");
    }
    if core.starts_with("pointer-events-") {
        return ("pointer-events", "");
    }
    if core.starts_with("duration-") {
        return ("duration", "");
    }
    if core.starts_with("aspect-") {
        return ("aspect", "");
    }
    if core.starts_with("line-clamp-") {
        return ("line-clamp", "");
    }
    if core.starts_with("select-") {
        return ("select", "");
    }
    if core.starts_with("overscroll-") {
        return ("overscroll", "");
    }
    if core.starts_with("overflow-") {
        if core.starts_with("overflow-x-") {
            return ("overflow-x", "");
        }
        if core.starts_with("overflow-y-") {
            return ("overflow-y", "");
        }
        return ("overflow", "");
    }
    if core == "gap" || core.starts_with("gap-") {
        if core.starts_with("gap-x-") {
            return ("gap-x", "");
        }
        if core.starts_with("gap-y-") {
            return ("gap-y", "");
        }
        return ("gap", "");
    }
    if core == "size" || core.starts_with("size-") {
        return ("size", "");
    }
    if core == "w" || core.starts_with("w-") {
        return ("w", "");
    }
    if core == "h" || core.starts_with("h-") {
        return ("h", "");
    }
    if core.starts_with("min-w-") {
        return ("min-w", "");
    }
    if core.starts_with("max-w-") {
        return ("max-w", "");
    }
    if core.starts_with("min-h-") {
        return ("min-h", "");
    }
    if core.starts_with("max-h-") {
        return ("max-h", "");
    }
    if core.starts_with("px-") {
        return ("px", "");
    }
    if core.starts_with("py-") {
        return ("py", "");
    }
    if core.starts_with("pt-") {
        return ("pt", "");
    }
    if core.starts_with("pr-") {
        return ("pr", "");
    }
    if core.starts_with("pb-") {
        return ("pb", "");
    }
    if core.starts_with("pl-") {
        return ("pl", "");
    }
    if core.starts_with("ps-") {
        return ("ps", "");
    }
    if core.starts_with("pe-") {
        return ("pe", "");
    }
    if core == "p" || core.starts_with("p-") {
        return ("p", "");
    }
    if core.starts_with("mx-") {
        return ("mx", "");
    }
    if core.starts_with("my-") {
        return ("my", "");
    }
    if core.starts_with("mt-") {
        return ("mt", "");
    }
    if core.starts_with("mr-") {
        return ("mr", "");
    }
    if core.starts_with("mb-") {
        return ("mb", "");
    }
    if core.starts_with("ml-") {
        return ("ml", "");
    }
    if core.starts_with("ms-") {
        return ("ms", "");
    }
    if core.starts_with("me-") {
        return ("me", "");
    }
    if core == "m" || core.starts_with("m-") {
        return ("m", "");
    }
    if core == "rounded" {
        return ("rounded", "");
    }
    if let Some(rest) = core.strip_prefix("rounded-") {
        let segment = rest.split('-').next().unwrap_or(rest);
        if matches!(
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
        ) && !rest.starts_with('[')
        {
            return ("rounded-dir", segment);
        }
        return ("rounded", "");
    }
    if core == "border" {
        return ("border-width", "");
    }
    if let Some(rest) = core.strip_prefix("border-") {
        if matches!(
            rest,
            "solid" | "dashed" | "dotted" | "double" | "hidden" | "none"
        ) {
            return ("border-style", "");
        }
        for direction in ["t", "r", "b", "l", "x", "y"] {
            if rest == direction || rest.starts_with(&format!("{direction}-")) {
                return ("border-dir", direction);
            }
        }
        if matches!(rest, "0" | "2" | "4" | "8") {
            return ("border-width", "");
        }
        if rest.starts_with('[') {
            return match rest.as_bytes().get(1) {
                Some(first) if first.is_ascii_digit() => ("border-width", ""),
                _ => ("border-color", ""),
            };
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
        if rest.starts_with("gradient-") || rest.starts_with("linear-") || rest.starts_with('[') {
            return ("bg-image", "");
        }
        return ("bg-color", "");
    }
    if core == "inset" || core.starts_with("inset-") {
        if core.starts_with("inset-x-") {
            return ("inset-x", "");
        }
        if core.starts_with("inset-y-") {
            return ("inset-y", "");
        }
        return ("inset", "");
    }
    if core.starts_with("top-") {
        return ("top", "");
    }
    if core.starts_with("right-") {
        return ("right", "");
    }
    if core.starts_with("bottom-") {
        return ("bottom", "");
    }
    if core.starts_with("left-") {
        return ("left", "");
    }
    if core.starts_with("rotate-") {
        return ("rotate", "");
    }
    if core.starts_with("scale-") {
        return ("scale", "");
    }
    ("", base)
}

#[cfg(test)]
mod tests {
    use super::merge_classes;

    fn tokens(merged: &str) -> Vec<&str> {
        merged.split_whitespace().collect()
    }

    #[test]
    fn fast_path_returns_single_slice_verbatim() {
        assert_eq!(merge_classes(&["flex flex-col"]), "flex flex-col");
        assert_eq!(merge_classes(&["flex", ""]), "flex");
        assert_eq!(merge_classes(&["", "gap-4"]), "gap-4");
        assert_eq!(merge_classes(&[""]), "");
        assert_eq!(merge_classes(&[]), "");
    }

    #[test]
    fn card_library_landing_override_wins() {
        let merged = merge_classes(&[
            "bg-card text-card-foreground flex flex-col rounded-xl shadow-sm",
            "py-6 gap-4",
            "block bg-canvas p-0 text-lg",
        ]);
        let class = tokens(&merged);
        // Direct conflicts resolve last-wins: display, background, padding, text size.
        for kept in ["block", "bg-canvas", "p-0", "text-lg"] {
            assert!(class.contains(&kept), "{merged}");
        }
        for dropped in ["flex", "bg-card", "py-6"] {
            assert!(!class.contains(&dropped), "{merged}");
        }
        // Independent groups survive: direction, text color (no color override was
        // passed), radius, shadow and gap.
        for kept in [
            "text-card-foreground",
            "flex-col",
            "rounded-xl",
            "shadow-sm",
            "gap-4",
        ] {
            assert!(class.contains(&kept), "{merged}");
        }
    }

    #[test]
    fn button_overrides_replace_size_variant_and_colors() {
        let merged = merge_classes(&[
            "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm",
            "bg-primary text-primary-foreground shadow-xs",
            "h-control px-4 py-2",
            "w-fit h-9 px-2 bg-red-500 text-white gap-1",
        ]);
        let class = tokens(&merged);
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
            assert!(class.contains(&kept), "{merged}");
        }
        for dropped in [
            "h-control",
            "px-4",
            "bg-primary",
            "text-primary-foreground",
            "gap-2",
        ] {
            assert!(!class.contains(&dropped), "{merged}");
        }
    }

    #[test]
    fn icons_caller_size_supersedes_base() {
        assert_eq!(merge_classes(&["size-4", "size-3.5"]), "size-3.5");
        assert_eq!(
            merge_classes(&["size-4 animate-spin", "size-6"]),
            "animate-spin size-6"
        );
    }

    #[test]
    fn input_focus_ring_override_keeps_modifier_scope() {
        let merged = merge_classes(&["focus-visible:ring-2", "focus-visible:ring-[3px]", "ring-2"]);
        let class = tokens(&merged);
        assert!(class.contains(&"focus-visible:ring-[3px]"));
        assert!(!class.contains(&"focus-visible:ring-2"));
        assert!(class.contains(&"ring-2"));
    }

    #[test]
    fn input_group_button_gap_override_wins() {
        assert_eq!(
            merge_classes(&["text-sm shadow-none flex gap-2 items-center", "gap-1"]),
            "text-sm shadow-none flex items-center gap-1"
        );
    }

    #[test]
    fn responsive_variant_does_not_clash_with_base() {
        let merged = merge_classes(&["flex flex-col", "max-[760px]:flex"]);
        let class = tokens(&merged);
        assert!(class.contains(&"flex"));
        assert!(class.contains(&"max-[760px]:flex"));
        assert!(class.contains(&"flex-col"));
    }

    #[test]
    fn text_utilities_split_into_independent_groups() {
        let merged = merge_classes(&["text-sm text-foreground whitespace-nowrap", "text-xs"]);
        let class = tokens(&merged);
        assert!(class.contains(&"text-xs"));
        assert!(!class.contains(&"text-sm"));
        assert!(class.contains(&"whitespace-nowrap"));
        let merged = merge_classes(&["text-muted-foreground", "text-foreground"]);
        assert_eq!(merged, "text-foreground");
    }

    #[test]
    fn bracket_aware_modifiers_keep_exact_scope() {
        let merged = merge_classes(&["[&>svg]:size-4", "[&>svg]:size-3"]);
        assert_eq!(merged, "[&>svg]:size-3");
        let merged = merge_classes(&[
            "[&_tr:last-child]:border-0",
            "border",
            "dark:aria-invalid:ring-destructive/40",
        ]);
        let class = tokens(&merged);
        assert!(class.contains(&"[&_tr:last-child]:border-0"));
        assert!(class.contains(&"border"));
        assert!(class.contains(&"dark:aria-invalid:ring-destructive/40"));
    }

    #[test]
    fn spacing_supersession_follows_sides() {
        assert_eq!(merge_classes(&["px-6 py-6", "p-0"]), "p-0");
        assert_eq!(merge_classes(&["p-4", "px-2"]), "p-4 px-2");
        assert_eq!(merge_classes(&["pl-4 pr-4", "px-2"]), "px-2");
        assert_eq!(merge_classes(&["my-0", "mt-2"]), "my-0 mt-2");
        assert_eq!(merge_classes(&["m-4", "md:-mx-1"]), "m-4 md:-mx-1");
        let merged = merge_classes(&["-mx-1 my-1 h-px bg-border", "-mx-2"]);
        assert_eq!(merged, "my-1 h-px bg-border -mx-2");
    }

    #[test]
    fn rounded_border_background_groups() {
        assert_eq!(merge_classes(&["rounded-xl", "rounded-md"]), "rounded-md");
        assert_eq!(
            merge_classes(&["rounded-xl", "rounded-t-lg"]),
            "rounded-xl rounded-t-lg"
        );
        assert_eq!(merge_classes(&["rounded-t-lg", "rounded"]), "rounded");
        assert_eq!(merge_classes(&["border", "border-2"]), "border-2");
        assert_eq!(merge_classes(&["bg-card", "bg-cover"]), "bg-card bg-cover");
        assert_eq!(merge_classes(&["bg-card", "bg-canvas"]), "bg-canvas");
    }

    #[test]
    fn unknown_tokens_dedup_exactly() {
        // Exact duplicates collapse, keeping the last-occurrence position.
        assert_eq!(
            merge_classes(&["group/item flex", "group/item flex-none"]),
            "flex group/item flex-none"
        );
        assert_eq!(
            merge_classes(&["[-webkit-tap-highlight-color:transparent]", "select-none"]),
            "[-webkit-tap-highlight-color:transparent] select-none"
        );
    }

    #[test]
    fn later_conflicts_move_to_last_position() {
        assert_eq!(
            merge_classes(&["px-6 text-sm flex", "block"]),
            "px-6 text-sm block"
        );
    }

    #[test]
    fn size_collides_with_width_and_height() {
        assert_eq!(merge_classes(&["size-4", "w-2"]), "w-2");
        assert_eq!(merge_classes(&["w-2 h-2", "size-4"]), "size-4");
    }
}
