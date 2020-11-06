build:
ifeq (, $(shell which cargo))
	$(error "No cargo in $(PATH), please install using script at https://rustup.rs/")
else
	cargo build --release --bin ekcc
	mkdir -p ./bin
	cp ./target/release/ekcc ./bin/ekcc
endif

fuzzbuild:
ifeq (, $(shell which cargo))
	$(error "No cargo in $(PATH), please install using script at https://rustup.rs/")
else
	cargo install afl
	cargo afl build --release --bin ekcc-fuzz
endif

fuzz: fuzzbuild
	cargo afl fuzz -i in -o out target/release/ekcc-fuzz

clean:
	cargo clean
	rm -rf ./bin
	rm -rf ./out
