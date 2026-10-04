#!/usr/bin/env python3
"""Fixtures for check-wasm-tests.py."""

import importlib.util
from pathlib import Path
import tempfile
import unittest


SPEC = importlib.util.spec_from_file_location(
    "check_wasm_tests", Path(__file__).resolve().parent / "check-wasm-tests.py")
lint = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(lint)

WASM_ONLY_MAIN = '''
#[cfg(all(target_arch = "wasm32", feature = "page"))]
mod presentation;
#[cfg(all(feature = "page", test, not(target_arch = "wasm32")))]
mod stub {
    #[path = "native_ok.rs"]
    mod native_ok;
}
'''


class CheckWasmTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="check-wasm-tests-")
        self.root = Path(self.temp.name)
        (self.root / "web/src/presentation").mkdir(parents=True)
        (self.root / "web/src/lib.rs").write_text("")
        (self.root / "web/src/main.rs").write_text(WASM_ONLY_MAIN)
        (self.root / "web/src/presentation.rs").write_text("mod wasm_unit;\nmod wasm_bindgen_unit;\n")
        (self.root / "web/src/stub").mkdir()
        (self.root / "web/src/stub/native_ok.rs").write_text("#[test]\nfn runs() {}\n")

    def tearDown(self):
        self.temp.cleanup()

    def write(self, relative, text):
        path = self.root / "web/src" / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def violations(self):
        return lint.find_violations(self.root)

    def test_plain_test_in_wasm_only_file_fails_with_file_and_line(self):
        self.write("presentation/wasm_unit.rs", "fn a() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n")
        self.write("presentation/wasm_bindgen_unit.rs", "")
        self.assertEqual(self.violations(), [("web/src/presentation/wasm_unit.rs", 5)])

    def test_wasm_bindgen_test_and_other_runners_pass(self):
        self.write("presentation/wasm_unit.rs",
                   "#[wasm_bindgen_test]\nfn a() {}\n#[tokio::test]\nasync fn b() {}\n")
        self.write("presentation/wasm_bindgen_unit.rs", "#[wasm_bindgen_test::wasm_bindgen_test]\nfn c() {}\n")
        self.assertEqual(self.violations(), [])

    def test_natively_included_file_passes_even_when_also_in_wasm_tree(self):
        self.write("presentation/wasm_unit.rs", "#[test]\nfn a() {}\n")
        self.write("presentation/wasm_bindgen_unit.rs", "")
        main = self.root / "web/src/main.rs"
        main.write_text(main.read_text() + '#[cfg(test)]\n#[path = "presentation/wasm_unit.rs"]\nmod native_copy;\n')
        self.assertEqual(self.violations(), [])

    def test_native_file_with_wasm_gated_inline_test_module_fails(self):
        main = self.root / "web/src/main.rs"
        main.write_text(main.read_text() + '#[cfg(any(test, target_arch = "wasm32"))]\nmod shared;\n')
        self.write("shared.rs",
                   '#[test]\nfn ok() {}\n#[cfg(target_arch = "wasm32")]\nmod w {\n    #[test]\n    fn bad() {}\n}\n')
        self.write("presentation/wasm_unit.rs", "")
        self.write("presentation/wasm_bindgen_unit.rs", "")
        self.assertEqual(self.violations(), [("web/src/shared.rs", 5)])

    def test_strings_and_comments_do_not_confuse_the_scanner(self):
        self.write("presentation/wasm_unit.rs",
                   '// #[test]\nconst S: &str = "{ #[test] mod x; }";\n#[wasm_bindgen_test]\nfn a() { let _ = format!("}}"); }\n')
        self.write("presentation/wasm_bindgen_unit.rs", "")
        self.assertEqual(self.violations(), [])

    def test_baseline_tolerates_known_debt_but_rejects_increase(self):
        self.write("presentation/wasm_unit.rs", "#[test]\nfn a() {}\n")
        self.write("presentation/wasm_bindgen_unit.rs", "")
        baseline = self.root / "baseline.json"
        argv = ["--root", str(self.root), "--baseline", str(baseline)]
        self.assertEqual(lint.main(argv), 1)
        self.assertEqual(lint.main([*argv, "--write-baseline"]), 0)
        self.assertEqual(lint.main(argv), 0)
        self.write("presentation/wasm_unit.rs", "#[test]\nfn a() {}\n#[test]\nfn b() {}\n")
        self.assertEqual(lint.main(argv), 1)


if __name__ == "__main__":
    unittest.main(verbosity=2)
