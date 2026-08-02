EXE := justbot

ifeq ($(OS),Windows_NT)
	NAME := $(EXE).exe
else
	NAME := $(EXE)
endif

RUSTFLAGS ?= -C target-cpu=native
export RUSTFLAGS

.PHONY: build pgo check-all

build:
	cargo rustc --release --bin justbot -- --emit link=$(NAME)

pgo:
	cargo pgo instrument
	cargo pgo run -- bench
	cargo pgo optimize

check-all:
	RUSTFLAGS="-C target-cpu=x86-64" cargo check
	RUSTFLAGS="-C target-cpu=x86-64-v2" cargo check
	RUSTFLAGS="-C target-cpu=x86-64-v3" cargo check
	RUSTFLAGS="-C target-cpu=x86-64-v4" cargo check