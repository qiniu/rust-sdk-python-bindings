.PHONY: build initialize test clean

all: build
build:
	rm -rf .env
	$(MAKE) initialize
initialize:
	set -e; \
	${PYO3_PYTHON} -m venv .env; \
	. .env/bin/activate; \
	.env/bin/python -m pip install maturin; \
	maturin develop
test: build
	set -e; \
	. .env/bin/activate; \
	cd tests; \
	../.env/bin/python -m unittest
clean:
	cargo clean
	rm -rf .env
