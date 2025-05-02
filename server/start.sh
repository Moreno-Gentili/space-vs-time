#!/bin/bash
spacetime start --in-memory --listen-addr 0.0.0.0:3000 &
pid=$!
echo "spacetime process started with PID $pid"
until curl --fail-with-body --no-progress-meter http://localhost:3000/v1/ping; do sleep 5; done
echo "SpaceTimeDB is now available"
/app/server/publish-module.sh

wait $pid
echo "Process completed"