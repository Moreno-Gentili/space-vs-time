#!/bin/bash
source /usr/local/cargo/env
source /usr/local/cargo/env

# CSharp client
spacetime generate --lang csharp --out-dir ../client/csharp/ModuleBindings --project-path . --yes

# Rust client
spacetime generate --lang rust --out-dir ../client/rust/src/module_bindings --project-path . --yes

# TypeScript client
spacetime generate --lang typescript --out-dir ../client/typescript/src/module_bindings --project-path . --yes

# Simulation client
spacetime generate --lang csharp --out-dir ../simulation/ModuleBindings --project-path . --yes

# Visualization client
spacetime generate --lang typescript --out-dir ../visualization/src/module_bindings --project-path . --yes