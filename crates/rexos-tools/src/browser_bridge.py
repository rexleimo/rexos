#!/usr/bin/env python3
"""RexOS Browser Bridge — Playwright automation over JSON-lines stdio.

This is a small helper used by RexOS `browser_*` tools. Rust launches this
script as a subprocess, then sends commands as JSON (1 per line) to stdin.
The bridge replies with JSON (1 per line) on stdout.
"""

import argparse
import base64
import json
import sys


def main() -> int:
    parser = argparse.ArgumentParser(description="RexOS Browser Bridge")
    parser.add_argument("--headless", action="store_true", default=True)
    parser.add_argument("--no-headless", dest="headless", action="store_false")
    parser.add_argument("--width", type=int, default=1280)
    parser.add_argument("--height", type=int, default=720)
    parser.add_argument("--timeout", type=int, default=30)
    args = parser.parse_args()

    timeout_ms = args.timeout * 1000

    try:
        from playwright.sync_api import sync_playwright
    except ImportError:
        respond(
            {
                "success": False,
                "error": "playwright not installed. Run: python -m pip install playwright && python -m playwright install chromium",
            }
        )
        return 1

    pw = sync_playwright().start()
    browser = pw.chromium.launch(headless=args.headless)
    context = browser.new_context(viewport={"width": args.width, "height": args.height})
    page = context.new_page()
    page.set_default_timeout(timeout_ms)
    page.set_default_navigation_timeout(timeout_ms)

    respond({"success": True, "data": {"status": "ready"}})

    current_url = ""
    try:
        for line in sys.stdin:
            line = line.strip()
            if not line:
                continue
            try:
                cmd = json.loads(line)
            except Exception as e:
                respond({"success": False, "error": f"invalid json: {e}"})
                continue

            action = cmd.get("action", "")
            try:
                if action == "Navigate":
                    url = cmd.get("url", "")
                    if not url:
                        respond({"success": False, "error": "missing url"})
                        continue
                    page.goto(url, wait_until="domcontentloaded", timeout=timeout_ms)
                    current_url = page.url
                    respond({"success": True, "data": {"title": page.title(), "url": current_url}})
                elif action == "Back":
                    try:
                        page.go_back(wait_until="domcontentloaded", timeout=timeout_ms)
                    except Exception:
                        pass
                    current_url = page.url if page.url else current_url
                    respond({"success": True, "data": {"title": page.title(), "url": current_url}})
                elif action == "Scroll":
                    direction = cmd.get("direction", "down")
                    amount = cmd.get("amount", 600)
                    try:
                        amount = int(amount)
                    except Exception:
                        amount = 600

                    dx, dy = 0, 0
                    if direction == "up":
                        dy = -amount
                    elif direction == "down":
                        dy = amount
                    elif direction == "left":
                        dx = -amount
                    elif direction == "right":
                        dx = amount

                    pos = page.evaluate(
                        "({dx, dy}) => { window.scrollBy(dx, dy); return {scrollX: window.scrollX || 0, scrollY: window.scrollY || 0}; }",
                        {"dx": dx, "dy": dy},
                    )
                    respond({"success": True, "data": pos})
                elif action == "Click":
                    selector = cmd.get("selector", "")
                    if not selector:
                        respond({"success": False, "error": "missing selector"})
                        continue
                    try:
                        page.click(selector, timeout=timeout_ms)
                    except Exception:
                        page.get_by_text(selector, exact=False).first.click(timeout=timeout_ms)
                    page.wait_for_load_state("domcontentloaded", timeout=timeout_ms)
                    current_url = page.url
                    respond(
                        {
                            "success": True,
                            "data": {"clicked": selector, "title": page.title(), "url": current_url},
                        }
                    )
                elif action == "Type":
                    selector = cmd.get("selector", "")
                    text = cmd.get("text", "")
                    if not selector:
                        respond({"success": False, "error": "missing selector"})
                        continue
                    page.fill(selector, text, timeout=timeout_ms)
                    respond({"success": True, "data": {"selector": selector, "typed": text}})
                elif action == "PressKey":
                    key = cmd.get("key", "")
                    selector = cmd.get("selector", "")
                    if not key:
                        respond({"success": False, "error": "missing key"})
                        continue
                    if selector:
                        page.press(selector, key, timeout=timeout_ms)
                    else:
                        page.keyboard.press(key)
                    try:
                        page.wait_for_load_state("domcontentloaded", timeout=timeout_ms)
                    except Exception:
                        pass
                    current_url = page.url
                    respond(
                        {
                            "success": True,
                            "data": {
                                "key": key,
                                "selector": selector or None,
                                "title": page.title(),
                                "url": current_url,
                            },
                        }
                    )
                elif action == "WaitFor":
                    selector = cmd.get("selector", "")
                    text = cmd.get("text", "")
                    per_call_timeout = cmd.get("timeout_ms")
                    try:
                        per_call_timeout = int(per_call_timeout) if per_call_timeout else timeout_ms
                    except Exception:
                        per_call_timeout = timeout_ms

                    waited_for = {}
                    if selector:
                        page.wait_for_selector(selector, timeout=per_call_timeout)
                        waited_for["selector"] = selector
                    if text:
                        page.get_by_text(text, exact=False).first.wait_for(
                            state="visible", timeout=per_call_timeout
                        )
                        waited_for["text"] = text

                    if not waited_for:
                        respond({"success": False, "error": "missing selector/text"})
                        continue

                    current_url = page.url if page.url else current_url
                    respond(
                        {
                            "success": True,
                            "data": {
                                "waited_for": waited_for,
                                "timeout_ms": per_call_timeout,
                                "title": page.title(),
                                "url": current_url,
                            },
                        }
                    )
                elif action == "RunJs":
                    expression = cmd.get("expression", "")
                    if not expression:
                        respond({"success": False, "error": "missing expression"})
                        continue

                    result = page.evaluate(expression)
                    try:
                        json.dumps(result)
                    except Exception:
                        result = str(result)

                    current_url = page.url if page.url else current_url
                    respond({"success": True, "data": {"result": result, "url": current_url}})
                elif action == "ReadPage":
                    content = "(empty)"
                    try:
                        content = page.inner_text("body")
                    except Exception:
                        pass
                    max_chars = 50_000
                    if len(content) > max_chars:
                        content = content[:max_chars] + f"\n\n[Truncated — {len(content)} total chars]"
                    current_url = page.url if page.url else current_url
                    respond(
                        {
                            "success": True,
                            "data": {"title": page.title(), "url": current_url, "content": content},
                        }
                    )
                elif action == "Screenshot":
                    screenshot_bytes = page.screenshot(full_page=False)
                    b64 = base64.b64encode(screenshot_bytes).decode("utf-8")
                    current_url = page.url if page.url else current_url
                    respond(
                        {
                            "success": True,
                            "data": {
                                "format": "png",
                                "url": current_url,
                                "image_base64": b64,
                            },
                        }
                    )
                elif action == "Close":
                    respond({"success": True, "data": {"status": "closed"}})
                    break
                else:
                    respond({"success": False, "error": f"unknown action: {action}"})
            except Exception as e:
                respond({"success": False, "error": f"{type(e).__name__}: {e}"})
    finally:
        try:
            context.close()
            browser.close()
            pw.stop()
        except Exception:
            pass

    return 0


def respond(obj) -> None:
    sys.stdout.write(json.dumps(obj) + "\n")
    sys.stdout.flush()


if __name__ == "__main__":
    raise SystemExit(main())
