// === Stdb__GenTypeGuard.res — TSC type safety guard ===

// === Step 1: Bottom type — no type params ===
@genType.import(("spacetimedb/dist/sdk/spacetime_module", "UntypedRemoteModule"))
type untypedRemoteModule

// === Step 2: Concrete aliases — reference Stdb__SdkBindings generics with concrete type args ===
// genType will pull in the @genType.import types from Stdb__SdkBindings transitively.

@genType
type dbConnectionImpl = Stdb__SdkBindings.dbConnectionImpl<untypedRemoteModule>

@genType
type dbConnectionBuilder = Stdb__SdkBindings.dbConnectionBuilder<dbConnectionImpl>

@genType
type subscriptionBuilder = Stdb__SdkBindings.subscriptionBuilder<untypedRemoteModule>

@genType
type subscriptionHandle = Stdb__SdkBindings.subscriptionHandle<untypedRemoteModule>
