#!/usr/bin/env python3
"""Check that the packaged gate batches every regression, including sliver."""
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from urllib.request import urlopen

import provider_pages as pages


class ProviderGateTests(unittest.TestCase):
    def test_full_inventory_uses_one_provider_invocation(self):
        def run_page(url, suites, *, timeout):
            self.assertEqual(suites, list(pages.SUITES))
            self.assertEqual(timeout, 900)
            with urlopen(url + 'tests/step-face-completeness.mjs') as response:
                source = response.read().decode()
            self.assertIn('every source face keeps analysis coverage', source)
            self.assertIn("case: 'sliver'", source)
            return {suite: [dict(suite=suite, case=case, ok=True) for case in pages.SUITES[suite]]
                    for suite in suites}

        with tempfile.TemporaryDirectory() as directory:
            artifact = Path(directory) / 'pkg'
            artifact.touch()
            with patch.object(pages, 'preflight', return_value=(artifact, artifact)), \
                    patch.object(pages.browser_tools, 'run_provider_page', side_effect=run_page) as run:
                pages.run()
            run.assert_called_once()


if __name__ == '__main__':
    unittest.main()
