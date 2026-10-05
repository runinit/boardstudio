# F7.6 test packet preparation receipt

Prepared against `bda42cae76b342147330f971c108711b80549038`.

| File | Baseline SHA-256 |
|---|---|
| `web/src/runtime.rs` | `5d94109e52827ee05831c154329ec4af96a30bd6286c20447f543fcf77fc410f` |
| `web/src/presentation.rs` | `fae911b0f35c6d935c6ce512319b2ef8b83b70e8e5a3a34c3a63f7defacc7b5c` |
| `web/src/cad_presentation.rs` | `d7009cad80c088584ced010b3735220d0b2092801a5bcf473e5411216f2e9495` |

Prepared copies are under `copies/web/src/`. The applicable unified diff is `F76_REQUIRED_TESTS_READY.patch` (three paths only). It contains private cfg(test) fixtures/bridge and the two required mounted Case tests; no production behavior is changed. `git apply --check F76_REQUIRED_TESTS_READY.patch` passed on the clean owned paths. No live source was edited; no tests, compiler, package, or browser action was run. The prior `F76_REQUIRED_TESTS.patch` was preserved unchanged.
