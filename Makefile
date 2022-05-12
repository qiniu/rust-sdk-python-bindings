.PHONY: build initialize console test clippy docs clean

all: build
build:
	set -e; \
	. .env/bin/activate; \
	.env/bin/python -m pip uninstall -y qiniu-sdk-bindings || true; \
	maturin develop
initialize:
	set -e; \
	${PYO3_PYTHON} -m venv .env; \
	. .env/bin/activate; \
	.env/bin/python -m pip install pip --upgrade; \
	.env/bin/python -m pip install ".[tests]"; \
	maturin develop
test:
	set -e; \
	. .env/bin/activate; \
	cd tests; \
	../.env/bin/python -m unittest -v
console:
	set -e; \
	. .env/bin/activate; \
	.env/bin/python
clippy:
	cargo clippy
docs: build
	$(MAKE) -C docs html
clean:
	make -C rust-sdk clean
	cargo clean
	rm -rf .env docs/_build
