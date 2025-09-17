check:
	cargo fmt --all -- --check
	cargo clippy --all-features --tests -- -Dwarnings
	cargo test --all-features
