/* TypeScript file generated from Stdb__GenTypeGuard.res by genType. */

/* eslint-disable */
/* tslint:disable */

import type {UntypedRemoteModule as $$untypedRemoteModule} from 'spacetimedb/dist/sdk/spacetime_module';

import type {dbConnectionBuilder as Stdb__SdkBindings_dbConnectionBuilder} from './Stdb__SdkBindings.gen';

import type {dbConnectionImpl as Stdb__SdkBindings_dbConnectionImpl} from './Stdb__SdkBindings.gen';

import type {subscriptionBuilder as Stdb__SdkBindings_subscriptionBuilder} from './Stdb__SdkBindings.gen';

import type {subscriptionHandle as Stdb__SdkBindings_subscriptionHandle} from './Stdb__SdkBindings.gen';

export type untypedRemoteModule = $$untypedRemoteModule;

export type dbConnectionImpl = Stdb__SdkBindings_dbConnectionImpl<untypedRemoteModule>;

export type dbConnectionBuilder = Stdb__SdkBindings_dbConnectionBuilder<dbConnectionImpl>;

export type subscriptionBuilder = Stdb__SdkBindings_subscriptionBuilder<untypedRemoteModule>;

export type subscriptionHandle = Stdb__SdkBindings_subscriptionHandle<untypedRemoteModule>;
