"""Check the built docs chart after client-side navigation, not only SSR.

Run with a Python environment containing Playwright and system Chromium:
  python scripts/hero.browser-test.py
The server uses an ephemeral loopback port and shuts down when the test ends.
"""
import functools
import json
from pathlib import Path
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from threading import Thread
from urllib.parse import unquote, urlsplit

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parent.parent
DIST = ROOT / "docs/.vitepress/dist"


class DocsHandler(SimpleHTTPRequestHandler):
    def translate_path(self, path):
        relative = unquote(urlsplit(path).path).removeprefix("/repomap")
        local = Path(super().translate_path(relative))
        if not local.exists() and local.with_suffix(".html").is_file():
            local = local.with_suffix(".html")
        return str(local)

    def log_message(self, *args):
        pass


def main():
    assert (DIST / "index.html").is_file(), "Run bun run docs:build first"
    server = ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(DocsHandler, directory=str(DIST)))
    thread = Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        origin = f"http://127.0.0.1:{server.server_port}"
        with sync_playwright() as playwright:
            browser = playwright.chromium.launch(executable_path="/usr/bin/chromium", args=["--no-sandbox"])
            try:
                for mode, colors in (
                    ("light", ["rgb(42, 120, 214)", "rgb(235, 104, 52)", "rgb(27, 175, 122)"]),
                    ("dark", ["rgb(57, 135, 229)", "rgb(217, 89, 38)", "rgb(25, 158, 112)"]),
                ):
                    context = browser.new_context(color_scheme=mode, viewport={"width": 1000, "height": 900})
                    page = context.new_page()
                    page.goto(origin + "/repomap/benchmarks", wait_until="networkidle")
                    # Vue hydration must finish before assigning the reload sentinel.
                    page.wait_for_function("!!document.querySelector('.VPNavBarTitle a')")
                    page.evaluate("window.heroNavigationSentinel = 'preserved'")
                    page.locator(".VPNavBarTitle a").click()
                    page.wait_for_url(origin + "/repomap/")
                    chart = page.locator("svg.localization-chart")
                    chart.wait_for(state="visible")
                    assert page.evaluate("window.heroNavigationSentinel") == "preserved", "Home navigation reloaded the document"
                    # Production docs force dark; explicitly exercise both theme-class states.
                    page.evaluate("mode => document.documentElement.classList.toggle('dark', mode === 'dark')", mode)
                    assert chart.locator("style").count() == 0, "Docs should use the imported theme stylesheet"
                    fills = chart.locator("g.bar > path").evaluate_all("els => els.slice(0, 3).map(el => getComputedStyle(el).fill)")
                    assert fills == colors, f"{mode} bar fills: {fills}"
                    for bar in chart.locator("g.bar > path").all():
                        box = bar.bounding_box()
                        assert box and box["width"] > 0 and box["height"] > 0
                    second = chart.locator("g.bar").nth(1)
                    tooltip = second.locator("text.tooltip")
                    assert tooltip.evaluate("el => getComputedStyle(el).opacity") == "0"
                    chart.locator("g.bar").first.focus()
                    page.keyboard.press("Tab")
                    assert second.evaluate("el => el === document.activeElement"), "Keyboard did not focus the semble bar"
                    assert tooltip.evaluate("el => getComputedStyle(el).opacity") == "1", "Keyboard value remained hidden"
                    assert tooltip.text_content() == "34.0%"
                    assert chart.locator("g.bar").count() == 9
                    print(json.dumps({"mode": mode, "client_navigation": True, "fills": fills, "keyboard_value": tooltip.text_content()}), flush=True)
                    context.close()
            finally:
                browser.close()
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


if __name__ == "__main__":
    main()
