#!/usr/bin/env python3
"""Hermetic checks for the packaged smoke's HTTP, W3C input and CLI contracts."""
import contextlib
import importlib
import io
from pathlib import Path
import tempfile
import unittest
from unittest.mock import Mock, call, patch
from urllib.request import urlopen

drag = importlib.import_module("run-drag-browser")


class DragBrowserTests(unittest.TestCase):
    def test_collision_setup_waits_for_durable_configuration_and_mount_before_generation(self):
        smoke = drag.Smoke(Mock())
        initial = {"revision": 3}
        configured = {"revision": 4, "mechanical": {"mount": "tray"}}
        rigid = {"revision": 5, "mechanical": {"mount": "rigid"}}
        events = []
        smoke.project = Mock(return_value=initial)
        smoke.button = Mock(side_effect=lambda name: events.append(name))
        smoke.evaluate = Mock(side_effect=["true", "false"])
        smoke.click = Mock(side_effect=lambda selector: events.append(selector))
        smoke.select_option = Mock(side_effect=lambda name, value: events.append((name, value)))
        smoke.persisted_after = Mock(side_effect=lambda revision: {
            3: configured, 4: rigid,
        }[revision])
        smoke.prepare_collision()
        self.assertEqual(events, ["Open Case workspace", '[aria-label="Live preview"]', "Configure mechanical stack",
                                  ("Mount style", "Rigid mount"), "Update preview"])
        self.assertEqual(smoke.persisted_after.call_args_list, [call(3), call(4)])
        self.assertEqual(drag.mechanical_mounts({"hardware": {"instances": [
            {"mechanical": {"mount": "rigid"}}, {"mechanical": None},
        ]}}), ["rigid"])

    def test_service_worker_is_stubbed_without_changing_other_assets(self):
        with tempfile.TemporaryDirectory() as directory:
            site = Path(directory)
            page = site / "site-subpath/boardstudio"
            page.mkdir(parents=True)
            (page / "service-worker.js").write_text("precache()")
            (page / "index.html").write_text("<h1>BoardStudio</h1>")
            served = set()
            with drag.browser_tools.serve(drag.handler_for(site, served)) as url:
                for route in ("service-worker.js", "service-worker.js?rev=1"):
                    with urlopen(f"{url}/boardstudio/{route}") as response:
                        self.assertEqual(response.status, 200)
                        self.assertEqual(response.headers.get_content_type(), "text/javascript")
                        self.assertEqual(response.headers["Cross-Origin-Embedder-Policy"], "require-corp")
                        self.assertEqual(response.read(), b"")
                with urlopen(url + "/boardstudio/") as response:
                    self.assertEqual(response.read(), b"<h1>BoardStudio</h1>")
            self.assertEqual(served, {"/boardstudio/"})

    def test_ctrl_is_released_during_a_held_pointer_drag(self):
        browser = Mock()
        smoke = drag.Smoke(browser)
        smoke.point = Mock(return_value=(100, 100))
        selector = '[aria-label="Drag to offset column"]'
        events = [
            dict(type="pointerdown", trusted=True, pointer="mouse", ctrl=True, handle="Drag to offset column"),
            dict(type="pointermove", trusted=True, ctrl=False, buttons=1),
            dict(type="pointermove", trusted=True, ctrl=False, buttons=1),
            dict(type="pointerup", trusted=True),
        ]
        smoke.evaluate = Mock(side_effect=[["scene moved"], True, events])
        smoke.gesture("Ctrl Offset handle", selector, [], ctrl=True)

        actions = browser.actions.call_args_list
        self.assertEqual(len(actions), 3)
        self.assertEqual(actions[0], call({"actions": [{"type": "key", "id": "key",
                                                        "actions": [{"type": "keyDown", "value": "\ue009"}]}]}))
        pointer, keyboard = actions[1].args[0]["actions"]
        release, = actions[2].args[0]["actions"]
        for source in (pointer, release):
            self.assertEqual(source["type"], "pointer")
            self.assertEqual(source["id"], "pointer")
            self.assertEqual(source["parameters"], {"pointerType": "mouse"})
        self.assertEqual([event["type"] for event in pointer["actions"]],
                         ["pointerMove", "pointerDown", "pause", *["pointerMove"] * 4])
        self.assertEqual([(event["x"], event["y"]) for event in pointer["actions"][3:]],
                         [(108, 105), (117, 110), (126, 116), (136, 122)])
        self.assertEqual(keyboard["id"], "key")
        self.assertEqual([event["type"] for event in keyboard["actions"]], ["pause", "pause", "keyUp"])
        self.assertEqual(keyboard["actions"][2]["value"], "\ue009")
        self.assertEqual(release["actions"], [{"type": "pointerUp", "button": 0}])

    def test_cli_defaults_to_all_and_propagates_a_failure(self):
        with patch.object(drag, "run") as run:
            for argv, suite in (([], "all"), (["--suite", "drag"], "drag"), (["--suite", "collision"], "collision")):
                with self.subTest(suite=suite):
                    self.assertEqual(drag.main(argv), 0)
                    self.assertEqual(run.call_args, call(suite, drag.SITE))
            with self.assertRaises(SystemExit), contextlib.redirect_stderr(io.StringIO()):
                drag.main(["--suite", "unknown"])
            run.side_effect = RuntimeError("collision did not complete")
            with contextlib.redirect_stderr(io.StringIO()) as stderr:
                self.assertEqual(drag.main([]), 1)
            self.assertIn("FAIL collision did not complete", stderr.getvalue())


if __name__ == "__main__":
    unittest.main()
