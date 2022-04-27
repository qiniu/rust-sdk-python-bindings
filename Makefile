.PHONY: build initialize console test clippy clean

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
	.env/bin/python -m pip install .; \
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
clean:
	cargo clean
	rm -rf .env
