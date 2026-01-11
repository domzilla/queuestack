#!/bin/sh
#
# Install git hooks for qstack development
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
HOOKS_DIR="$REPO_ROOT/.git/hooks"

echo "Installing git hooks..."

# Pre-commit hook
cat > "$HOOKS_DIR/pre-commit" << 'EOF'
#!/bin/sh
#
# Pre-commit hook for qstack
# Runs the full quality gate before allowing commits
#

set -e

# Ensure cargo is in PATH (needed for GUI apps like Tower)
export PATH="$HOME/.cargo/bin:$PATH"

echo "Running pre-commit quality gate..."

# Format check
echo "  Checking formatting..."
cargo fmt --check

# Clippy lints
echo "  Running clippy..."
cargo clippy -- -D warnings

# Build
echo "  Building..."
cargo build --quiet

# Tests
echo "  Running tests..."
cargo test --quiet

echo "Quality gate passed!"
EOF

chmod +x "$HOOKS_DIR/pre-commit"

echo "Done! Pre-commit hook installed."
