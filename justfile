set unstable
set lists

target := "tegra20-unknown-linux-gnueabihf.json"
flags := "-Zbuild-std -Zjson-target-spec --target=" + target

# Packages holding the on-device hardware tests.
hw_pkgs := ["hw-tests"]

# Cross-compile for the device.
[arg("release", long, value="--release", help="build with the release profile")]
build release="":
    cargo build {{ flags }} {{ release }}

# Build a binary and run it on the device.
[arg("bin", long, help="binary to run")]
[arg("release", long, value="--release", help="build with the release profile")]
run release="" bin="":
    cargo run {{ flags }} {{ release }} {{ if bin == "" { "" } else { "--bin " + bin } }}

# Run all tests: the hardware-independent ones natively on this machine, then
# the hardware tests on the device.
[arg("release", long, value="--release", help="build with the release profile")]
test release="":
    cargo test --workspace {{ "--exclude=" + hw_pkgs }} {{ release }}
    cargo test {{ flags }} {{ "--package=" + hw_pkgs }} {{ release }}
