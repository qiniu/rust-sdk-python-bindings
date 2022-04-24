.PHONY: rebuild initialize clean

all: rebuild
rebuild:
	rm -rf .env
	$(MAKE) initialize
initialize: SHELL:=/bin/bash
initialize:
	set -e ; \
	${PYO3_PYTHON} -m venv .env; \
	source .env/bin/activate; \
	${PYO3_PIP} install maturin; \
	maturin develop
clean:
	cargo clean
	rm -rf .env
