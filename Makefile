.DEFAULT_GOAL := build

EXE      ?= Venus
ARCH     ?= native
FEATURES ?=
BUILDDIR := $(CURDIR)/build

ifeq ($(OS),Windows_NT)
	NAME := $(EXE).exe
	NNUE := $(file < network.txt)
else
	NAME := $(EXE)
	NNUE := $(shell cat "net.txt")
endif

DEFAULT_EVALFILE := $(CURDIR)/nn-$(NNUE).bin
EVALFILE ?= $(DEFAULT_EVALFILE)
export EVALFILE

FEATURES_ARG := --features embed,$(FEATURES)

RUSTFLAGS_BASE := -C target-cpu=$(ARCH) -C target-feature=+crt-static
CARGO_BUILD := cargo rustc --release --bins $(FEATURES_ARG)

.PHONY: build clean datagen release download-net

build: $(EVALFILE)
	$(info Using EVALFILE $(EVALFILE))
	$(CARGO_BUILD) --package cli -- $(RUSTFLAGS_BASE) --emit link=$(NAME)

datagen:
	$(info Using EVALFILE $(EVALFILE))
	$(CARGO_BUILD) --package datagen -- $(RUSTFLAGS_BASE) --emit link=$(NAME)-datagen

$(BUILDDIR):
	mkdir -p $@

release: | $(BUILDDIR)
	$(CARGO_BUILD) --package cli -- $(RUSTFLAGS_BASE) -C profile-generate=$(BUILDDIR) --emit link=$(NAME)
	./$(NAME) bench
	llvm-profdata merge -o $(BUILDDIR)/merged.profdata $(BUILDDIR)
	$(CARGO_BUILD) --package cli -- $(RUSTFLAGS_BASE) -C profile-use=$(BUILDDIR)/merged.profdata --emit link=$(NAME)

clean:
	rm -rf $(BUILDDIR)
	rm -f *.pdb
	cargo clean

$(DEFAULT_EVALFILE):
	$(info "Downloading NNUE $(NNUE)")
	curl -L -o $@ https://github.com/TheGogy/venus-nets/releases/download/$(NNUE)/net.bin
