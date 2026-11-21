#!/usr/bin/env python3
"""Run all packaged CAD provider regressions in one Chrome session."""
import sys

import provider_pages


if __name__ == '__main__':
    try:
        provider_pages.run()
    except (OSError, ValueError, RuntimeError) as error:
        sys.exit(str(error))
