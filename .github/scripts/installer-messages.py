#!/usr/bin/env python3
"""Export literal installer messages from the shared application catalogs."""

import argparse
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MESSAGES = {
    "installer-welcome-title": "WelcomeLabel1",
    "installer-welcome-description": "WelcomeLabel2",
    "installer-launch": "LaunchProgram",
}


def export(output: Path):
    sections = {"Messages": [], "CustomMessages": []}
    for language, name in (("en", "english"), ("ru", "russian"), ("es", "spanish")):
        catalog = dict(line.split(" = ", 1) for line in (ROOT / "locales" / f"{language}.ftl").read_text(encoding="utf-8").splitlines() if " = " in line and not line.startswith(" "))
        for key, target in MESSAGES.items():
            text = catalog[key]
            if not text.strip() or any(symbol in text for symbol in "{}\r\n"):
                raise ValueError(f"Installer messages must contain literal single-line text: {language}/{key}")
            section = "CustomMessages" if key == "installer-launch" else "Messages"
            sections[section].append(f"{name}.{target}={text}")
    output.write_text("\n\n".join(f"[{name}]\n" + "\n".join(values) for name, values in sections.items()) + "\n", encoding="utf-8-sig")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    export(parser.parse_args().output)
