EXE        ?= Venus
ARCH       ?= native
FEATURES   ?=
DIR        := $(realpath $(dir $(abspath $(lastword $(MAKEFILE_LIST)))))/build

ifeq ($(OS),Windows_NT)
	NAME := $(EXE).exe
else
	NAME := $(EXE)
endif

FEATURES_ARG := $(if $(strip $(FEATURES)),--features $(FEATURES),)

RUSTFLAGS_BASE := -C target-cpu=$(ARCH)
RUSTFLAGS_PGO_GEN := $(RUSTFLAGS_BASE) -C profile-generate=$(DIR)
RUSTFLAGS_PGO_USE := $(RUSTFLAGS_BASE) -C profile-use=$(DIR)/merged.profdata -C target-feature=+crt-static
RUSTFLAGS_TUNED := -Z tune-cpu=$(ARCH)

.PHONY: rule dir clean release

rule:
	cargo clean
	cargo rustc --release --package cli --bins $(FEATURES_ARG) -- $(RUSTFLAGS_BASE) --emit link=$(NAME)

dir:
	mkdir -p $(DIR)

clean:
	rm -rf $(DIR)
	rm -f *.pdb

release: dir
	cargo rustc --release --package cli --bins $(FEATURES_ARG) -- $(RUSTFLAGS_PGO_GEN) --emit link=$(NAME)
	./$(NAME) bench
	llvm-profdata merge -o $(DIR)/merged.profdata $(DIR)
	cargo rustc --release --package cli --bins $(FEATURES_ARG) -- $(RUSTFLAGS_PGO_USE) --emit link=$(NAME)
