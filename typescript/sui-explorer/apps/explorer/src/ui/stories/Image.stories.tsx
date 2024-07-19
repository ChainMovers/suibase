// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { SuiClientProvider } from '@mysten/dapp-kit';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';

import { Image, type ImageProps } from '../image/Image';
import { VISIBILITY } from '~/hooks/useImageMod';

import type { Meta, StoryObj } from '@storybook/react';

export default {
	component: Image,
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

export const Default: StoryObj<ImageProps> = {
	args: {
		src: 'https://images.unsplash.com/photo-1588466585717-f8041aec7875?ixlib=rb-4.0.3&ixid=MnwxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8&auto=format&fit=crop&w=1374&q=80',
		alt: 'Goat',
	},
};

export const Sized: StoryObj<ImageProps> = {
	render: () => (
		<div className="flex space-x-2">
			<Image
				size="sm"
				rounded="md"
				src="https://images.unsplash.com/photo-1588466585717-f8041aec7875?auto=format&fit=crop&w=1374&q=80"
			/>
			<Image
				size="md"
				rounded="md"
				src="https://images.unsplash.com/photo-1588466585717-f8041aec7875?auto=format&fit=crop&w=1374&q=80"
			/>
			<Image
				size="lg"
				rounded="lg"
				src="https://images.unsplash.com/photo-1588466585717-f8041aec7875?auto=format&fit=crop&w=1374&q=80"
			/>
		</div>
	),
};

export const FallbackImage: StoryObj<ImageProps> = {
	args: {
		size: 'lg',
		rounded: 'lg',
		src: '',
	},
};

export const ModeratedBlurred: StoryObj<ImageProps> = {
	args: {
		src: 'https://upload.wikimedia.org/wikipedia/commons/4/4f/SIG_Pro_by_Augustas_Didzgalvis.jpg',
		size: 'lg',
		rounded: 'lg',
		visibility: VISIBILITY.BLUR,
	},
};
