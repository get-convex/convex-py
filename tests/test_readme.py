from pathlib import Path

from convex.values import ConvexMap, ConvexSet, Id


def test_readme_duplicates_docstrings() -> None:
    # The README duplicates content from docstrings. Make sure the content matches
    # even if formatting is a little different.
    # Feel free to delete this test if this content is diverging significantly,
    # this test is meant to prevent it from diverging accidently.

    readme = (Path(__file__).parent.parent / "README.md").open().read()

    readme_sections = {}
    for region in readme.split("\n###"):
        title = region.splitlines()[0].strip()
        content = "\n".join(region.splitlines()[1:])
        lines_to_keep = [
            line
            for line in content.splitlines()
            if line.strip() not in ("```python", "```")
        ]
        words = "\n".join(lines_to_keep).split()

        readme_sections[title] = words

    for cls in [Id, ConvexSet, ConvexMap]:
        docstring = cls.__doc__
        name = cls.__name__
        assert docstring
        words = docstring.split()

        assert words == readme_sections[name]
