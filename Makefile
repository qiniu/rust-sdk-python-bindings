.PHONY: build initialize console test clippy clean

all: build
build:
	set -e; \
	. .env/bin/activate; \
	.env/bin/python -m pip uninstall -y qiniu-sdk-python-bindings || true; \
	maturin develop
initialize:
	set -e; \
	${PYO3_PYTHON} -m venv .env; \
	. .env/bin/activate; \
	.env/bin/python -m pip install -r requirement.txt; \
	maturin develop
test:
	set -e; \
	. .env/bin/activate; \
	cd tests; \
	../.env/bin/python -m unittest -v
console: build
	set -e; \
	. .env/bin/activate; \
	.env/bin/python
clippy:
	cargo clippy
clean:
	cargo clean
	rm -rf .env
