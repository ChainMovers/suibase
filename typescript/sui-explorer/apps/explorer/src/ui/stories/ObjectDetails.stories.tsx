// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { SuiClientProvider } from '@mysten/dapp-kit';
import { type Meta, type StoryObj } from '@storybook/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';

import { ObjectDetails, type ObjectDetailsProps } from '../ObjectDetails';

export default {
	component: ObjectDetails,
	decorators: [
		(Story) => (
			<MemoryRouter>
				<QueryClientProvider client={new QueryClient()}>
					<SuiClientProvider>
						<Story />
					</SuiClientProvider>
				</QueryClientProvider>
			</MemoryRouter>
		),
	],
} as Meta;

export const Default: StoryObj<ObjectDetailsProps> = {
	args: {
		name: 'Rare Apepé 4042',
		type: 'JPEG Image',
		variant: 'small',
		id: '0x4897c931565428a2a3842afb523ca5559d4b6726',
		image: 'https://ipfs.io/ipfs/bafkreibngqhl3gaa7daob4i2vccziay2jjlp435cf66vhono7nrvww53ty',
	},
};

export const Large: StoryObj<ObjectDetailsProps> = {
	args: {
		name: 'Rare Apepé 4042',
		type: 'JPEG Image',
		variant: 'large',
		id: '0x4897c931565428a2a3842afb523ca5559d4b6726',
		image: 'https://ipfs.io/ipfs/bafkreibngqhl3gaa7daob4i2vccziay2jjlp435cf66vhono7nrvww53ty',
	},
};

export const Video: StoryObj<ObjectDetailsProps> = {
	args: {
		name: 'Rare Apepé 4042',
		type: 'JPEG Image',
		variant: 'small',
		id: '0x4897c931565428a2a3842afb523ca5559d4b6726',
		image: 'https://ipfs.io/ipfs/bafkreibngqhl3gaa7daob4i2vccziay2jjlp435cf66vhono7nrvww53ty',
		video: 'https://interactive-examples.mdn.mozilla.net/media/cc0-videos/flower.webm',
	},
};
