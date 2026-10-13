#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
: "${JAVA_HOME:?Set JAVA_HOME to a JDK 25 directory}"
: "${VELOCITY_JAR:?Set VELOCITY_JAR to the deployed Velocity 4.2.0 jar}"
cargo test --locked
cargo build --locked --release --target wasm32-wasip2
mkdir -p proxy-helper/build dist
"$JAVA_HOME/bin/javac" -proc:none -cp "$VELOCITY_JAR" -d proxy-helper/build proxy-helper/src/tablist/TablistBridge.java
cp proxy-helper/velocity-plugin.json proxy-helper/build/
"$JAVA_HOME/bin/jar" --create --file dist/rust-tab-list-bridge.jar -C proxy-helper/build .
cp target/wasm32-wasip2/release/rust_tab_list.wasm dist/
