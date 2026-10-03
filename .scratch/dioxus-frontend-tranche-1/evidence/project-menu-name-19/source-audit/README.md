# Project-menu name source audit

Fresh isolated Headless Chrome 154 sessions compared the pinned React oracle at `http://127.0.0.1:5173/` with the Dioxus root package at `http://127.0.0.1:34736/`. Both imported the same layered Sofle archive, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`, and used a 1280×577 viewport.

React source is pinned to `5a472a9426e6e38993361da402cd4ec730feb369`; it rendered the loaded Project menu with visible `Project name`, title `Rename project`, value `Sofle v2`. The candidate used root-reported source `d7ff5e3dcae67a719caf6660cfcff57c3811df34`; opening its `details.m1-project-menu` showed the library menu and no input labelled Project name. See paired snapshots, DOM observations, and captures in this directory.

This is a source-gap audit only; no project was renamed and it is not a post-port acceptance run.
