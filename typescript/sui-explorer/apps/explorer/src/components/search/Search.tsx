// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
import { useCallback, useState } from "react";

import { useDebouncedValue } from "~/hooks/useDebouncedValue";
import { useSearch } from "~/hooks/useSearch";
import { Search as SearchBox, type SearchResult } from "~/ui/Search";
import { useNavigateWithQuery } from "~/ui/utils/LinkWithQuery";

function Search() {
  const [query, setQuery] = useState("");
  const debouncedQuery = useDebouncedValue(query);
  const { isPending, data: results } = useSearch(debouncedQuery);
  const navigate = useNavigateWithQuery();
  const handleSelectResult = useCallback(
    (result: SearchResult) => {
      if (result) {
        navigate(`/${result?.type}/${encodeURIComponent(result?.id)}`, {});
        setQuery("");
      }
    },
    [navigate]
  );

  return (
    <div className="max-w flex">
      <SearchBox
        queryValue={query}
        onChange={(value) => setQuery(value?.trim() ?? "")}
        onSelectResult={handleSelectResult}
        placeholder="Search"
        isLoading={isPending || debouncedQuery !== query}
        options={results}
      />
    </div>
  );
}

export default Search;
