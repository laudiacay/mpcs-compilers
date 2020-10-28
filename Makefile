build:
ifeq (, $(shell which cargo))
	$(error "No cargo in $(PATH), please install using script at https://rustup.rs/")
else
	cargo build --release
	mkdir -p ./bin
	cp ./target/release/ekcc ./bin/ekcc
endif

clean: 
	cargo clean 
	rm ./bin/*
