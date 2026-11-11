#!/usr/bin/env bash
set -euo pipefail

if sudo apt-get update -o APT::Update::Error-Mode=any && "$@"; then
  exit 0
fi

shopt -s nullglob
sources=(/etc/apt/apt-mirrors.txt /etc/apt/sources.list /etc/apt/sources.list.d/*.list /etc/apt/sources.list.d/*.sources)
changed=false
for source in "${sources[@]}"; do
  if [[ -f "$source" ]] && grep -Eq 'https?://azure[.]archive[.]ubuntu[.]com/ubuntu([/[:space:]]|$)' "$source"; then
    sudo sed -Ei 's#(https?://)azure[.]archive[.]ubuntu[.]com(/ubuntu)([/[:space:]]|$)#\1archive.ubuntu.com\2\3#g' "$source"
    changed=true
  fi
done

if [[ "$changed" != true ]]; then
  echo 'APT setup failed and no Azure Ubuntu mirror was configured.' >&2
  exit 1
fi

echo 'Retrying APT setup with archive.ubuntu.com.'
sudo apt-get update -o APT::Update::Error-Mode=any
"$@"
