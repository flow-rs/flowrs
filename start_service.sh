#!/bin/sh


cp service_config.json ../flowrs-build/target/debug/service_config.json &&

cd ../flowrs-build/target/debug && ./service_main --config-file service_config.json
