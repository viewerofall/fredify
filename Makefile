.PHONY: install build clean check-deps help uninstall

INSTALL_PREFIX ?= $(HOME)/.local/bin
FREDC_BINARY := fredc/target/release/fredc
FRED_WRAPPER := $(INSTALL_PREFIX)/fred

help:
	@echo "fredify - Lua+JS→C Compiler"
	@echo ""
	@echo "Usage:"
	@echo "  make install      Build and install fred command"
	@echo "  make build        Build fredc binary only"
	@echo "  make check-deps   Check system dependencies"
	@echo "  make clean        Remove build artifacts"
	@echo "  make uninstall    Remove installed fred command"
	@echo ""
	@echo "After install, run: fred [file.fred|file.js] or fred for REPL"

check-deps:
	@echo "Checking dependencies..."
	@which gcc > /dev/null || (echo "✗ gcc not found. Install with: apt-get install build-essential" && exit 1)
	@which cargo > /dev/null || (echo "✗ cargo not found. Install Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh" && exit 1)
	@echo "✓ All dependencies found"

build: check-deps
	@echo "Building fredc..."
	@cd fredc && cargo build --release
	@echo "✓ fredc built"

install: build
	@mkdir -p $(INSTALL_PREFIX)
	@cp $(FREDC_BINARY) $(INSTALL_PREFIX)/fredc-bin.tmp && mv -f $(INSTALL_PREFIX)/fredc-bin.tmp $(INSTALL_PREFIX)/fredc-bin
	@echo '#!/bin/bash' > /tmp/fred_wrapper.sh
	@echo 'FREDC="$$(dirname "$$0")/fredc-bin"' >> /tmp/fred_wrapper.sh
	@echo 'ARGS=()' >> /tmp/fred_wrapper.sh
	@echo 'FILE=""' >> /tmp/fred_wrapper.sh
	@echo 'while [[ $$# -gt 0 ]]; do' >> /tmp/fred_wrapper.sh
	@echo '  case "$$1" in' >> /tmp/fred_wrapper.sh
	@echo '    -h|--help|help) exec "$$FREDC" --help;;' >> /tmp/fred_wrapper.sh
	@echo '    -o|--output) ARGS+=("$$1" "$$2"); shift 2;;' >> /tmp/fred_wrapper.sh
	@echo '    --to-lua|--to-fred|--to-c) ARGS+=("$$1"); shift;;' >> /tmp/fred_wrapper.sh
	@echo '    *) FILE="$$1"; break;;' >> /tmp/fred_wrapper.sh
	@echo '  esac' >> /tmp/fred_wrapper.sh
	@echo 'done' >> /tmp/fred_wrapper.sh
	@echo '[ -z "$$FILE" ] && exec "$$FREDC"' >> /tmp/fred_wrapper.sh
	@echo 'case "$$FILE" in' >> /tmp/fred_wrapper.sh
	@echo '  *.js|*.lua|*.fred) OUTPUT="$${2:-./$$(basename "$$FILE" | sed '\''s/\.[^.]*$$//'\'')}" ; exec "$$FREDC" "$${ARGS[@]}" "$$FILE" "$$OUTPUT";;' >> /tmp/fred_wrapper.sh
	@echo '  *) exec "$$FREDC" "$${ARGS[@]}" "$$FILE";;' >> /tmp/fred_wrapper.sh
	@echo 'esac' >> /tmp/fred_wrapper.sh
	@cp /tmp/fred_wrapper.sh $(FRED_WRAPPER) && chmod +x $(FRED_WRAPPER) && rm /tmp/fred_wrapper.sh
	@echo "✓ Installed! Run: export PATH=$(INSTALL_PREFIX):$$PATH && fred"

uninstall:
	@rm -f $(FRED_WRAPPER) $(INSTALL_PREFIX)/fredc-bin
	@echo "✓ Uninstalled fred"

clean:
	@echo "Cleaning..."
	@cd fredc && cargo clean && cd ..
	@rm -f /tmp/__repl_* /tmp/*.c
	@echo "✓ Cleaned"

test: build
	@echo "Compiling every example (.fred / .js / .lua)..."
	@fail=0; \
	for f in examples/*.fred examples/*.js examples/*.lua; do \
		printf '  %-40s ' "$$f"; \
		if $(FREDC_BINARY) "$$f" /tmp/fredtest_out > /dev/null 2>&1; then \
			echo "ok"; \
		else \
			echo "FAIL"; fail=1; \
			$(FREDC_BINARY) "$$f" /tmp/fredtest_out 2>&1 | sed 's/^/      /'; \
		fi; \
	done; \
	rm -f /tmp/fredtest_out /tmp/fredtest_out.c; \
	if [ $$fail -ne 0 ]; then echo "✗ Some examples failed"; exit 1; fi
	@echo "✓ All examples compile (.fred, .js, .lua)"
