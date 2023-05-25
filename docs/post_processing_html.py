"""Post-processing HTML script for rendering non-duplicated contributors
cd docs
pnpm docs:build
python3 post_processing_html.py
"""
import json
import os
from subprocess import run


# Contributors files that contain `const e`
e_contributors_files: list = []

# Contributors files that contain `const t`
t_contributors_files: list = []

# Change working directory
os.chdir("src/.vuepress/dist/assets")


def find_files() -> tuple[list]:
    """Return js files that contain 'contributors'"""

    # Run grep -ril 'contributors' inside os.chdir()
    command: list = ["grep", "-ril", "contributors"]
    output: str = run(command, capture_output=True).stdout.decode("utf-8")

    for out in output.splitlines():
        if "html" in out and ".js" in out:
            # Append contributors files that contain `const e`
            with open(out, "r") as f:
                if "const e" in f.read():
                    e_contributors_files.append(out)
            # Append contributors files that contain `const t`
            with open(out, "r") as f:
                if "const t" in f.read():
                    t_contributors_files.append(out)

    # Return file names that contain "contributors"
    return e_contributors_files, t_contributors_files


def json_source(raw_source: str) -> dict:
    """Load raw source file of contributor file"""

    # Decompose JS raw source file
    if "const e=JSON.parse('" in raw_source:
        raw_source = (
            raw_source.replace("const e=JSON.parse('", "")
            .replace("');export{e as data};", "")
            .replace("\\\\", "\\")
        )

    if "const e=JSON.parse(`" in raw_source:
        raw_source = (
            raw_source.replace("const e=JSON.parse(`", "")
            .replace("`);export{e as data};", "")
            .replace("\\\\", "\\")
        )

    if "const t=JSON.parse('" in raw_source:
        raw_source = (
            raw_source.replace("const t=JSON.parse('", "")
            .replace("');export{t as data};", "")
            .replace("\\\\", "\\")
        )

    if "const t=JSON.parse(`" in raw_source:
        raw_source = (
            raw_source.replace("const t=JSON.parse(`", "")
            .replace("`);export{t as data};", "")
            .replace("\\\\", "\\")
        )

    # Load processed raw source into a JSON object
    json_source = json.loads(raw_source)

    # Return JSON object
    return json_source


def get_raw_contributors(json_source: dict) -> list:
    """Get non-processed contributors from JSON source"""

    # Get raw contributors from JSON source
    raw_contributors = json_source["git"]["contributors"]

    # Return raw contributors from JSON source
    return raw_contributors


def remove_duplicated_contributors(contributors: list) -> list[dict]:
    """Remove duplicated contributors in the JSON object"""

    # Remove duplicated contributors
    contributors: list = list({d["name"]: d for d in contributors[::-1]}.values())

    # Return removal of duplicated contributors
    return contributors


def sort_contributors_by_commits(contributors: list) -> list:
    """Sort contributors by number of commits"""

    # Get sorted contributors by number of commits
    sorted_contributors: list = sorted(contributors, key=lambda d: d["commits"])

    # Return sorted contributors by number of commits
    return sorted_contributors


def append_contributors(json_source: dict, contributors: list) -> str:
    """Allows us to append what kind of processed contributors"""

    # Append processed contributors to actual JSON object
    json_source["git"]["contributors"] = contributors

    # Return
    return json.dumps(json_source)


def e_compose_source(dumped: str) -> str:
    """Compose `const e` source file"""

    js_source = "const e=JSON.parse('" + dumped + "');export{e as data};"
    js_source = js_source.replace("\\", "\\\\")
    return js_source


def t_compose_source(dumped: str) -> str:
    """Compose `const f` source file"""

    js_source = "const t=JSON.parse('" + dumped + "');export{t as data};"
    js_source = js_source.replace("\\", "\\\\")
    return js_source


def inject(f, composed_source: str) -> bool:
    """Inject composed source back into contributors files"""

    # Move cleaning position
    f.seek(0)

    # Clean before injection
    f.truncate(0)

    # Inject the composed_source
    f.write(composed_source)

    return True


if __name__ == "__main__":

    # Get corresponding file names
    find_files()

    # Start post-processing for `const e` contributor files
    for e_contributor_file in e_contributors_files:

        # Open file in read + write mode
        with open(e_contributor_file, "r+") as f:
            raw_source = f.read()
            js_source = json_source(raw_source)

            raw_contributors = get_raw_contributors(js_source)

            # Avoid extra processing by checking if there are more than 1 contributor
            if len(raw_contributors) > 1:
                contributors = remove_duplicated_contributors(raw_contributors)
                dumped = append_contributors(js_source, contributors)

                # Peform the injection
                inject(f, e_compose_source(dumped))

    # Start post-processing for `const t` contributor files
    for t_contributor_file in t_contributors_files:

        # Open file in read + write mode
        with open(t_contributor_file, "r+") as f:
            raw_source = f.read()
            js_source = json_source(raw_source)

            raw_contributors = get_raw_contributors(js_source)

            # Avoid extra processing by checking if there are more than 1 contributor
            if len(raw_contributors) > 1:
                contributors = remove_duplicated_contributors(raw_contributors)
                dumped = append_contributors(js_source, contributors)

                # Peform the injection
                inject(f, t_compose_source(dumped))
