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
	@which node > /dev/null || (echo "✗ node not found. Install from https://nodejs.org" && exit 1)
	@which cargo > /dev/null || (echo "✗ cargo not found. Install Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh" && exit 1)
	@echo "✓ All dependencies found"

install-node-deps:
	@echo "Installing Node dependencies for CASTL..."
	@cd castl-js && npm install --silent && cd ..
	@echo "✓ Node dependencies installed"

build: check-deps
	@echo "Building fredc..."
	@cd fredc && cargo build --release
	@echo "✓ fredc built"

install: build install-node-deps
	@mkdir -p $(INSTALL_PREFIX) $(HOME)/.local/share/fredify
	@cp $(FREDC_BINARY) $(INSTALL_PREFIX)/fredc-bin.tmp && mv -f $(INSTALL_PREFIX)/fredc-bin.tmp $(INSTALL_PREFIX)/fredc-bin
	@cp -r castl-js $(HOME)/.local/share/fredify/
	@echo '#!/bin/bash' > /tmp/fred_wrapper.sh
	@echo 'FREDC="$$(dirname "$$0")/fredc-bin"' >> /tmp/fred_wrapper.sh
	@echo 'CASTL="$(HOME)/.local/share/fredify/castl-js"' >> /tmp/fred_wrapper.sh
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
	@echo '  *.js) TMP="/tmp/$$(basename "$$FILE" .js).lua"; OUTPUT="$${2:-./$$(basename "$$FILE" .js)}"; node "$$CASTL/castl.js" "$$FILE" > "$$TMP" 2>/dev/null || exit 1; if [[ "$${ARGS[@]}" == *"--to-lua"* ]]; then cp "$$TMP" "$$OUTPUT.lua"; echo "✓ Generated: $$OUTPUT.lua"; else exec "$$FREDC" "$${ARGS[@]}" "$$TMP" "$$OUTPUT"; fi;;' >> /tmp/fred_wrapper.sh
	@echo '  *.lua|*.fred) OUTPUT="$${2:-./$$(basename "$$FILE" | sed '\''s/\.[^.]*$$//'\'')}" ; exec "$$FREDC" "$${ARGS[@]}" "$$FILE" "$$OUTPUT";;' >> /tmp/fred_wrapper.sh
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

test: build install-node-deps
	@echo "Testing examples..."
	@for f in examples/0[1-3]_*.fred; do \
		echo "Testing $$f..."; \
		$(FREDC_BINARY) "$$f" > /dev/null || exit 1; \
	done
	@echo "✓ Examples pass"
