import React from 'react';
import SearchFilterModal from './SearchFilterModal';
import {
  Button,
  Checkbox,
  Dropdown,
  Header,
  Icon,
  Input,
  Label,
  List,
  Popup,
  Segment,
} from 'semantic-ui-react';

const sortDropdownOptions = [
  {
    key: 'smart',
    text: '⭐ Smart Ranking (Best Overall)',
    value: 'smart',
  },
  {
    key: 'uploadSpeed',
    text: 'Upload Speed (Fastest to Slowest)',
    value: 'uploadSpeed',
  },
  {
    key: 'queueLength',
    text: 'Queue Depth (Least to Most)',
    value: 'queueLength',
  },
  {
    key: 'fileCount',
    text: 'File Count (Most to Least)',
    value: 'fileCount',
  },
];

const SearchResultControls = ({ actions, state }) => {
  const {
    albumCandidates,
    blockedUsers,
    deduplicatedResults,
    foldDuplicateResults,
    foldResults,
    hasSavedDefault,
    hideBlockedUsers,
    hideLocked,
    hideNoFreeSlots,
    loaded,
    pageSize,
    resultFilters,
    resultSort,
    savedFilters,
    search,
  } = state;
  const {
    clearSavedDefault,
    deleteNamedFilter,
    focusAlbumCandidate,
    handleFoldDuplicateResultsChange,
    handlePageSizeChange,
    loadNamedFilter,
    saveAlbumCandidateRule,
    saveAsDefault,
    saveNamedFilter,
    setFoldResults,
    setHideBlockedUsers,
    setHideLocked,
    setHideNoFreeSlots,
    setResultFilters,
    setResultSort,
  } = actions;

  return (
    <>
        {loaded && (
          <Segment
            className="search-options"
            raised
          >
            <Dropdown
              aria-label="Sort search results"
              button
              className="search-options-sort icon"
              floating
              icon="sort"
              labeled
              onChange={(_event, { value }) => setResultSort(value)}
              options={sortDropdownOptions}
              text={
                (sortDropdownOptions.find((o) => o.value === resultSort) ??
                  sortDropdownOptions[0]).text
              }
            />
            <Dropdown
              aria-label="Search results per page"
              button
              className="search-options-pagesize"
              floating
              onChange={(_event, { value }) => handlePageSizeChange(value)}
              options={[
                { key: '10', text: '10 per page', value: 10 },
                { key: '25', text: '25 per page', value: 25 },
                { key: '50', text: '50 per page', value: 50 },
                { key: '100', text: '100 per page', value: 100 },
                { key: 'all', text: 'Show All', value: 999_999 },
              ]}
              style={{ marginLeft: '0.5em' }}
              text={pageSize >= 999_999 ? 'Show All' : `${pageSize} per page`}
            />
            <div className="search-option-toggles">
              <Checkbox
                checked={hideLocked}
                className="search-options-hide-locked"
                label="Hide Locked Results"
                onChange={() => setHideLocked(!hideLocked)}
                toggle
              />
              <Checkbox
                checked={hideNoFreeSlots}
                className="search-options-hide-no-slots"
                label="Hide Results with No Free Slots"
                onChange={() => setHideNoFreeSlots(!hideNoFreeSlots)}
                toggle
              />
              <Checkbox
                checked={hideBlockedUsers}
                className="search-options-hide-blocked"
                label={`Hide Blocked Users (${blockedUsers.length})`}
                onChange={() => setHideBlockedUsers(!hideBlockedUsers)}
                toggle
              />
              <Checkbox
                checked={foldResults}
                className="search-options-fold-results"
                label="Fold Results"
                onChange={() => setFoldResults(!foldResults)}
                toggle
              />
              <Popup
                content="Fold duplicate file candidates that appear from multiple providers or peers, keeping the highest-ranked visible result and showing the folded sources as metadata."
                position="top center"
                trigger={
                  <Checkbox
                    checked={foldDuplicateResults}
                    className="search-options-fold-duplicates"
                    label={`Fold Duplicates${
                      deduplicatedResults.foldedCount > 0
                        ? ` (${deduplicatedResults.foldedCount})`
                        : ''
                    }`}
                    onChange={handleFoldDuplicateResultsChange}
                    toggle
                  />
                }
              />
            </div>
            <div
              className="search-wishlist-ignore-guidance"
              role="note"
            >
              <Icon name={search.wishlistItemId ? 'ban' : 'info circle'} />
              {search.wishlistItemId
                ? 'Wishlist search: use “Ignore for Wishlist” below a result folder to hide that peer and folder from future runs of this wishlist item.'
                : 'Folder ignores are available only for wishlist searches because each rule belongs to one wishlist item. Open a result from Wishlist history to use them.'}
            </div>
            <Input
              action={
                <Button.Group>
                  {savedFilters.length > 0 && (
                    <Dropdown
                      button
                      className="icon"
                      floating
                      icon="bookmark"
                      onChange={(_event, { value }) => loadNamedFilter(value)}
                      options={savedFilters.map((filter) => ({
                        key: filter.name,
                        text: filter.name,
                        value: filter.name,
                      }))}
                      title="Load saved filter"
                    />
                  )}
                  {Boolean(resultFilters) && (
                    <Button
                      color="red"
                      icon="x"
                      onClick={() => setResultFilters('')}
                      title="Clear current filter"
                    />
                  )}
                  {Boolean(resultFilters) && (
                    <Button
                      color="teal"
                      icon="bookmark"
                      onClick={saveNamedFilter}
                      title="Save named filter"
                    />
                  )}
                  {savedFilters.some((filter) => filter.value === resultFilters) && (
                    <Button
                      color="orange"
                      icon="minus circle"
                      onClick={deleteNamedFilter}
                      title="Delete matching saved filter"
                    />
                  )}
                  <Button
                    color="blue"
                    icon="save"
                    onClick={saveAsDefault}
                    title="Save as default filter"
                  />
                  {hasSavedDefault && (
                    <Button
                      color="orange"
                      icon="trash"
                      onClick={clearSavedDefault}
                      title="Clear saved default filter"
                    />
                  )}
                  <SearchFilterModal
                    filterString={resultFilters}
                    onChange={setResultFilters}
                    trigger={
                      <Button
                        icon
                        title="Advanced Filters"
                      >
                        <Icon name="sliders horizontal" />
                      </Button>
                    }
                  />
                </Button.Group>
              }
              className="search-filter"
              label={{ content: 'Filter', icon: 'filter' }}
              onChange={(_event, data) => setResultFilters(data.value)}
              placeholder="
                lackluster container -bothersome iscbr|isvbr islossless|islossy 
                minbr:320 minfilesize:100mb maxfilesize:2gb minfilesinfolder:8 minlength:5000
              "
              value={resultFilters}
            />
          </Segment>
        )}
        {loaded && albumCandidates.length > 0 && (
          <Segment
            className="search-album-picker-segment"
            raised
          >
            <Header as="h4">
              Album candidates
              <Label
                color="blue"
                size="mini"
              >
                {albumCandidates.length}
              </Label>
            </Header>
            <List
              className="search-album-candidate-list"
              divided
              relaxed
            >
              {albumCandidates.map((candidate) => (
                <List.Item
                  className="search-album-candidate"
                  key={candidate.key}
                >
                  <List.Content floated="right">
                    <Popup
                      content="Save this visible album review as a browser-local rule preview for similar future searches. This does not alter download behavior or contact peers."
                      position="top center"
                      trigger={
                        <Button
                          aria-label={`Save album rule ${candidate.albumTitle}`}
                          icon="bookmark outline"
                          onClick={() => saveAlbumCandidateRule(candidate)}
                          size="mini"
                        />
                      }
                    />
                    <Popup
                      content="Focus the current result filter on this album folder name without starting another search or download."
                      position="top center"
                      trigger={
                        <Button
                          aria-label={`Focus album candidate ${candidate.albumTitle}`}
                          icon="filter"
                          onClick={() => focusAlbumCandidate(candidate)}
                          size="mini"
                        />
                      }
                    />
                  </List.Content>
                  <List.Content>
                    <List.Header>
                      {candidate.albumTitle}
                      <Label
                        color="purple"
                        size="tiny"
                      >
                        {candidate.score}/100
                      </Label>
                    </List.Header>
                    <List.Description>
                      {candidate.trackCount}/{candidate.expectedTrackCount}{' '}
                      visible tracks · {candidate.sourceCount} source
                      {candidate.sourceCount === 1 ? '' : 's'} ·{' '}
                      {Math.round(candidate.completenessRatio * 100)}%
                    </List.Description>
                    <div className="search-album-candidate-review">
                      <span>
                        Formats:{' '}
                        {candidate.formatMix
                          .map((item) => `${item.format} ${item.count}`)
                          .join(', ')}
                      </span>
                      {candidate.missingTrackNumbers.length > 0 && (
                        <span>
                          Missing:{' '}
                          {candidate.missingTrackNumbers.slice(0, 8).join(', ')}
                        </span>
                      )}
                      {candidate.durationVarianceSeconds > 0 && (
                        <span>
                          Duration spread:{' '}
                          {Math.round(candidate.durationVarianceSeconds / 60)}m
                        </span>
                      )}
                      {candidate.substitutionOptions.length > 0 && (
                        <span>
                          Substitutions:{' '}
                          {candidate.substitutionOptions
                            .map(
                              (option) =>
                                `track ${option.trackNumber} (${option.optionCount})`,
                            )
                            .join(', ')}
                        </span>
                      )}
                    </div>
                    {candidate.substitutionOptions.length > 0 && (
                      <div className="search-album-candidate-substitutions">
                        {candidate.substitutionOptions.slice(0, 4).map((option) => (
                          <Popup
                            content={`Manual review options from ${option.sources.join(', ')} in ${option.formats.join(', ')}. This only describes visible alternatives; it does not select or download them.`}
                            key={option.trackNumber}
                            position="top center"
                            trigger={
                              <Label
                                color="teal"
                                size="tiny"
                              >
                                <Icon name="exchange" />
                                Track {option.trackNumber}: {option.optionCount}{' '}
                                options
                              </Label>
                            }
                          />
                        ))}
                      </div>
                    )}
                    <div className="search-album-candidate-labels">
                      {candidate.reasons.map((reason) => (
                        <Label
                          key={reason}
                          size="tiny"
                        >
                          {reason}
                        </Label>
                      ))}
                      {candidate.warnings.map((warning) => (
                        <Popup
                          content="This is a local confidence warning from visible search result metadata only; it does not reject the candidate or contact peers."
                          key={warning}
                          position="top center"
                          trigger={
                            <Label
                              color="yellow"
                              size="tiny"
                            >
                              <Icon name="warning sign" />
                              {warning}
                            </Label>
                          }
                        />
                      ))}
                    </div>
                    <div className="search-album-candidate-paths">
                      {candidate.directories.join(' | ')}
                    </div>
                  </List.Content>
                </List.Item>
              ))}
            </List>
          </Segment>
        )}

    </>
  );
};

export default SearchResultControls;
