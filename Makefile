lint:
	cargo fmt --check
	cargo clippy --all-features -- -Dwarnings

test:
	cargo test
	cargo test --all-features