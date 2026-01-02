#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "Usage: $0 <base-yaml> <overlay-yaml>" >&2
  exit 1
fi

BASE=$1
OVERLAY=$2

if ! command -v yq >/dev/null 2>&1; then
  echo "yqがインストールされていません。インストールします..." >&2
  if command -v brew >/dev/null 2>&1; then
    brew install yq
  else
    echo "Homebrewが見つかりません。yqを手動でインストールしてください: https://mikefarah.gitbook.io/yq/" >&2
    exit 1
  fi
fi

# cloud-initが認識するためのヘッダーを出力
echo "#cloud-config"

# 配列フィールド（write_files, runcmd, packages）を結合しつつマージ
# yqの出力から#cloud-configコメント行を除去（重複防止）
yq eval-all '
  select(fileIndex == 0) as $base |
  select(fileIndex == 1) as $overlay |
  $base * $overlay |
  .write_files = (($base.write_files // []) + ($overlay.write_files // []) | unique_by(.path)) |
  .runcmd = (($base.runcmd // []) + ($overlay.runcmd // [])) |
  .packages = (($base.packages // []) + ($overlay.packages // []) | unique)
' "$BASE" "$OVERLAY" | grep -v '^#cloud-config$'
