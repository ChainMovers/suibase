// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { useSuiClientQuery } from '@mysten/dapp-kit';
import { ArrowRight12 } from '@mysten/icons';
import { type SuiValidatorSummary } from '@mysten/sui.js/client';
import { Text } from '@mysten/ui';
import { useMemo } from 'react';

import { HighlightedTableCol } from "~/components/Table/HighlightedTableCol";
import { Banner } from "~/ui/Banner";
import { ImageIcon } from "~/ui/ImageIcon";
import { AddressLink, ValidatorLink } from "~/ui/InternalLink";
import { Link } from "~/ui/Link";
import { PlaceholderTable } from "~/ui/PlaceholderTable";
import { TableCard } from "~/ui/TableCard";
import { StakeColumn } from "./StakeColumn";

const NUMBER_OF_VALIDATORS = 10;

export function processValidators(set: SuiValidatorSummary[]) {
	return set.map((av) => ({
		name: av.name,
		address: av.suiAddress,
		stake: av.stakingPoolSuiBalance,
		logo: av.imageUrl,
	}));
}

const validatorsTable = (
	validatorsData: SuiValidatorSummary[],
	limit?: number,
	showIcon?: boolean,
) => {
	const validators = processValidators(validatorsData).sort((a, b) =>
		Math.random() > 0.5 ? -1 : 1,
	);

	const validatorsItems = limit ? validators.splice(0, limit) : validators;

	return {
    data: validatorsItems.map(({ name, stake, address, logo }) => ({
      name: (
        <HighlightedTableCol first>
          <div className="flex items-center gap-2.5">
            {showIcon && (
              <ImageIcon
                src={logo}
                size="sm"
                fallback={name}
                label={name}
                circle
              />
            )}

            <ValidatorLink address={address} label={name} />
          </div>
        </HighlightedTableCol>
      ),
      stake: <StakeColumn stake={stake} />,
      delegation: (
        <Text variant="bodySmall/medium" color="steel-darker">
          {stake.toString()}
        </Text>
      ),
      address: (
        <HighlightedTableCol>
          <AddressLink address={address} noTruncate={!limit} />
        </HighlightedTableCol>
      ),
    })),
    columns: [
      {
        header: "Name",
        accessorKey: "name",
      },
      {
        header: "Address",
        accessorKey: "address",
      },
      {
        header: "Stake",
        accessorKey: "stake",
      },
    ],
  };
};

type TopValidatorsCardProps = {
	limit?: number;
	showIcon?: boolean;
};

export function TopValidatorsCard({ limit, showIcon }: TopValidatorsCardProps) {
	const { data, isPending, isSuccess, isError } = useSuiClientQuery('getLatestSuiSystemState');

	const tableData = useMemo(
		() => (data ? validatorsTable(data.activeValidators, limit, showIcon) : null),
		[data, limit, showIcon],
	);

	if (isError || (!isPending && !tableData?.data.length)) {
		return (
			<Banner variant="error" fullWidth>
				Validator data could not be loaded
			</Banner>
		);
	}

	return (
		<>
			{isPending && (
				<PlaceholderTable
					rowCount={limit || NUMBER_OF_VALIDATORS}
					rowHeight="13px"
					colHeadings={['Name', 'Address', 'Stake']}
					colWidths={['220px', '220px', '220px']}
				/>
			)}

			{isSuccess && tableData && (
				<>
					<TableCard data={tableData.data} columns={tableData.columns} />
					<div className="mt-3 flex justify-between">
						<Link to="/validators">
							<div className="flex items-center gap-2">
								View all
								<ArrowRight12 fill="currentColor" className="h-3 w-3 -rotate-45" />
							</div>
						</Link>
						<Text variant="body/medium" color="steel-dark">
							{data ? data.activeValidators.length : '-'}
							{` Total`}
						</Text>
					</div>
				</>
			)}
		</>
	);
}
