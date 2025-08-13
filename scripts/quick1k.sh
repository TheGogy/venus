#!/bin/bash
BOOK="Books/Pohl.epd"
e1="$1"
e2="$2"

nice cutechess-cli \
    -engine cmd="$e1" name="$e1" \
    -engine cmd="$e2" name="$e2" \
    -each timemargin=400 tc=100/8+0.8 proto=uci \
    -openings file="$BOOK" order=random format=epd \
    -rounds 500 \
    -games 2 \
    -concurrency 24 \
    -repeat
