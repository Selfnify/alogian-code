#!/usr/bin/env sh
set -eu
repo="Selfnify/alogian-code"
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) target="darwin-arm64" ;;
  Darwin-x86_64) target="darwin-x64" ;;
  *) echo "Alogian Code supports macOS (Apple Silicon or Intel) and Windows x64." >&2; exit 1 ;;
esac
dir="$HOME/.local/bin"
mkdir -p "$dir"
base="https://github.com/$repo/releases/latest/download"
curl -fsSL "$base/alogian-$target" -o "$dir/alogian"
curl -fsSL "$base/ast-grep-$target" -o "$dir/ast-grep"
chmod +x "$dir/alogian" "$dir/ast-grep"
echo "Installed Alogian Code to $dir"
case ":$PATH:" in
  *":$dir:"*) echo "Run: alogian" ;;
  *) echo "Add $dir to your PATH, then run: alogian" ;;
esac
