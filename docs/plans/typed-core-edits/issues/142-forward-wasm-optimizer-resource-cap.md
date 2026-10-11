# 142 — Forward the optional WASM optimizer resource cap

Status: claimed
Type: build
Blocked by: none

## Problem and scope

Laptop multi-stream mode limits workers to four. Actual integration CAD `wasm-opt` used17threads/about1015%CPU: the container command forwards `CARGO_BUILD_JOBS` but omits the independent `BINARYEN_CORES` setting. Root limited that live process's affinity without interrupting it. Do not change checked-in defaults or infer that container Cargo ignores its existing cap.

L5 owns the isolated `.worktrees/lane-resource-caps`, branch `codex/laptop-resource-caps`, based on integration6c2fd1a525. Modify only `cad/scripts/cadrum_build.py` and `cad/scripts/test-cadrum-build.py`: conditionally forward the host `BINARYEN_CORES` value before the image argument for both supported Podman and Docker commands. Preserve existing Cargo forwarding and behavior when unset. No container, CAD, browser or full suite execution is needed for this command-construction change unless the focused evidence identifies a specific gap.

## Acceptance

- A focused command-construction test fails because a supplied Binaryen cap is absent before the fix.
- With the variable set, both runtimes receive the exact explicit environment argument in the right position.
- When unset, no Binaryen override is added and existing command/default behavior remains intact.
- Existing CadrumBuildTests pass, including Cargo forwarding and other command options.
- Luna at the lowest sufficient effort implements; one fresh Sol6.1 medium independently accepts after writer STOP, reusing adequate writer evidence.
- Source-bound impact and complete final change-analysis gates apply; root alone refreshes/stages/integrates. Focused tests obtain a scheduled HEAVY window. Use laptop caps, including `BINARYEN_CORES=4`.

## Evidence and exclusions

L5 read-only diagnosis confirms `check.py` copies the host environment and host build/dev subprocesses inherit it. Fast provider commands disable optimization only for direct wasm-pack invocations; nested CAD optimization remains. Binaryen's worker environment is documented at [upstream](https://github.com/WebAssembly/binaryen/wiki/Debugging). No broad check-speed audit, new runtime configuration or unrelated runner changes are included.
