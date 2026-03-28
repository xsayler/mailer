#!/bin/bash
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
TOPDIR="$PROJECT_DIR/rpmbuild"
SPEC="$PROJECT_DIR/pkg/rpm/mailer.spec"

echo "==> Building release binary..."
cargo build --release --manifest-path "$PROJECT_DIR/Cargo.toml"

echo "==> Preparing rpmbuild tree..."
rm -rf "$TOPDIR"
mkdir -p "$TOPDIR"/{BUILD,RPMS,SRPMS,SPECS,SOURCES}

echo "==> Building RPM..."
rpmbuild --define "_topdir $TOPDIR" --define "_project_dir $PROJECT_DIR" -bb "$SPEC"

RPM=$(find "$TOPDIR/RPMS" -name '*.rpm' | head -1)
echo ""
echo "==> RPM ready: $RPM"
echo "    Install with: sudo dnf install $RPM"
