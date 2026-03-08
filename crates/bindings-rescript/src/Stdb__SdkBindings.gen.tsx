/* TypeScript file generated from Stdb__SdkBindings.res by genType. */

/* eslint-disable */
/* tslint:disable */

import * as Stdb__SdkBindingsJS from './Stdb__SdkBindings.res.mjs';

import type {ConnectionId as $$connectionId} from 'spacetimedb';

import type {DbConnectionBuilder as $$dbConnectionBuilder} from 'spacetimedb';

import type {DbConnectionImpl as $$dbConnectionImpl} from 'spacetimedb';

import type {Identity as $$identity} from 'spacetimedb';

import type {SubscriptionBuilderImpl as $$subscriptionBuilder} from 'spacetimedb';

import type {SubscriptionHandleImpl as $$subscriptionHandle} from 'spacetimedb';

import type {TimeDuration as $$timeDuration} from 'spacetimedb';

import type {Timestamp as $$timestamp} from 'spacetimedb';

import type {Uuid as $$uuid} from 'spacetimedb';

export type identity = $$identity;

export type connectionId = $$connectionId;

export type timestamp = $$timestamp;

export type timeDuration = $$timeDuration;

export type uuid = $$uuid;

export type scheduleAt = 
    { tag: "Interval"; readonly value: timeDuration }
  | { tag: "Time"; readonly value: timestamp };

export type dbConnectionBuilder<conn> = $$dbConnectionBuilder<conn>;

export type dbConnectionImpl<rm> = $$dbConnectionImpl<rm>;

export type subscriptionBuilder<rm> = $$subscriptionBuilder<rm>;

export type subscriptionHandle<rm> = $$subscriptionHandle<rm>;

export type React_props<connectionBuilder,children> = { readonly connectionBuilder: connectionBuilder; readonly children: children };

export abstract class React_connectionState { protected opaque!: any }; /* simulate opaque types */

export const Timestamp_toFloatMs: (ts:timestamp) => number = Stdb__SdkBindingsJS.Timestamp.toFloatMs as any;

export const Timestamp: { toFloatMs: (ts:timestamp) => number } = Stdb__SdkBindingsJS.Timestamp as any;
