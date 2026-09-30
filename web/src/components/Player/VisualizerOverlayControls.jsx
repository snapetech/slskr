import React from 'react';
import { Button, Icon, Popup } from 'semantic-ui-react';
import {
  getEngineIcon,
  getEngineLabel,
  getNextEngine,
  getRustyMilkAutomationLabel,
  isRustyMilkEngine,
  nativeEditableParameters,
} from './visualizerPresetLibrary';

const VisualizerOverlayControls = ({
  actions,
  compactControls,
  mode,
  onModeChange,
  refs,
  state,
}) => {
  const {
    activeEngineType,
    activeNativePlaylistId,
    activeRustyMilkPresetId,
    activeRustyMilkPresetIsFavorite,
    canSaveNativePlaylist,
    hasNativeShapes,
    hasNativeWaves,
    hasRustyMilkPresetSearch,
    nativeBankNavigationDisabled,
    nativeFpsCap,
    nativeQualityPreset,
    rustyMilkAutomationMode,
    rustyMilkAutomationSettings,
    rustyMilkFavoritePresetIds,
    rustyMilkFragmentSummary,
    rustyMilkLibraryMode,
    rustyMilkParameter,
    rustyMilkParameterValue,
    rustyMilkPresetLibrary,
    rustyMilkPresetPlaylists,
    rustyMilkPresetSearch,
    selectedNativeShapeIndex,
    selectedNativeWaveIndex,
    selectedRustyMilkParameter,
    selectedRustyMilkPresetValue,
    showNativeDebug,
    visibleRustyMilkPresetLibrary,
  } = state;
  const {
    applyRustyMilkParameterEdit,
    clearActiveNativePlaylist,
    clearRustyMilkPresetLibrary,
    clearRustyMilkPresetSearch,
    cycleEngineType,
    cyclePreset,
    cycleRustyMilkAutomationMode,
    enterFullscreen,
    exitFullscreen,
    exportRustyMilkFragment,
    exportRustyMilkPreset,
    importRustyMilkPreset,
    loadRustyMilkLibraryPreset,
    previousRustyMilkLibraryPreset,
    randomRustyMilkLibraryPreset,
    randomizeRustyMilkPresetParameters,
    removeActiveNativePlaylist,
    removeActiveRustyMilkPreset,
    removeRustyMilkFragment,
    renameActiveNativePlaylist,
    saveNativePlaylistFromVisibleBank,
    selectNativePlaylist,
    selectRustyMilkParameter,
    setSelectedNativeShapeIndex,
    setSelectedNativeWaveIndex,
    setShowNativeDebug,
    toggleRustyMilkLibraryMode,
    toggleRustyMilkPresetFavorite,
    updateNativeFpsCap,
    updateNativeQualityPreset,
    updateRustyMilkAutomationBeats,
    updateRustyMilkAutomationInterval,
    updateRustyMilkParameterDraft,
    updateRustyMilkPresetSearch,
  } = actions;
  const { directoryInputRef, fileInputRef } = refs;

  return (
        <div
          className="player-visualizer-overlay-controls"
          onClick={(event) => event.stopPropagation()}
        >
          <input
            accept=".milk,.milk2,.shape,.wave,text/plain,image/png,image/jpeg,image/webp,image/gif"
            hidden
            multiple
            onChange={importRustyMilkPreset}
            ref={fileInputRef}
            type="file"
          />
          <input
            data-testid="visualizer-native-pack-input"
            directory=""
            hidden
            multiple
            onChange={importRustyMilkPreset}
            ref={directoryInputRef}
            type="file"
            webkitdirectory=""
          />
          <Popup
            content={`Cycle visualizer engine to ${getEngineLabel(getNextEngine(activeEngineType))}.`}
            trigger={
              <Button
                aria-label={`Cycle visualizer engine to ${getEngineLabel(getNextEngine(activeEngineType))}`}
                data-testid="visualizer-switch-engine"
                icon
                onClick={cycleEngineType}
                size="mini"
              >
                <Icon name={getEngineIcon(activeEngineType)} />
              </Button>
            }
          />
          {isRustyMilkEngine(activeEngineType) && !compactControls ? (
            <>
              {rustyMilkPresetLibrary.length > 0 ? (
                <Popup
                  content="Filter imported RustyMilk presets by title or file name. The current filter also scopes next and random preset jumps."
                  trigger={
                    <input
                      aria-label="Search RustyMilk presets"
                      className="player-visualizer-native-search"
                      data-testid="visualizer-native-preset-search"
                      onChange={updateRustyMilkPresetSearch}
                      placeholder="Search presets"
                      type="search"
                      value={rustyMilkPresetSearch}
                    />
                  }
                />
              ) : null}
              {rustyMilkPresetLibrary.length > 0 ? (
                <Popup
                  content="Clear the RustyMilk preset search filter."
                  trigger={
                    <Button
                      aria-label="Clear RustyMilk preset search"
                      data-testid="visualizer-clear-native-preset-search"
                      disabled={!hasRustyMilkPresetSearch}
                      icon
                      onClick={clearRustyMilkPresetSearch}
                      size="mini"
                    >
                      <Icon name="remove" />
                    </Button>
                  }
                />
              ) : null}
              {rustyMilkPresetPlaylists.length > 0 ? (
                <Popup
                  content="Use a saved native playlist as the active preset bank."
                  trigger={
                    <select
                      aria-label="RustyMilk playlist"
                      className="player-visualizer-native-library"
                      data-testid="visualizer-native-playlist"
                      onChange={selectNativePlaylist}
                      value={activeNativePlaylistId}
                    >
                      <option value="">All imported</option>
                      {rustyMilkPresetPlaylists.map((playlist) => (
                        <option key={playlist.id} value={playlist.id}>
                          {playlist.name}
                        </option>
                      ))}
                    </select>
                  }
                />
              ) : null}
              {rustyMilkPresetLibrary.length > 0 ? (
                <Popup
                  content="Save the current visible RustyMilk preset bank as a browser-local playlist."
                  trigger={
                    <Button
                      aria-label="Save visible RustyMilk presets as playlist"
                      data-testid="visualizer-save-native-playlist"
                      disabled={!canSaveNativePlaylist}
                      icon
                      onClick={saveNativePlaylistFromVisibleBank}
                      size="mini"
                    >
                      <Icon name="save outline" />
                    </Button>
                  }
                />
              ) : null}
              {activeNativePlaylistId ? (
                <Popup
                  content="Rename the active native playlist in this browser."
                  trigger={
                    <Button
                      aria-label="Rename active native playlist"
                      data-testid="visualizer-rename-native-playlist"
                      icon
                      onClick={renameActiveNativePlaylist}
                      size="mini"
                    >
                      <Icon name="edit outline" />
                    </Button>
                  }
                />
              ) : null}
              {activeNativePlaylistId ? (
                <Popup
                  content="Return to the full imported RustyMilk preset bank without deleting this playlist."
                  trigger={
                    <Button
                      aria-label="Clear active native playlist"
                      data-testid="visualizer-clear-active-native-playlist"
                      icon
                      onClick={clearActiveNativePlaylist}
                      size="mini"
                    >
                      <Icon name="list" />
                    </Button>
                  }
                />
              ) : null}
              {activeNativePlaylistId ? (
                <Popup
                  content="Delete the active native playlist from this browser."
                  trigger={
                    <Button
                      aria-label="Delete active native playlist"
                      data-testid="visualizer-remove-native-playlist"
                      icon
                      onClick={removeActiveNativePlaylist}
                      size="mini"
                    >
                      <Icon name="times circle outline" />
                    </Button>
                  }
                />
              ) : null}
              {rustyMilkPresetLibrary.length > 0 ? (
                <Popup
                  content={
                    rustyMilkLibraryMode === 'favorites'
                      ? 'Reload a favorite RustyMilk preset from this browser.'
                      : 'Reload a previously imported RustyMilk preset from this browser.'
                  }
                  trigger={
                    <select
                      aria-label="RustyMilk preset library"
                      className="player-visualizer-native-library"
                      data-testid="visualizer-native-preset-library"
                      onChange={loadRustyMilkLibraryPreset}
                      value={selectedRustyMilkPresetValue}
                    >
                      <option value="">
                        {visibleRustyMilkPresetLibrary.length === 0 ? 'No matches' : (
                          rustyMilkLibraryMode === 'favorites' ? 'Favorites' : 'Presets'
                        )}
                      </option>
                      {visibleRustyMilkPresetLibrary.map((preset) => (
                        <option key={preset.id} value={preset.id}>
                          {rustyMilkFavoritePresetIds.includes(preset.id) ? '(favorite) ' : ''}
                          {preset.title || preset.fileName}
                        </option>
                      ))}
                    </select>
                  }
                />
              ) : null}
              {rustyMilkPresetLibrary.length > 0 ? (
                <Popup
                  content={
                    activeRustyMilkPresetIsFavorite
                      ? 'Remove the active RustyMilk preset from favorites.'
                      : 'Mark the active RustyMilk preset as a favorite.'
                  }
                  trigger={
                    <Button
                      aria-label={
                        activeRustyMilkPresetIsFavorite
                          ? 'Unfavorite active RustyMilk preset'
                          : 'Favorite active RustyMilk preset'
                      }
                      active={activeRustyMilkPresetIsFavorite}
                      data-testid="visualizer-toggle-native-favorite"
                      disabled={!activeRustyMilkPresetId}
                      icon
                      onClick={toggleRustyMilkPresetFavorite}
                      size="mini"
                    >
                      <Icon name={activeRustyMilkPresetIsFavorite ? 'star' : 'star outline'} />
                    </Button>
                  }
                />
              ) : null}
              {rustyMilkPresetLibrary.length > 0 ? (
                <Popup
                  content={
                    rustyMilkLibraryMode === 'favorites'
                      ? 'Show all imported RustyMilk presets.'
                      : 'Show only favorite RustyMilk presets.'
                  }
                  trigger={
                    <Button
                      aria-label={
                        rustyMilkLibraryMode === 'favorites'
                          ? 'Show all RustyMilk presets'
                          : 'Show favorite RustyMilk presets'
                      }
                      active={rustyMilkLibraryMode === 'favorites'}
                      data-testid="visualizer-toggle-native-favorites-only"
                      disabled={rustyMilkFavoritePresetIds.length === 0}
                      icon
                      onClick={toggleRustyMilkLibraryMode}
                      size="mini"
                    >
                      <Icon name="filter" />
                    </Button>
                  }
                />
              ) : null}
              {rustyMilkPresetLibrary.length > 1 ? (
                <Popup
                  content="Return to the previous RustyMilk preset, or move backward in the local preset library."
                  trigger={
                    <Button
                      aria-label="Previous RustyMilk preset"
                      data-testid="visualizer-previous-native-preset"
                      disabled={visibleRustyMilkPresetLibrary.length === 0}
                      icon
                      onClick={previousRustyMilkLibraryPreset}
                      size="mini"
                    >
                      <Icon name="step backward" />
                    </Button>
                  }
                />
              ) : null}
              {rustyMilkPresetLibrary.length > 1 ? (
                <Popup
                  content="Jump to a random imported RustyMilk preset from this browser."
                  trigger={
                    <Button
                      aria-label="Random imported RustyMilk preset"
                      data-testid="visualizer-random-native-preset"
                      disabled={visibleRustyMilkPresetLibrary.length === 0}
                      icon
                      onClick={randomRustyMilkLibraryPreset}
                      size="mini"
                    >
                      <Icon name="random" />
                    </Button>
                  }
                />
              ) : null}
              {rustyMilkPresetLibrary.length > 0 ? (
                <Popup
                  content="Remove the selected RustyMilk preset from this browser."
                  trigger={
                    <Button
                      aria-label="Remove selected RustyMilk preset"
                      data-testid="visualizer-remove-native-preset"
                      disabled={!activeRustyMilkPresetId}
                      icon
                      onClick={removeActiveRustyMilkPreset}
                      size="mini"
                    >
                      <Icon name="minus circle" />
                    </Button>
                  }
                />
              ) : null}
              {rustyMilkPresetLibrary.length > 0 ? (
                <Popup
                  content="Clear imported RustyMilk presets from this browser."
                  trigger={
                    <Button
                      aria-label="Clear imported RustyMilk presets"
                      data-testid="visualizer-clear-native-preset-library"
                      icon
                      onClick={clearRustyMilkPresetLibrary}
                      size="mini"
                    >
                      <Icon name="trash alternate outline" />
                    </Button>
                  }
                />
              ) : null}
              <Popup
                content="Import a local .milk or .milk2 preset into the RustyMilk renderer."
                trigger={
                  <Button
                    aria-label="Import RustyMilk preset"
                    data-testid="visualizer-import-native-preset"
                    icon
                    onClick={() => fileInputRef.current?.click()}
                    size="mini"
                  >
                    <Icon name="upload" />
                  </Button>
                }
              />
              <Popup
                content="Import a RustyMilk preset folder with its local image assets."
                trigger={
                  <Button
                    aria-label="Import RustyMilk preset folder"
                    data-testid="visualizer-import-native-preset-folder"
                    icon
                    onClick={() => directoryInputRef.current?.click()}
                    size="mini"
                  >
                    <Icon name="folder open outline" />
                  </Button>
                }
              />
              <Popup
                content="Choose a global RustyMilk parameter to edit on the active preset."
                trigger={
                  <select
                    aria-label="RustyMilk editable parameter"
                    className="player-visualizer-native-library"
                    data-testid="visualizer-native-parameter"
                    onChange={selectRustyMilkParameter}
                    value={selectedRustyMilkParameter}
                  >
                    {nativeEditableParameters.map((parameter) => (
                      <option key={parameter.key} value={parameter.key}>
                        {parameter.label}
                      </option>
                    ))}
                  </select>
                }
              />
              <Popup
                content={`Adjust ${rustyMilkParameter.label.toLowerCase()} for the active RustyMilk preset before applying the edited copy locally.`}
                trigger={
                  <input
                    aria-label={`Adjust RustyMilk ${rustyMilkParameter.label}`}
                    className="player-visualizer-native-range"
                    data-testid="visualizer-native-parameter-value"
                    max={rustyMilkParameter.max}
                    min={rustyMilkParameter.min}
                    onChange={updateRustyMilkParameterDraft}
                    step={rustyMilkParameter.step}
                    type="range"
                    value={rustyMilkParameterValue}
                  />
                }
              />
              <Popup
                content="Apply the selected RustyMilk parameter value and save the edited preset in this browser."
                trigger={
                  <Button
                    aria-label="Apply RustyMilk parameter edit"
                    data-testid="visualizer-apply-native-parameter"
                    icon
                    onClick={applyRustyMilkParameterEdit}
                    size="mini"
                  >
                    <Icon name="sliders horizontal" />
                  </Button>
                }
              />
              <Popup
                content="Randomize the active RustyMilk preset's common visual parameters and save the edited copy locally."
                trigger={
                  <Button
                    aria-label="Randomize RustyMilk visual parameters"
                    data-testid="visualizer-randomize-native-parameters"
                    icon
                    onClick={randomizeRustyMilkPresetParameters}
                    size="mini"
                  >
                    <Icon name="shuffle" />
                  </Button>
                }
              />
              <Popup
                content="Show or hide RustyMilk debug details for the active preset."
                trigger={
                  <Button
                    aria-label={showNativeDebug ? 'Hide RustyMilk debug details' : 'Show RustyMilk debug details'}
                    active={showNativeDebug}
                    data-testid="visualizer-toggle-native-debug"
                    icon
                    onClick={() => setShowNativeDebug((current) => !current)}
                    size="mini"
                  >
                    <Icon name="bug" />
                  </Button>
                }
              />
              <Popup
                content="Cap RustyMilk rendering for lower GPU load, or leave it uncapped for maximum smoothness."
                trigger={
                  <select
                    aria-label="RustyMilk FPS cap"
                    className="player-visualizer-native-library"
                    data-testid="visualizer-native-fps-cap"
                    onChange={updateNativeFpsCap}
                    value={nativeFpsCap}
                  >
                    <option value="full">Full FPS</option>
                    <option value="60">60 FPS</option>
                    <option value="30">30 FPS</option>
                    <option value="24">24 FPS</option>
                  </select>
                }
              />
              <Popup
                content="Choose a RustyMilk quality preset. Efficient lowers GPU load, Balanced caps at 60 FPS, and Full leaves rendering uncapped."
                trigger={
                  <select
                    aria-label="RustyMilk quality preset"
                    className="player-visualizer-native-library"
                    data-testid="visualizer-native-quality"
                    onChange={updateNativeQualityPreset}
                    value={nativeQualityPreset}
                  >
                    <option value="balanced">Balanced</option>
                    <option value="efficient">Efficient</option>
                    <option value="full">Full</option>
                    <option value="custom">Custom</option>
                  </select>
                }
              />
              <Popup
                content="Export the active RustyMilk preset text after any local edits."
                trigger={
                  <Button
                    aria-label="Export active RustyMilk preset"
                    data-testid="visualizer-export-native-preset"
                    icon
                    onClick={exportRustyMilkPreset}
                    size="mini"
                  >
                    <Icon name="file alternate outline" />
                  </Button>
                }
              />
              <Popup
                content="Choose which custom shape from the active RustyMilk preset should be exported or removed."
                trigger={
                  <select
                    aria-label="RustyMilk shape fragment"
                    className="player-visualizer-native-library"
                    data-testid="visualizer-native-shape-fragment"
                    disabled={!hasNativeShapes}
                    onChange={(event) => setSelectedNativeShapeIndex(Number(event.target.value))}
                    value={hasNativeShapes ? selectedNativeShapeIndex : 0}
                  >
                    {hasNativeShapes ? rustyMilkFragmentSummary.shapes.map((shape) => (
                      <option key={shape.index} value={shape.index}>
                        {shape.label}
                      </option>
                    )) : (
                      <option value={0}>No shapes</option>
                    )}
                  </select>
                }
              />
              <Popup
                content="Export the selected custom shape in the active RustyMilk preset as a .shape fragment."
                trigger={
                  <Button
                    aria-label="Export RustyMilk shape fragment"
                    data-testid="visualizer-export-native-shape"
                    disabled={!hasNativeShapes}
                    icon
                    onClick={() => exportRustyMilkFragment('shape')}
                    size="mini"
                  >
                    <Icon name="download" />
                  </Button>
                }
              />
              <Popup
                content="Remove the selected custom shape from the active RustyMilk preset and persist the edited copy locally."
                trigger={
                  <Button
                    aria-label="Remove RustyMilk shape fragment"
                    data-testid="visualizer-remove-native-shape"
                    disabled={!hasNativeShapes}
                    icon
                    onClick={() => removeRustyMilkFragment('shape')}
                    size="mini"
                  >
                    <Icon name="erase" />
                  </Button>
                }
              />
              <Popup
                content="Choose which custom wave from the active RustyMilk preset should be exported or removed."
                trigger={
                  <select
                    aria-label="RustyMilk wave fragment"
                    className="player-visualizer-native-library"
                    data-testid="visualizer-native-wave-fragment"
                    disabled={!hasNativeWaves}
                    onChange={(event) => setSelectedNativeWaveIndex(Number(event.target.value))}
                    value={hasNativeWaves ? selectedNativeWaveIndex : 0}
                  >
                    {hasNativeWaves ? rustyMilkFragmentSummary.waves.map((wave) => (
                      <option key={wave.index} value={wave.index}>
                        {wave.label}
                      </option>
                    )) : (
                      <option value={0}>No waves</option>
                    )}
                  </select>
                }
              />
              <Popup
                content="Export the selected custom wave in the active RustyMilk preset as a .wave fragment."
                trigger={
                  <Button
                    aria-label="Export RustyMilk wave fragment"
                    data-testid="visualizer-export-native-wave"
                    disabled={!hasNativeWaves}
                    icon
                    onClick={() => exportRustyMilkFragment('wave')}
                    size="mini"
                  >
                    <Icon name="download" />
                  </Button>
                }
              />
              <Popup
                content="Remove the selected custom wave from the active RustyMilk preset and persist the edited copy locally."
                trigger={
                  <Button
                    aria-label="Remove RustyMilk wave fragment"
                    data-testid="visualizer-remove-native-wave"
                    disabled={!hasNativeWaves}
                    icon
                    onClick={() => removeRustyMilkFragment('wave')}
                    size="mini"
                  >
                    <Icon name="erase" />
                  </Button>
                }
              />
              <Popup
                content={`Native automatic preset changes: ${getRustyMilkAutomationLabel(rustyMilkAutomationMode)}. Beat mode advances after repeated detected bass beats; timed mode advances on an interval.`}
                trigger={
                  <Button
                    aria-label={`Native automatic preset changes: ${getRustyMilkAutomationLabel(rustyMilkAutomationMode)}`}
                    active={rustyMilkAutomationMode !== 'off'}
                    data-testid="visualizer-native-automation"
                    icon
                    onClick={cycleRustyMilkAutomationMode}
                    size="mini"
                  >
                    <Icon name={rustyMilkAutomationMode === 'beat' ? 'heartbeat' : 'clock outline'} />
                  </Button>
                }
              />
              {rustyMilkAutomationMode === 'beat' ? (
                <Popup
                  content="Choose how many detected bass beats should pass before RustyMilk advances to another preset."
                  trigger={
                    <select
                      aria-label="RustyMilk beats per preset"
                      className="player-visualizer-native-library"
                      data-testid="visualizer-native-automation-beats"
                      onChange={updateRustyMilkAutomationBeats}
                      value={rustyMilkAutomationSettings.beatsPerPreset}
                    >
                      <option value={4}>4 beats</option>
                      <option value={8}>8 beats</option>
                      <option value={16}>16 beats</option>
                    </select>
                  }
                />
              ) : null}
              {rustyMilkAutomationMode === 'timed' ? (
                <Popup
                  content="Choose how long RustyMilk should wait before timed preset changes."
                  trigger={
                    <select
                      aria-label="RustyMilk timed preset interval"
                      className="player-visualizer-native-library"
                      data-testid="visualizer-native-automation-interval"
                      onChange={updateRustyMilkAutomationInterval}
                      value={rustyMilkAutomationSettings.timedIntervalSeconds}
                    >
                      <option value={15}>15 sec</option>
                      <option value={30}>30 sec</option>
                      <option value={60}>60 sec</option>
                    </select>
                  }
                />
              ) : null}
            </>
          ) : null}
          <Popup
            content={
              nativeBankNavigationDisabled
                ? 'No imported RustyMilk presets match the current filter.'
                : 'Load a different RustyMilk preset.'
            }
            trigger={
              <Button
                aria-label="Next visualizer preset"
                data-testid="visualizer-next-preset"
                disabled={nativeBankNavigationDisabled}
                icon
                onClick={cyclePreset}
                size="mini"
              >
                <Icon name="random" />
              </Button>
            }
          />
          {mode === 'inline' && !compactControls ? (
            <>
              <Popup
                content="Expand visualizer to fill the browser window."
                trigger={
                  <Button
                    aria-label="Expand visualizer to full browser window"
                    data-testid="visualizer-fullwindow"
                    icon
                    onClick={() => onModeChange('fullwindow')}
                    size="mini"
                  >
                    <Icon name="expand arrows alternate" />
                  </Button>
                }
              />
              <Popup
                content="Enter true fullscreen."
                trigger={
                  <Button
                    aria-label="Enter fullscreen visualizer"
                    data-testid="visualizer-fullscreen"
                    icon
                    onClick={enterFullscreen}
                    size="mini"
                  >
                    <Icon name="expand" />
                  </Button>
                }
              />
            </>
          ) : mode === 'inline' ? null : (
            <>
              {mode === 'fullwindow' ? (
                <Popup
                  content="Enter true fullscreen."
                  trigger={
                    <Button
                      aria-label="Enter fullscreen visualizer"
                      data-testid="visualizer-fullscreen"
                      icon
                      onClick={enterFullscreen}
                      size="mini"
                    >
                      <Icon name="expand" />
                    </Button>
                  }
                />
              ) : null}
              <Popup
                content="Return visualizer to the player bar."
                trigger={
                  <Button
                    aria-label="Collapse visualizer"
                    data-testid="visualizer-collapse"
                    icon
                    onClick={exitFullscreen}
                    size="mini"
                  >
                    <Icon name="compress" />
                  </Button>
                }
              />
            </>
          )}
        </div>
  );
};

export default VisualizerOverlayControls;
