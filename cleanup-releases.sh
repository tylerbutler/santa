#!/bin/bash
set -e

echo "🧹 Cleaning up broken releases and tags..."
echo ""

# Delete the broken GitHub releases
echo "📦 Deleting GitHub releases..."
gh release delete santa-v0.1.2 -y 2>/dev/null || echo "  ⚠️  santa-v0.1.2 release not found (might already be deleted)"
gh release delete santa-data-v0.1.2 -y 2>/dev/null || echo "  ⚠️  santa-data-v0.1.2 release not found (might already be deleted)"
echo ""

# Get the commit SHA for the tags before deleting them
echo "🔍 Getting commit SHAs for tags..."
SANTA_COMMIT=$(git rev-parse santa-v0.1.2 2>/dev/null || echo "")
SANTA_DATA_COMMIT=$(git rev-parse santa-data-v0.1.2 2>/dev/null || echo "")

if [ -z "$SANTA_COMMIT" ] || [ -z "$SANTA_DATA_COMMIT" ]; then
    echo "  ⚠️  One or both tags not found locally"
    exit 1
fi

echo "  santa-v0.1.2 -> $SANTA_COMMIT"
echo "  santa-data-v0.1.2 -> $SANTA_DATA_COMMIT"
echo ""

# Delete local tags
echo "🏷️  Deleting local tags..."
git tag -d santa-v0.1.2 2>/dev/null || echo "  ⚠️  santa-v0.1.2 tag not found locally"
git tag -d santa-data-v0.1.2 2>/dev/null || echo "  ⚠️  santa-data-v0.1.2 tag not found locally"
echo ""

# Delete remote tags
echo "🌐 Deleting remote tags..."
git push origin :refs/tags/santa-v0.1.2 2>/dev/null || echo "  ⚠️  santa-v0.1.2 tag not found on remote"
git push origin :refs/tags/santa-data-v0.1.2 2>/dev/null || echo "  ⚠️  santa-data-v0.1.2 tag not found on remote"
echo ""

# Recreate tags
echo "✨ Recreating tags..."
git tag santa-v0.1.2 $SANTA_COMMIT
git tag santa-data-v0.1.2 $SANTA_DATA_COMMIT
echo ""

# Push tags
echo "⬆️  Pushing tags to remote..."
git push origin santa-v0.1.2
git push origin santa-data-v0.1.2
echo ""

echo "✅ Done! The tags have been recreated and pushed."
echo "   This will trigger the cargo-dist Release workflow to create proper releases with assets."
