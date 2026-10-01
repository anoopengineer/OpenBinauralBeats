#!/usr/bin/env python3
"""Static checks for website/: internal links resolve, JSON-LD parses, one <h1> and a <title> per page."""
import glob, json, os, re, sys

ROOT = os.path.join(os.path.dirname(__file__), "..", "website")
os.chdir(ROOT)
errors = []
pages = sorted(glob.glob("*.html") + glob.glob("*/index.html"))

for page in pages:
    s = open(page, encoding="utf-8").read()
    for block in re.findall(r'<script type="application/ld\+json">(.*?)</script>', s, re.S):
        try:
            json.loads(block)
        except json.JSONDecodeError as e:
            errors.append(f"{page}: invalid JSON-LD: {e}")
    if not re.search(r"<title>.+?</title>", s):
        errors.append(f"{page}: missing <title>")
    if len(re.findall(r"<h1[ >]", s)) != 1:
        errors.append(f"{page}: expected exactly one <h1>")
    for ref in re.findall(r'(?:href|src)="(/[^"#?]*)', s):
        if ref.startswith("/download/"):
            continue  # Netlify redirects, see netlify.toml
        path = ref.lstrip("/")
        if not (os.path.isfile(path) or os.path.isfile(os.path.join(path, "index.html"))):
            errors.append(f"{page}: broken link {ref}")

for loc in re.findall(r"<loc>https://openbinauralbeats\.com/(.*?)</loc>", open("sitemap.xml").read()):
    if not os.path.isfile(os.path.join(loc, "index.html")):
        errors.append(f"sitemap.xml: {loc or '/'} has no page")

print(f"checked {len(pages)} pages")
for e in errors:
    print("ERROR", e)
sys.exit(1 if errors else 0)
