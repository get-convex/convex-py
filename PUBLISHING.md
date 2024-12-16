# Manual deploy process

## Test publish

First set up a section of your ~/.pypirc with a token

```
[testpypi]
  username = __token__
  password = <your token here, like pypi-AaBbCcDdEd...>
```

Then increment the version number in pyproject.yaml and open that PR. Download
the wheels artifact (aprox. 100MB) from the GitHub CI workflow
python-client-build.yml and copy it into the dist directory.

```
# This is only required for distribution
rm -r dist python/_convex/_convex.*.so
poetry install
poetry run maturin build --out dist
# test publish
MATURIN_REPOSITORY=testpypi maturin upload dist/*
# Now you can download thei convex package from test pypi
python -m pip install --index-url https://test.pypi.org/simple/ convex
```

Navigate https://pypi.org/project/convex/ and double check things look good.
