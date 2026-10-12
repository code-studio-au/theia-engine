#!/usr/bin/env python3
"""Extract the numbered worldbuilding question prompts from the original PDF.
Usage: python tools/extract_theia_questions.py "Create MMO Story.pdf"
Requires: poppler-utils (pdftotext). No Python dependencies.
This is a SOURCE INDEX, not an automated confirmation of selected answers.
"""
from pathlib import Path
import re
import subprocess
import sys

if len(sys.argv) != 2:
    raise SystemExit('Usage: python tools/extract_theia_questions.py "Create MMO Story.pdf"')
source = Path(sys.argv[1])
if not source.is_file():
    raise SystemExit(f"Source not found: {source}")
result = subprocess.run(
    ["pdftotext", "-layout", str(source), "-"],
    check=True, capture_output=True, text=True
)
full = result.stdout
start = full.find("Stage 1 — The foundations of the world")
if start < 0:
    raise SystemExit("Cannot locate collaborative Stage 1. Verify PDF source.")
text = full[start:]
pat = re.compile(r"(?m)^\s*(\d{1,3})\.\s+([^\n]+\?)\s*$")
matches = list(pat.finditer(text))
questions = {}
for i, match in enumerate(matches):
    number = int(match.group(1))
    if number > 444 or number in questions:
        continue
    limit = matches[i + 1].start() if i + 1 < len(matches) else len(text)
    fragment = text[match.end():min(limit, match.end() + 3500)]
    fragment = re.sub(r"(?m)^\s*https://chatgpt\.com/[^\n]*$", "", fragment)
    options, active = [], ""
    for line in fragment.splitlines():
        stripped = line.strip()
        found = re.match(r"^([A-E])\.\s+(.+)", stripped)
        if found:
            if active:
                options.append(active)
            active = found.group(1) + ". " + found.group(2)
            if len(options) >= 5:
                break
        elif active and not stripped and active.startswith("E. "):
            break
        elif active and stripped and len(line) - len(line.lstrip()) >= 5:
            if len(active) < 400 and not re.match(r"^\d+\.", stripped):
                active += " " + stripped
    if active:
        options.append(active)
    page = full[:start + match.start()].count("\f") + 1
    questions[number] = (match.group(2).strip(), options[:5], page)

lines = [
    "# THEIA — ORIGINAL QUESTION PROMPTS Q001–Q444", "",
    f"**Source:** {source.name} · 403-page original chat export", "",
    "> These are question prompts and available options, NOT confirmed answers.",
    "> Question numbering was reused in a few later stages. Later explicit",
    "> corrections and user decisions take precedence over form defaults.", "",
]
for number in range(1, 445):
    if number not in questions:
        lines.extend([f"## Q{number} — not found", ""])
        continue
    title, options, page = questions[number]
    lines.extend([f"## Q{number}. {title}", f"*PDF page {page}*", ""])
    lines.extend("- " + option for option in options)
    lines.append("")
out = Path("docs/THEIA_ORIGINAL_QUESTIONS_Q001-Q444.md")
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text("\n".join(lines), encoding="utf-8")
print(f"Wrote {out}: {len(questions)} questions")
