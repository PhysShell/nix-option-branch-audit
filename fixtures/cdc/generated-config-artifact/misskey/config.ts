/*
 * Real, bounded excerpt of misskey's own config schema -- fetched from
 * misskey-dev/misskey @ tag 2026.6.0, packages/backend/src/config.ts,
 * lines 1-118 verbatim (imports + both real type declarations this
 * project's D-extraction needs: `RedisOptionsSource` (lines 14-21,
 * referenced by `Source.redis`/`redisForPubsub`/etc.) and `Source`
 * itself (lines 26-118), the real top-level accepted config shape).
 */
/*
 * SPDX-FileCopyrightText: syuilo and misskey-project
 * SPDX-License-Identifier: AGPL-3.0-only
 */

import * as fs from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { type FastifyServerOptions } from 'fastify';
import type * as Sentry from '@sentry/node';
import type * as SentryVue from '@sentry/vue';
import type { RedisOptions } from 'ioredis';

type RedisOptionsSource = Partial<RedisOptions> & {
	host: string;
	port: number;
	family?: number;
	pass: string;
	db?: number;
	prefix?: string;
};

/**
 * 設定ファイルの型
 */
type Source = {
	url?: string;
	port?: number;
	socket?: string;
	trustProxy?: FastifyServerOptions['trustProxy'];
	chmodSocket?: string;
	enableIpRateLimit?: boolean;
	disableHsts?: boolean;
	db: {
		host: string;
		port: number;
		db?: string;
		user?: string;
		pass?: string;
		disableCache?: boolean;
		extra?: { [x: string]: string };
	};
	dbReplications?: boolean;
	dbSlaves?: {
		host: string;
		port: number;
		db: string;
		user: string;
		pass: string;
	}[];
	redis: RedisOptionsSource;
	redisForPubsub?: RedisOptionsSource;
	redisForJobQueue?: RedisOptionsSource;
	redisForTimelines?: RedisOptionsSource;
	redisForReactions?: RedisOptionsSource;
	fulltextSearch?: {
		provider?: FulltextSearchProvider;
	};
	meilisearch?: {
		host: string;
		port: string;
		apiKey: string;
		ssl?: boolean;
		index: string;
		scope?: 'local' | 'global' | string[];
	};
	sentryForBackend?: { options: Partial<Sentry.NodeOptions>; enableNodeProfiling: boolean; };
	sentryForFrontend?: {
		options: Partial<SentryVue.BrowserOptions> & { dsn: string };
		vueIntegration?: SentryVue.VueIntegrationOptions | null;
		browserTracingIntegration?: Parameters<typeof SentryVue.browserTracingIntegration>[0] | null;
		replayIntegration?: Parameters<typeof SentryVue.replayIntegration>[0] | null;
	};

	publishTarballInsteadOfProvideRepositoryUrl?: boolean;

	setupPassword?: string;

	proxy?: string;
	proxySmtp?: string;
	proxyBypassHosts?: string[];

	allowedPrivateNetworks?: string[];

	maxFileSize?: number;

	clusterLimit?: number;
	threadPoolSize?: number;

	id: string;

	outgoingAddress?: string;
	outgoingAddressFamily?: 'ipv4' | 'ipv6' | 'dual';

	deliverJobConcurrency?: number;
	inboxJobConcurrency?: number;
	relationshipJobConcurrency?: number;
	deliverJobPerSec?: number;
	inboxJobPerSec?: number;
	relationshipJobPerSec?: number;
	deliverJobMaxAttempts?: number;
	inboxJobMaxAttempts?: number;

	mediaProxy?: string;
	videoThumbnailGenerator?: string;

	perChannelMaxNoteCacheCount?: number;
	perUserNotificationsMaxCount?: number;
	deactivateAntennaThreshold?: number;
	pidFile: string;

	logging?: {
		sql?: {
			disableQueryTruncation?: boolean,
			enableQueryParamLogging?: boolean,
		}
	}
};
