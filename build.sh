#!/bin/sh

cd ../flowrs && cargo build &&

cd ../flowrs-std && cargo build &&

cd ../flowrs-build && cargo build &&

cd flow-projects/$1 && cargo build &&