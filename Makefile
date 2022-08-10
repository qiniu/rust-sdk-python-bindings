.PHONY: build initialize apis console test clippy docs clean

all: build
build:
ifeq ($(OS),Windows_NT)
	powershell "(.\.env\Scripts\Activate.ps1); (.\.env\Scripts\python.exe -m pip uninstall -y qiniu-sdk-alpha); (maturin.exe develop)"
else
	set -e; \
	. .env/bin/activate; \
	.env/bin/python -m pip uninstall -y qiniu-sdk-alpha || true; \
	maturin develop
endif
apis:
	cargo run --example api-generator
	cargo fmt
	$(MAKE) clippy
initialize:
ifeq ($(OS),Windows_NT)
	powershell "(${PYO3_PYTHON} -m venv .env); (.\.env\Scripts\Activate.ps1); (.\.env\Scripts\python.exe -m pip install pip --upgrade); (.\.env\Scripts\python.exe -m pip install .[tests]); (maturin.exe develop)"
else
	set -e; \
	${PYO3_PYTHON} -m venv .env; \
	. .env/bin/activate; \
	.env/bin/python -m pip install pip --upgrade; \
	.env/bin/python -m pip install ".[tests]"; \
	maturin develop
endif
test:
ifeq ($(OS),Windows_NT)
	powershell "(.\.env\Scripts\Activate.ps1); (cd tests); (..\.env\Scripts\python.exe -m unittest -v)"
else
	set -e; \
	. .env/bin/activate; \
	cd tests; \
	../.env/bin/python -m unittest -v
endif
console:
ifeq ($(OS),Windows_NT)
	powershell "(.\.env\Scripts\Activate.ps1); (.\.env\Scripts\python.exe)"
else
	set -e; \
	. .env/bin/activate; \
	.env/bin/python
endif
clippy:
	cargo clippy --examples --tests
docs: build
	$(MAKE) -C docs html
clean:
	make -C rust-sdk clean
	cargo clean
ifeq ($(OS),Windows_NT)
	powershell "(Get-ChildItem .env -Recurse | Remove-Item -Force -Recurse)"
	powershell "(Get-ChildItem docs\_build -Recurse | Remove-Item -Force -Recurse)"
else
	rm -rf .env docs/_build
endif
