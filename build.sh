#!/bin/bash

for src in ./src/as/*; do
  (
    cd "$src"

    if grep -Eq '"deprecated"[[:space:]]*:[[:space:]]*true' ./res/source.json; then
      rm -rf ./build/package.aix
    else
      npm run build
    fi
  )
done

for src in ./src/rust/*; do
  (
    cd "$src"

    if grep -Eq '"deprecated"[[:space:]]*:[[:space:]]*true' ./res/source.json; then
      rm -rf ./package.aix
    else
      ./build.sh -a
    fi
  )
done

aidoku build ./src/**/*.aix --name "Aidoku 中文图源"
