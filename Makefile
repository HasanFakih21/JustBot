EXE := justbot

ifeq ($(OS),Windows_NT)
	NAME := $(EXE).exe
else
	NAME := $(EXE)
endif

RUSTFLAGS ?= -C target-cpu=native
export RUSTFLAGS

.PHONY: build pgo

build:
	cargo rustc --release --bin justbot -- --emit link=$(NAME)

pgo:
	cargo pgo instrument
	cargo pgo run -- bench
	cargo pgo optimize