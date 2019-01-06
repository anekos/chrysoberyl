
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
	GTK_DEBUG=interactive ./target/debug/chrysoberyl test-files/

benchmark-cherenkov:
	cargo run --release \
		@@set fit-to original \
		@@set fit-to 5000% \
		@@push-image test-files/cell.png \
		@@shell timeit start cherenkov \
		@@cherenkov -x 0.2 -y 0.2 --color '#00FF00' --radius 0.1 --spokes 100 --random-hue 0 --seed cat \
		@@cherenkov -x 0.4 -y 0.2 --color '#FF0000' --radius 0.1 --spokes 100 --random-hue 0 --seed cat \
		@@cherenkov -x 0.2 -y 0.4 --color '#0000FF' --radius 0.1 --spokes 100 --random-hue 0 --seed cat \
		@@cherenkov -x 0.4 -y 0.4 --color '#FFFF00' --radius 0.1 --spokes 100 --random-hue 0 --seed cat \
		@@queue \; @shell --sync timeit lap cherenkov \; @quit
	timeit show cherenkov
