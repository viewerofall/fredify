.PHONY: install build clean check-deps help uninstall test test-run gold

INSTALL_PREFIX ?= $(HOME)/.local/bin
FREDC_BINARY := fredc/target/release/fredc
FRED_WRAPPER := $(INSTALL_PREFIX)/fred

# Every example compiles (make test). A deterministic subset also gets run and
# diffed against golden output (make test-run). Excluded: random, stdin, network,
# and environment-dependent output (07 prints len($HOME), differs per machine).
ALL_EXAMPLES := $(wildcard examples/*.fred examples/*.js examples/*.lua)
SKIP_RUN := \
	examples/04_math_library.fred \
	examples/07_advanced_features.fred \
	examples/11_rock_paper_scissors.fred \
	examples/12_number_guessing_game.fred \
	examples/13_snake_game.fred \
	examples/15_weather_http.fred \
	examples/17_rock_paper_scissors.js
RUN_EXAMPLES := $(filter-out $(SKIP_RUN),$(ALL_EXAMPLES))

help:
	@echo "fredify - Lua+JS→C Compiler"
	@echo ""
	@echo "Usage:"
	@echo "  make install      Build and install fred command"
	@echo "  make build        Build fredc binary only"
	@echo "  make test         Compile every example (.fred/.js/.lua)"
	@echo "  make test-run     Run deterministic examples, diff vs golden output"
	@echo "  make gold         Regenerate golden output (after intended changes)"
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
	@rm -f /tmp/__repl_* /tmp/*.c example_output.txt
	@rm -rf /tmp/fredrun
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

# Run the deterministic examples and diff stdout against examples/expected/*.out.
test-run: build
	@echo "Running examples + diffing golden output..."
	@mkdir -p /tmp/fredrun
	@fail=0; \
	for f in $(RUN_EXAMPLES); do \
		name=$$(basename $$f); \
		printf '  %-40s ' "$$name"; \
		if ! $(FREDC_BINARY) "$$f" /tmp/fredrun/bin > /dev/null 2>&1; then \
			echo "COMPILE FAIL"; fail=1; continue; \
		fi; \
		/tmp/fredrun/bin > /tmp/fredrun/got 2>&1; \
		if [ ! -f "examples/expected/$$name.out" ]; then \
			echo "NO GOLDEN (run: make gold)"; fail=1; continue; \
		fi; \
		if diff -u "examples/expected/$$name.out" /tmp/fredrun/got > /tmp/fredrun/d 2>&1; then \
			echo "ok"; \
		else \
			echo "MISMATCH"; fail=1; sed 's/^/      /' /tmp/fredrun/d; \
		fi; \
	done; \
	rm -rf /tmp/fredrun example_output.txt; \
	if [ $$fail -ne 0 ]; then echo "✗ test-run failures"; exit 1; fi
	@echo "✓ All runnable examples match golden output"

# Regenerate golden output. Run this when output legitimately changes, then
# eyeball `git diff examples/expected/` before committing.
gold: build
	@mkdir -p examples/expected /tmp/fredrun
	@for f in $(RUN_EXAMPLES); do \
		name=$$(basename $$f); \
		if $(FREDC_BINARY) "$$f" /tmp/fredrun/bin > /dev/null 2>&1; then \
			/tmp/fredrun/bin > "examples/expected/$$name.out" 2>&1; \
			echo "  gold $$name"; \
		else \
			echo "  SKIP $$name (compile failed)"; \
		fi; \
	done; \
	rm -rf /tmp/fredrun example_output.txt
	@echo "✓ Regenerated golden outputs in examples/expected/"
