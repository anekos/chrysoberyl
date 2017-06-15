
release:
	cargo build --release

release-without-lock:
	cargo build --release --features poppler_lock

test:
	cargo test

rlwrap-completions:
	ruby -e 'ARGF.readlines.uniq.sort.each{|it| puts it}' ~/.config/chrysoberyl/config.chry > ~/.chrysoberyl_completions

format:
	rustfmt --write-mode overwrite **/*.rs

rustfmt-test:
	git cancel
	rustfmt --write-mode overwrite **/*.rs
