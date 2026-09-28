#!/bin/bash

BOOK="Books/Pohl.epd"

if [ "$#" -ne 3 ]; then
    echo "Usage: $0 {stc|mtc|ltc} ENGINE1 ENGINE2" >&2
    exit 2
fi

mode="$1"
e1="$2"
e2="$3"

case "$mode" in
stc)
    tc="8+0.08"
    hash=32
    ;;
mtc)
    tc="20.0+0.2"
    hash=64
    ;;
ltc)
    tc="60.0+0.6"
    hash=128
    ;;
*)
    echo "Usage: $0 {stc|mtc|ltc} ENGINE1 ENGINE2" >&2
    exit 2
    ;;
esac

echo "Starting $mode match (tc=$tc hash=$hash)..."

nice fastchess \
    -engine cmd="$e1" name="$e1" \
    -engine cmd="$e2" name="$e2" \
    -openings file="$BOOK" format=epd order=random \
    -each tc="$tc" option.Hash="$hash" \
    -rounds 1600 \
    -repeat \
    -concurrency 16 \
    -tb "$HOME"/syzygy/
