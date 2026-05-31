#!/bin/bash
BOOK="Books/Pohl.epd"
e1="$1"
e2="$2"

nice cutechess-cli \
    -engine cmd="$e1" name="$e1" \
    -engine cmd="$e2" name="$e2" \
    -each timemargin=400 tc=8+0.08 proto=uci \
    -openings file="$BOOK" order=random format=epd \
    -rounds 10000 \
    -games 2 \
    -concurrency 20 \
    -ratinginterval 100 \
    -sprt elo0=0.0 elo1=5.0 alpha=0.05 beta=0.05 \
    -repeat \
    -tb "$HOME"/syzygy/
