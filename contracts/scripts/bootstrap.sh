#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
install_dependency() {
  local directory="$1" repository="$2" revision="$3"
  if [[ ! -d "$directory/.git" ]]; then
    if [[ -e "$directory" ]]; then echo "Refusing to overwrite dependency directory: $directory" >&2; exit 1; fi
    git clone --quiet "$repository" "$directory"
    git -C "$directory" checkout --quiet --detach "$revision"
  fi
  if [[ "$(git -C "$directory" rev-parse HEAD)" != "$revision" ]]; then
    echo "Dependency revision mismatch: $directory" >&2; exit 1
  fi
  if [[ -n "$(git -C "$directory" status --porcelain --untracked-files=no)" ]]; then
    echo "Dependency has modified tracked files: $directory" >&2; exit 1
  fi
}
mkdir -p lib
install_dependency lib/openzeppelin-contracts https://github.com/OpenZeppelin/openzeppelin-contracts.git c64a1edb67b6e3f4a15cca8909c9482ad33a02b0
install_dependency lib/forge-std https://github.com/foundry-rs/forge-std.git 77041d2ce690e692d6e03cc812b57d1ddaa4d505
