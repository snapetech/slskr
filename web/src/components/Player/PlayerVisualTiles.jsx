import React, { lazy, Suspense, useRef, useState } from 'react';
import { getLocalStorageItem } from '../../lib/storage';
import SpectrumAnalyzer from './SpectrumAnalyzer';
import { Button, Icon, Popup } from 'semantic-ui-react';

const Visualizer = lazy(() => import('./Visualizer'));
const visualizerEngineStorageKey = 'slskr.player.visualizerEngine';

export const readStoredVisualizerEngineTileMode = () => {
  const engine = getLocalStorageItem(visualizerEngineStorageKey);
  if (engine === 'native') return 'rustymilk-webgl2';
  return ['rustymilk-webgl2', 'rustymilk-webgpu'].includes(engine)
    ? engine
    : 'rustymilk-webgl2';
};

export const PlayerVisualTile = ({
  audioElement,
  runtimeProfile,
  current,
  mode,
  onModeChange,
  onTileModeChange,
  tileMode,
}) => {
  const nativeProfile = runtimeProfile === 'native';
  const tileModes = ['art', 'rustymilk-webgl2', 'rustymilk-webgpu', 'spectrum', 'scope'];
  const visualizerTileModes = ['rustymilk-webgl2', 'rustymilk-webgpu'];
  const tileModeLabels = {
    art: 'album art',
    'rustymilk-webgl2': nativeProfile ? 'MilkDrop3 WebGL2' : 'RustyMilk WebGL2',
    'rustymilk-webgpu': nativeProfile ? 'MilkDrop3 WebGPU' : 'RustyMilk WebGPU',
    butterchurn: 'Butterchurn',
    scope: 'signal scope',
    spectrum: 'spectrum bars',
  };
  const tileModeIcons = {
    'rustymilk-webgl2': 'microchip',
    'rustymilk-webgpu': 'bolt',
    butterchurn: 'magic',
    scope: 'signal',
    spectrum: 'chart bar',
  };
  const tileRef = useRef(null);
  const [visualizerRevision, setVisualizerRevision] = useState(0);
  const title = current?.title || current?.fileName || 'slskr';
  const artist = current?.artist || '';
  const initials = (artist || title)
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0])
    .join('')
    .toUpperCase() || 'N';
  const artworkUrl = current?.artworkUrl;
  const normalizedTileMode = tileModes.includes(tileMode) ? tileMode : 'art';
  const showingVisualizer = visualizerTileModes.includes(normalizedTileMode);
  const showingAnalyzer = ['spectrum', 'scope'].includes(normalizedTileMode);
  const nextTileMode = nativeProfile && normalizedTileMode === 'art'
    ? 'butterchurn'
    : tileModes[(tileModes.indexOf(normalizedTileMode) + 1) % tileModes.length];
  const visualizerDisplayMode = mode === 'off' ? 'inline' : mode;
  const setTileMode = (nextMode) => {
    const effectiveMode = nextMode === 'butterchurn' ? 'rustymilk-webgl2' : nextMode;
    onTileModeChange(effectiveMode);
    if (visualizerTileModes.includes(effectiveMode) && mode === 'off') {
      onModeChange('inline');
    }
    if (visualizerTileModes.includes(effectiveMode)) {
      setVisualizerRevision((revision) => revision + 1);
    }
  };
  const switchTileMode = (event, nextMode) => {
    event.stopPropagation();
    setTileMode(nextMode);
  };
  const showVisualizerWindow = (event) => {
    event.stopPropagation();
    if (!showingVisualizer) {
      onTileModeChange(readStoredVisualizerEngineTileMode());
    }
    onModeChange('fullwindow');
  };
  const showVisualizerFullscreen = async (event) => {
    event.stopPropagation();
    if (!showingVisualizer) {
      onTileModeChange(readStoredVisualizerEngineTileMode());
    }
    if (tileRef.current?.requestFullscreen) {
      try {
        await tileRef.current.requestFullscreen();
      } catch {
        // Keep the visualizer in fullscreen layout even if the browser denies the request.
      }
    }
    onModeChange('fullscreen');
  };
  const handleTileActivate = () => setTileMode(nextTileMode);

  return (
    <div className="player-visual-tile">
      <Popup
        content={
          `Show ${tileModeLabels[nextTileMode]} in this square.`
        }
        trigger={
          <div
            aria-label={
              `Show ${tileModeLabels[nextTileMode]} in player visual tile`
            }
            role="button"
            className="player-visual-stage"
            data-testid="player-visual-tile"
            onKeyDown={(event) => {
              if (event.key === 'Enter' || event.key === ' ') {
                event.preventDefault();
                handleTileActivate();
              }
            }}
            onClick={handleTileActivate}
            ref={tileRef}
            tabIndex={0}
          >
            {showingVisualizer ? (
              <Suspense fallback={<div className="player-visual-loading">Loading visualizer...</div>}>
                <Visualizer
                  audioElement={audioElement}
                  compactControls
                  engineOverride={normalizedTileMode}
                  key={`${normalizedTileMode}-${visualizerRevision}`}
                  mode={visualizerDisplayMode}
                  onEngineChange={onTileModeChange}
                  onModeChange={onModeChange}
                />
              </Suspense>
            ) : showingAnalyzer ? (
              <SpectrumAnalyzer
                audioElement={audioElement}
                className="player-visualizer-fallback"
                mode={normalizedTileMode}
              />
            ) : (
              <span className="player-album-art" data-testid="player-album-art">
                {artworkUrl ? (
                  <img alt="" src={artworkUrl} />
                ) : (
                  <>
                    <span className="player-album-art-glow" />
                    <span className="player-album-art-mark">{initials}</span>
                  </>
                )}
              </span>
            )}
            <span className="player-visual-affordance">
              <Icon name={showingVisualizer ? 'magic' : (showingAnalyzer ? 'chart bar' : 'image outline')} />
            </span>
          </div>
        }
      />
      <div className="player-visual-tile-controls" onClick={(event) => event.stopPropagation()}>
        {(nativeProfile
          ? ['spectrum', 'scope', 'butterchurn', 'rustymilk-webgl2', 'rustymilk-webgpu']
          : ['spectrum', 'scope', 'rustymilk-webgl2', 'rustymilk-webgpu']
        ).map((option) => (
          <Popup
            content={`Show ${tileModeLabels[option]}.`}
            key={option}
            trigger={
              <Button
                aria-label={`Show ${tileModeLabels[option]}`}
                active={normalizedTileMode === option}
                data-testid={`player-visual-tile-mode-${option}`}
                icon
                onClick={(event) => switchTileMode(event, option)}
                size="mini"
              >
                <Icon name={tileModeIcons[option]} />
              </Button>
            }
          />
        ))}
        <Popup
          content="Maximize the visualizer to the browser window."
          trigger={
            <Button
              aria-label="Maximize visualizer to browser window"
              data-testid="player-visual-tile-fullwindow"
              icon
              onClick={showVisualizerWindow}
              size="mini"
            >
              <Icon name="expand arrows alternate" />
            </Button>
          }
        />
        <Popup
          content="Maximize the visualizer to fullscreen."
          trigger={
            <Button
              aria-label="Maximize visualizer to fullscreen"
              data-testid="player-visual-tile-fullscreen"
              icon
              onClick={showVisualizerFullscreen}
              size="mini"
            >
              <Icon name="expand" />
            </Button>
          }
        />
      </div>
    </div>
  );
};

