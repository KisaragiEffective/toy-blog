#!/bin/sh

die() {
  echo "$@" >&2
  exit 1
}

if git diff --name-only | grep -E '^packages/toy-blog/Cargo\.toml$' >/dev/null 2>&1; then
  root="$(git rev-parse --show-toplevel)"
  lock="$root/Cargo.lock"
  if [ -f "$lock" ]; then
    version_regex='^version *= *"[^"]+"$'
    lock_ver="$(grep -E '^\[\[package\]\]' -A 2 "$lock" | \
      grep -E '^name = "toy-blog"' -A 1 | \
      grep -E "$version_regex" 2>/dev/null || die "broken lock file")"
    toml_ver="$(grep -E "$version_regex" "$root/packages/toy-blog/Cargo.toml" 2>/dev/null  || die "broken package manifest")"
    if [ "$lock_ver" = "$toml_ver" ]; then
      exit 0
    else
      cargo update -p toy-blog
      git add Cargo.lock
    fi
  else
    die "Please generate Cargo.lock"
  fi
fi
