# Checks the locale files for missing or extra keys to the template

import json
import os
import pathlib
import re

template = "_template.json"

translations = os.path.dirname(__file__)


def check_template(template_keys):
    files = [
        file
        for file in os.listdir(translations)
        if file.endswith(".json") and not file.startswith("_")
    ]

    error = False
    for file in files:
        path = pathlib.Path(translations, file)
        with path.open() as fp:
            content = json.load(fp)
        keys = extract_keys(content)
        keys = set(keys)
        if template_keys != keys:
            error = True
            print("Error in", file)
            missing = template_keys.difference(keys)
            extra = keys.difference(template_keys)
            if len(missing) > 0:
                print("Missing keys:", missing)
            if len(extra) > 0:
                print("Extra keys:", extra)
            print()
    return error


def extract_keys(obj: dict, prefix: str = "", array: list = None):
    if array is None:
        array = []
    for k, v in obj.items():
        name = f"{prefix}.{k}" if len(prefix) > 0 else str(k)
        if isinstance(v, dict):
            extract_keys(v, name, array)
        else:
            array.append(name)
    return array


html_templates = os.path.join(translations, "..", "templates")

# Used but, not directly
safelist = set(
    [
        "_lang",
        "r.convertSelector.default",
        "r.convertSelector.metric",
        "r.convertSelector.imperial",
        "openInEditor.error",
        "openInEditor.success",
    ]
)


def check_uses(template_keys):
    error = False
    re_usages = re.compile(r"\bt\([\"'](\w+(?:.\w+)*)[\"']")
    all_uses = set(safelist)

    for root, _, tmpls in os.walk(html_templates):
        for tmpl in tmpls:
            path = pathlib.Path(root, tmpl)
            with path.open(encoding="utf8") as fp:
                content = fp.read()
            uses = re_usages.findall(content)
            for use in uses:
                all_uses.add(use)
    unused = template_keys.difference(all_uses)
    if len(unused) > 0:
        error = True
        print("Unused keys:", unused)
    not_found = all_uses.difference(template_keys)
    if len(not_found) > 0:
        error = True
        print("Not found keys:", not_found)
    return error


def main():
    with pathlib.Path(translations, template).open() as fp:
        content = json.load(fp)
    template_keys = extract_keys(content)
    template_keys = set(template_keys)
    error = False
    error |= check_template(template_keys)
    error |= check_uses(template_keys)
    if error:
        exit(1)


if __name__ == "__main__":
    main()
