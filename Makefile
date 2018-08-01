
release:
	RUSTC_WRAPPER=`which sccache` cargo build --release --features poppler_lock

release-without-lock:
	RUSTC_WRAPPER=`which sccache` cargo build --release

build-debug:
	CARGO_INCREMENTAL=1 RUSTC_WRAPPER=`which sccache` cargo build

test:
	RUSTC_WRAPPER=`which sccache` cargo test

install-sccache:
	cargo install --force --git https://github.com/mozilla/sccache

rlwrap-completions:
	ruby -e 'ARGF.readlines.uniq.sort.each{|it| puts it}' ~/.config/chrysoberyl/config.chry > ~/.chrysoberyl_completions

format:
	rustfmt --write-mode overwrite **/*.rs

rustfmt-test:
	git cancel
	rustfmt --write-mode overwrite **/*.rs
	
inspector:
	GTK_DEBUG=interactive ./target/debug/chrysoberyl