export const PlayerAnalyzerTile = ({ audioElement, mode, onModeChange }) => {
  const nextMode = mode === 'spectrum' ? 'scope' : 'spectrum';
  const label = mode === 'spectrum' ? 'Spectrum bars' : 'Signal scope';

  return (
    <Popup
      content={
        mode === 'spectrum'
          ? 'Show signal scope in this box.'
          : 'Show spectrum bars in this box.'
      }
      trigger={
        <div
          aria-label={`Show ${nextMode === 'spectrum' ? 'spectrum bars' : 'signal scope'}`}
          className="player-analyzer-tile"
          data-testid="player-analyzer-tile"
          onClick={() => onModeChange(nextMode)}
          onKeyDown={(event) => {
            if (event.key === 'Enter' || event.key === ' ') {
              event.preventDefault();
              onModeChange(nextMode);
            }
          }}
          role="button"
          tabIndex={0}
        >
          <div className="player-analyzer-label">{label}</div>
          <SpectrumAnalyzer
            audioElement={mode === 'off' ? null : audioElement}
            className="player-spectrum-switchable"
            mode={mode}
          />
          <span className="player-analyzer-affordance">
            <Icon name={mode === 'spectrum' ? 'signal' : 'chart bar'} />
          </span>
        </div>
      }
    />
  );
};

