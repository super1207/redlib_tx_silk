set RUSTFLAGS=-C target-feature=+crt-static
cargo install cross
rustup target add i686-pc-windows-msvc
rustup target add x86_64-pc-windows-msvc
cargo build --target=i686-pc-windows-msvc --release
cargo build --target=x86_64-pc-windows-msvc --release
cross build --target x86_64-unknown-linux-gnu --release
cross build --target i686-unknown-linux-gnu --release
cross build --target aarch64-linux-android --release
cross build --target aarch64-unknown-linux-gnu --release