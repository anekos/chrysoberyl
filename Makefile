
release:
	cargo build --release

test:
	cargo test

rlwrap-completions:
	ruby -e 'ARGF.readlines.uniq.sort.each{|it| puts it}' ~/.config/chrysoberyl/config.chry > ~/.chrysoberyl_completions

format:
	rustfmt --write-mode overwrite **/*.rs

rustfmt-test:
	git cancel
	rustfmt --write-mode overwrite **/*.rs
