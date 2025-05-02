#!/bin/bash
source /usr/local/cargo/env
MODULE_NAME="space-vs-time"
spacetime server add localhost --url http://localhost:3000
spacetime delete $MODULE_NAME --yes --server localhost
spacetime publish --project-path /app/server $MODULE_NAME --yes --server localhost
ADMIN_IDENTITY=$(spacetime login show | awk '{print $NF}')
spacetime sql $MODULE_NAME "INSERT INTO admins (identity) VALUES (0x$ADMIN_IDENTITY);" --server localhost
echo "=== ADMIN TOKEN ==="
cat ~/.config/spacetime/cli.toml | grep "token"