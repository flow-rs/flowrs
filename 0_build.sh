#!/bin/sh

#expects to be run from the flowrs project directory within a monorepo setup for the flow-rs project
cd ../flowrs && cargo build &&

cd ../flowrs-std && cargo build &&

cd ../flowrs-build && cargo build &&

if [ $# -eq 1 ] ; then
  cd ../flowrs-build/flow-projects/$1 && cargo build
fi