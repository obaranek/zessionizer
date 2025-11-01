# Zessionizer - Zellij Project Session Manager
# Makefile for building and installing the plugin

.PHONY: help build install clean test check uninstall all

# Default target
.DEFAULT_GOAL := help

# Configuration
PLUGIN_NAME := zessionizer
WASM_TARGET := wasm32-wasip1
PLUGIN_DIR := $(HOME)/.config/zellij/plugins
DATA_DIR := $(HOME)/.local/share/zellij/zessionizer
BUILD_DIR := target/$(WASM_TARGET)/release
WASM_FILE := $(BUILD_DIR)/$(PLUGIN_NAME).wasm

# Build flags
CARGO_FLAGS := --release --target $(WASM_TARGET)

help: ## Show this help message
	@echo "Zessionizer - Zellij Project Session Manager"
	@echo ""
	@echo "Available targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "Environment:"
	@echo "  Plugin dir:  $(PLUGIN_DIR)"
	@echo "  Data dir:    $(DATA_DIR)"

build: ## Build the plugin WASM binary
	@echo "Building $(PLUGIN_NAME) plugin..."
	@cargo build $(CARGO_FLAGS)
	@echo "✓ Build complete: $(WASM_FILE)"

install: build ## Build and install the plugin to Zellij plugins directory
	@echo "Installing plugin to $(PLUGIN_DIR)..."
	@mkdir -p $(PLUGIN_DIR)
	@mkdir -p $(DATA_DIR)
	@cp $(WASM_FILE) $(PLUGIN_DIR)/
	@echo "✓ Plugin installed successfully!"
	@echo ""
	@echo "Next steps:"
	@echo "  1. Add the plugin to your Zellij config (~/.config/zellij/config.kdl)"
	@echo "  2. See README.md for configuration examples"
	@echo "  3. Restart Zellij to load the plugin"

uninstall: ## Remove the plugin from Zellij plugins directory
	@echo "Uninstalling $(PLUGIN_NAME)..."
	@rm -f $(PLUGIN_DIR)/$(PLUGIN_NAME).wasm
	@echo "✓ Plugin uninstalled"
	@echo ""
	@echo "Note: Data directory $(DATA_DIR) was not removed."
	@echo "To remove all data, run: rm -rf $(DATA_DIR)"

clean: ## Remove build artifacts
	@echo "Cleaning build artifacts..."
	@cargo clean
	@echo "✓ Clean complete"

test: ## Run all tests
	@echo "Running tests..."
	@cargo test --all

check: ## Run cargo check to verify the code compiles
	@echo "Checking code..."
	@cargo check $(CARGO_FLAGS)

clippy: ## Run clippy linter with strict rules
	@echo "Running clippy with strict lints..."
	@cargo clippy $(CARGO_FLAGS) -- \
		-D warnings \
		-W clippy::pedantic \
		-W clippy::nursery \
		-W clippy::cargo

clippy-fix: ## Run clippy with automatic fixes
	@echo "Running clippy with automatic fixes..."
	@cargo clippy --fix $(CARGO_FLAGS) --allow-dirty --allow-staged -- \
		-D warnings \
		-W clippy::pedantic \
		-W clippy::nursery \
		-W clippy::cargo

fmt: ## Format code with rustfmt
	@echo "Formatting code..."
	@cargo fmt

fmt-check: ## Check code formatting without modifying files
	@echo "Checking code formatting..."
	@cargo fmt -- --check

all: clean build test ## Clean, build, and test

dev: fmt clippy test ## Run all development checks (format, lint, test)
