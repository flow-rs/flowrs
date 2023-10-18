#!/bin/sh

#expects to be run from the flowrs project directory within a monorepo setup for the flow-rs project
cp service_config.json ../flowrs-build/target/debug/service_config.json &&

cd ../flowrs-build/target/debug && ./service_main --config-file service_config.json
