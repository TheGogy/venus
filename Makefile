EXE ?= Venus
DIR := $(realpath $(dir $(abspath $(lastword $(MAKEFILE_LIST)))))/build

ifeq ($(OS),Windows_NT)
	NAME := $(EXE).exe
else
	NAME := $(EXE)
endif

FEATURES_ARG :=
ifneq ($(strip $(features)),)
FEATURES_ARG := --features $(features)
endif

rule:
	cargo clean
	cargo rustc --release --package cli --bins $(FEATURES_ARG) -- -C target-cpu=native --emit link=$(NAME)

dir:
	mkdir -p $(DIR)

clean:
	rm -fr $(DIR)
	rm -f *.pdb

release: dir
	cargo rustc --release --package cli --bins $(FEATURES_ARG) -- -C target-cpu=native -C profile-generate=$(DIR) --emit link=$(NAME)
	./$(NAME) bench
	llvm-profdata merge -o $(DIR)/merged.profdata $(DIR)
	cargo rustc --release --package cli --bins $(FEATURES_ARG) -- -C target-feature=+crt-static -C target-cpu=native -C profile-use=$(DIR)/merged.profdata --emit link=$(NAME)

