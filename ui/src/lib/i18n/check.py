# Checks the locale files for missing or extra keys to the template

import json
import os
import pathlib

template = "_template.json"
files = ["en.json", "es.json"]

translations = os.path.dirname(__file__)


def main():
    with pathlib.Path(translations, template).open() as fp:
        content = json.load(fp)
    templateKeys = extract_keys(content)
    templateKeys = set(templateKeys)

    for file in files:
        path = pathlib.Path(translations, file)
        with path.open() as fp:
            content = json.load(fp)
        keys = extract_keys(content)
        keys = set(keys)
        if templateKeys != keys:
            print("Error in", file)
            missing = templateKeys.difference(keys)
            extra = keys.difference(templateKeys)
            if len(missing) > 0:
                print("Missing keys:", missing)
            if len(extra) > 0:
                print("Extra keys:", extra)
            print()


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


if __name__ == "__main__":
    main()
