lint:
	cargo fmt --check
	cargo clippy --all-features -- -Dwarnings

test:
	cargo test
	cargo test --all-features

dist-test-pr:
	@set -euo pipefail; \
	BRANCH_NAME="dist-test-$$(date +%Y%m%d-%H%M%S)"; \
	echo "Creating new branch: $$BRANCH_NAME"; \
	git checkout -b "$$BRANCH_NAME"; \
	echo "Patching dist-workspace.toml..."; \
	sed -i.bak 's/pr-run-mode = "plan"/pr-run-mode = "upload"/' dist-workspace.toml; \
	rm -f dist-workspace.toml.bak; \
	echo "Running cargo dist generate..."; \
	dist generate; \
	if ! git diff --quiet; then \
		echo "Committing changes..."; \
		git add .; \
		git commit -m "test: cargo-dist release artifact build"; \
		echo "Pushing branch and creating PR..."; \
		git push -u origin "$$BRANCH_NAME"; \
		gh pr create \
			--title "test: cargo-dist release artifact build" \
			--body "Test cargo-dist release build process."; \
		echo "PR created successfully!"; \
	else \
		echo "No changes to commit. Cleaning up branch..."; \
		git checkout -; \
		git branch -D "$$BRANCH_NAME"; \
	fi
