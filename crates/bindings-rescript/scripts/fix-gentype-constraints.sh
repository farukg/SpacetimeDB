#!/usr/bin/env bash
# fix-gentype-constraints.sh — Post-process genType output to add extends constraints
#
# genType cannot generate TypeScript `extends` constraints on type parameters.
# This script patches the generated .gen.tsx files to add the correct SDK constraints.
# Run after `npx rescript build`, before `npx tsc --noEmit`.
#
# Idempotent: safe to run multiple times.

set -euo pipefail
cd "$(dirname "$0")/.."

FILE="src/Stdb__SdkBindings.gen.tsx"

if [ ! -f "$FILE" ]; then
  echo "⚠ $FILE not found — skipping constraint patching"
  exit 0
fi

# DbConnectionBuilder<conn> → <conn extends DbConnectionImpl<any>>
sd --write 'export type dbConnectionBuilder<conn>' \
   'export type dbConnectionBuilder<conn extends import("spacetimedb").DbConnectionImpl<any>>' \
   "$FILE"

# DbConnectionImpl<rm> → <rm extends UntypedRemoteModule>
sd --write 'export type dbConnectionImpl<rm>' \
   'export type dbConnectionImpl<rm extends import("spacetimedb/dist/sdk/spacetime_module").UntypedRemoteModule>' \
   "$FILE"

# SubscriptionBuilderImpl<rm> → <rm extends UntypedRemoteModule>
sd --write 'export type subscriptionBuilder<rm>' \
   'export type subscriptionBuilder<rm extends import("spacetimedb/dist/sdk/spacetime_module").UntypedRemoteModule>' \
   "$FILE"

# SubscriptionHandleImpl<rm> → <rm extends UntypedRemoteModule>
sd --write 'export type subscriptionHandle<rm>' \
   'export type subscriptionHandle<rm extends import("spacetimedb/dist/sdk/spacetime_module").UntypedRemoteModule>' \
   "$FILE"

echo "✓ genType constraints patched in $FILE"
