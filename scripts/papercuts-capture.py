#!/usr/bin/python3
"""Fail-open entrypoint for the local Pronto Papercuts corpus."""

from papercuts_capture.runtime import fail_open_warning, hook_warning, main


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except SystemExit:
        raise
    except Exception as error:
        # Hooks are deliberately fail-open. If even the health/spool path is
        # unusable, emit one sanitized coded warning and still exit successfully.
        try:
            hook_warning("UserPromptSubmit", fail_open_warning(error))
        except Exception:
            pass
        raise SystemExit(0)
