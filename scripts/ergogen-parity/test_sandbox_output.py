import tempfile
import unittest
from pathlib import Path

import sandbox_generate


class StagedOutputTests(unittest.TestCase):
    def test_rejects_oversized_and_invalid_output(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'fixture.json'
            output.write_bytes(b'x' * (sandbox_generate.MAX_OUTPUT_BYTES + 1))
            with self.assertRaises(ValueError):
                sandbox_generate.read_staged_output(output)
            output.write_bytes(b'{}')
            with self.assertRaises(ValueError):
                sandbox_generate.read_staged_output(output)

    def test_accepts_existing_fixture(self):
        expected = sandbox_generate.generate.FIXTURE.read_bytes()
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'fixture.json'
            output.write_bytes(expected)
            self.assertEqual(sandbox_generate.read_staged_output(output), expected)

    def test_rejects_symlink(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'fixture.json'
            output.symlink_to(sandbox_generate.generate.FIXTURE)
            with self.assertRaises(OSError):
                sandbox_generate.read_staged_output(output)

    def test_prepared_tree_is_read_only(self):
        root = Path('/example')
        args = sandbox_generate.command(root, root, root / 'driver', root / 'chrome', root / 'out', True, root / 'tree')
        index = args.index('/prepared')
        self.assertEqual(args[index - 2:index + 1], ['--ro-bind', str(root / 'tree'), '/prepared'])
        self.assertIn('sys.exit(generate.main(prepared=generate.Path("/prepared")))', ' '.join(args))
        self.assertNotIn('--prepared-runtime', args)


if __name__ == '__main__':
    unittest.main()
