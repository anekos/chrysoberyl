
release:
	cargo build --release

test:
	cargo test

rlwrap-completions:
	ruby -e 'ARGF.readlines.uniq.sort.each{|it| puts it}' ~/.config/chrysoberyl/rc.conf > ~/.chrysoberyl_completions
