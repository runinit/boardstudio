# 005: How Themes and Appearance are structured

Status: open
Type: grilling
Blocked by: [002](002-rust-ui-catalogue-facts.md)
Parent: [Map](../map.md)

## Question

How do two Themes × light/dark Appearance reach components: which colour, radius and
font names form the contract (rust-ui's set, plus any BoardStudio additions such as
canvas or selection colours); how a Theme and Appearance are applied to the page; how the
preference is stored next to Appearance; and how the picker presents both choices (it lives in the header, [ADR-0035](../../../adr/0035-rebuild-the-workbench-shell-on-rust-ui-patterns.md)). Also: control sizing. Ours are 40px (36px compact, 44px touch); rust-ui's default is 36px and its 40px size is `Lg`, so does the Original Theme tighten them or keep today's sizes through size tokens?
