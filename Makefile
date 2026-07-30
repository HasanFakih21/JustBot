ifeq ($(OS),Windows_NT)
	OUT := justbot.exe
else
	OUT := justbot
endif

RUSTFLAGS ?= -C target-cpu=native
export RUSTFLAGS

.PHONY: build pgo

build:
	cargo rustc --release --bin justbot -- --emit link=$(OUT)

pgo:
	cargo pgo instrument
	cargo pgo run -- bench
	cargo pgo optimize