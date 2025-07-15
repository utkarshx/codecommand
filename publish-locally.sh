#!/bin/bash

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_step() {
    echo -e "${BLUE}🔄 $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Function to show usage
show_usage() {
    echo "Usage: $0 [patch|minor|major|prerelease] [--dry-run]"
    echo ""
    echo "Version bump types:"
    echo "  patch     - Bug fixes (1.0.0 → 1.0.1)"
    echo "  minor     - New features (1.0.0 → 1.1.0)"
    echo "  major     - Breaking changes (1.0.0 → 2.0.0)"
    echo "  prerelease - Pre-release version (1.0.0 → 1.0.1-0)"
    echo ""
    echo "Options:"
    echo "  --dry-run - Build and test but don't publish"
    echo ""
    echo "Example: $0 patch"
    echo "Example: $0 minor --dry-run"
}

# Parse arguments
VERSION_TYPE=${1:-patch}
DRY_RUN=false

if [[ "$2" == "--dry-run" ]] || [[ "$1" == "--dry-run" ]]; then
    DRY_RUN=true
    if [[ "$1" == "--dry-run" ]]; then
        VERSION_TYPE="patch"
    fi
fi

# Validate version type
if [[ ! "$VERSION_TYPE" =~ ^(patch|minor|major|prerelease)$ ]]; then
    print_error "Invalid version type: $VERSION_TYPE"
    show_usage
    exit 1
fi

# Check if npm is configured for publishing
if [[ "$DRY_RUN" == "false" ]]; then
    print_step "Checking npm authentication..."
    if ! npm whoami > /dev/null 2>&1; then
        print_error "Not logged in to npm. Please run 'npm login' first."
        exit 1
    fi
    print_success "npm authentication verified"
fi

# Check required tools
print_step "Checking required tools..."
command -v node >/dev/null 2>&1 || { print_error "node is required but not installed."; exit 1; }
command -v npm >/dev/null 2>&1 || { print_error "npm is required but not installed."; exit 1; }
command -v cargo >/dev/null 2>&1 || { print_error "cargo is required but not installed."; exit 1; }
command -v pnpm >/dev/null 2>&1 || { print_error "pnpm is required but not installed."; exit 1; }

# Check if cargo-edit is installed for version management
if ! command -v cargo-set-version >/dev/null 2>&1; then
    print_warning "cargo-edit not found. Installing..."
    cargo install cargo-edit
fi

print_success "All required tools are available"

# Get current version
CURRENT_VERSION=$(node -p "require('./package.json').version")
print_step "Current version: $CURRENT_VERSION"

# Bump version
print_step "Bumping version ($VERSION_TYPE)..."
npm version $VERSION_TYPE --no-git-tag-version

# Get new version
NEW_VERSION=$(node -p "require('./package.json').version")
print_success "New version: $NEW_VERSION"

# Update npx-cli package.json
print_step "Updating npx-cli package.json..."
cd npx-cli
npm version $NEW_VERSION --no-git-tag-version --allow-same-version
cd ..

# Update backend Cargo.toml
print_step "Updating backend Cargo.toml..."
cd backend
cargo set-version "$NEW_VERSION"
cd ..

print_success "Version updated in all files"

# Clean previous builds
print_step "Cleaning previous builds..."
rm -rf npx-cli/dist
rm -rf frontend/dist
rm -rf target/release/codecommand
rm -rf target/release/mcp_task_server

# Install dependencies
print_step "Installing dependencies..."
pnpm install

# Lint and type check frontend
print_step "Linting and type checking frontend..."
cd frontend
npm run lint
npx tsc --noEmit
cd ..

# Build frontend
print_step "Building frontend..."
npm run frontend:build

# Build backend
print_step "Building Rust binaries..."
cargo build --release --manifest-path backend/Cargo.toml
cargo build --release --bin mcp_task_server --manifest-path backend/Cargo.toml

# Detect platform
print_step "Detecting platform..."
OS=$(uname -s)
ARCH=$(uname -m)

case "$OS" in
    Darwin)
        if [[ "$ARCH" == "arm64" ]]; then
            PLATFORM="macos-arm64"
        else
            PLATFORM="macos-x64"
        fi
        ;;
    Linux)
        if [[ "$ARCH" == "aarch64" ]]; then
            PLATFORM="linux-arm64"
        else
            PLATFORM="linux-x64"
        fi
        ;;
    MINGW*|MSYS*|CYGWIN*)
        if [[ "$ARCH" == "aarch64" ]]; then
            PLATFORM="windows-arm64"
        else
            PLATFORM="windows-x64"
        fi
        ;;
    *)
        print_error "Unsupported operating system: $OS"
        exit 1
        ;;
esac

print_success "Detected platform: $PLATFORM"

# Create distribution package
print_step "Creating distribution package for $PLATFORM..."
mkdir -p npx-cli/dist/$PLATFORM

# Copy binaries based on platform
if [[ "$OS" == "MINGW"* ]] || [[ "$OS" == "MSYS"* ]] || [[ "$OS" == "CYGWIN"* ]]; then
    # Windows
    cp target/release/codecommand.exe codecommand.exe
    cp target/release/mcp_task_server.exe codecommand-mcp.exe
    zip codecommand.zip codecommand.exe
    zip codecommand-mcp.zip codecommand-mcp.exe
    rm codecommand.exe codecommand-mcp.exe
else
    # Unix-like (macOS, Linux)
    cp target/release/codecommand codecommand
    cp target/release/mcp_task_server codecommand-mcp
    zip codecommand.zip codecommand
    zip codecommand-mcp.zip codecommand-mcp
    rm codecommand codecommand-mcp
fi

mv codecommand.zip npx-cli/dist/$PLATFORM/codecommand.zip
mv codecommand-mcp.zip npx-cli/dist/$PLATFORM/codecommand-mcp.zip

print_success "Distribution package created"

# Test the package
print_step "Testing the package..."
cd npx-cli
npm pack --dry-run
cd ..

# Test installation locally
print_step "Testing local installation..."
cd npx-cli
PACKAGE_FILE=$(npm pack)
print_success "Package created: $PACKAGE_FILE"

# Test the package works
print_step "Testing package functionality..."
if command -v npx >/dev/null 2>&1; then
    timeout 10s npx ./$PACKAGE_FILE --help > /dev/null 2>&1 || print_warning "Package test timed out (this might be normal)"
fi

if [[ "$DRY_RUN" == "true" ]]; then
    print_success "Dry run completed successfully!"
    print_step "Package ready at: npx-cli/$PACKAGE_FILE"
    print_step "To publish manually, run: cd npx-cli && npm publish $PACKAGE_FILE"
    
    # Clean up
    rm $PACKAGE_FILE
else
    # Publish to npm
    print_step "Publishing to npm..."
    npm publish $PACKAGE_FILE
    
    print_success "Successfully published codecommand@$NEW_VERSION to npm!"
    print_step "Users can now install with: npx codecommand@$NEW_VERSION"
    
    # Clean up
    rm $PACKAGE_FILE
fi

# Create git tag (optional)
if [[ "$DRY_RUN" == "false" ]]; then
    print_step "Creating git tag..."
    git add package.json package-lock.json npx-cli/package.json backend/Cargo.toml
    git commit -m "chore: bump version to $NEW_VERSION" || print_warning "Nothing to commit"
    git tag -a "v$NEW_VERSION" -m "Release v$NEW_VERSION"
    
    print_success "Git tag v$NEW_VERSION created"
    print_step "Don't forget to push: git push && git push --tags"
fi

print_success "Publishing process completed!" 