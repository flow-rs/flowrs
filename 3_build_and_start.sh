#!/bin/sh

# fails if any command fails
set -e

#expects to be run from the flowrs project directory within a monorepo setup for the flow-rs project
#example: ./2_start_flow.sh flow_project_80 libflow_project_80.so

if [ $# -eq 2 ] ; then
  cd ../flowrs-build/flow-projects/$1
  cargo build
  cd ../../../flowrs

  cd ../flowrs-build/target/debug && ./runner_main --workers 4 --flow ../../flow-projects/$1/target/debug/$2
fi
