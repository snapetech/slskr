import React from 'react';
import {
  Button,
  Header,
  Label,
  List,
  Popup,
} from 'semantic-ui-react';
import {
  detailStyle,
  detailSummaryStyle,
  formatPercent,
  getIdentityVerdictColor,
  getSyntheticVerdictColor,
  hasScorecardSignal,
} from './songIdPanelHelpers';

const SongIDEvidenceDetails = ({
  actionLoading,
  copyForensicMatrix,
  copyLoading,
  handleOptionAction,
  handleTrackSearchBatch,
  options,
  run,
}) => {
  const renderOptionScores = (item) => (
    <div style={{ marginTop: '0.35em' }}>
      <Label color="blue" size="tiny">
        Overall {Math.round((item.overallScore || 0) * 100)}
      </Label>
      <Label size="tiny">Quality {Math.round((item.qualityScore || 0) * 100)}</Label>
      <Label size="tiny">Byzantine {Math.round((item.byzantineScore || 0) * 100)}</Label>
      <Label size="tiny">Ready {Math.round((item.readinessScore || 0) * 100)}</Label>
    </div>
  );

  const renderSyntheticPopupContent = () => {
    if (!run?.forensicMatrix) {
      return run?.syntheticAssessment?.summary || 'No synthetic detail available.';
    }

    const matrix = run.forensicMatrix;
    return (
      <div style={{ maxWidth: 360 }}>
        <div>
          Synthetic score {matrix.syntheticScore || 0} · confidence{' '}
          {matrix.confidenceScore || 0} · family {matrix.familyLabel || 'none'}
        </div>
        {Array.isArray(matrix.topEvidenceFor) && matrix.topEvidenceFor.length > 0 ? (
          <div style={{ marginTop: '0.5em' }}>
            For: {matrix.topEvidenceFor.slice(0, 3).join(' | ')}
          </div>
        ) : null}
        {Array.isArray(matrix.topEvidenceAgainst) &&
        matrix.topEvidenceAgainst.length > 0 ? (
          <div style={{ marginTop: '0.5em' }}>
            Against: {matrix.topEvidenceAgainst.slice(0, 3).join(' | ')}
          </div>
        ) : null}
        {Array.isArray(matrix.notes) && matrix.notes.length > 0 ? (
          <div style={{ marginTop: '0.5em' }}>
            Notes: {matrix.notes.slice(0, 4).join(' | ')}
          </div>
        ) : null}
      </div>
    );
  };

  const renderLane = (label, lane) => {
    if (!lane) {
      return null;
    }

    const metricSummary = Object.entries(lane.metrics || {})
      .slice(0, 5)
      .map(([key, value]) => `${key}: ${value}`)
      .join(' | ');

    return (
      <Popup
        key={label}
        content={
          <div style={{ maxWidth: 360 }}>
            <div>{lane.summary || 'No lane summary.'}</div>
            {metricSummary ? (
              <div style={{ marginTop: '0.5em' }}>{metricSummary}</div>
            ) : null}
          </div>
        }
        position="top left"
        trigger={
          <Label size="tiny">
            {label} {Math.round((lane.score || 0) * 100)}
          </Label>
        }
      />
    );
  };

  const identityVerdict = run?.identityAssessment?.verdict || run?.assessment?.verdict;
  const identityConfidence = (run?.identityAssessment?.confidence ?? run?.assessment?.confidence) || 0;
  const identitySummary = run?.identityAssessment?.summary || run?.assessment?.summary;
  const showIdentity = Boolean(identitySummary || identityConfidence > 0 || (identityVerdict && identityVerdict !== 'unclassified'));
  const showSynthetic = Boolean(
    run?.syntheticAssessment?.summary ||
    (run?.syntheticAssessment?.verdict && run.syntheticAssessment.verdict !== 'insufficient_evidence') ||
    run?.forensicMatrix?.familyLabel ||
    hasScorecardSignal(run?.scorecard),
  );

  return (
    <>
          {showIdentity ? (
            <>
              <Header as="h5">Identity</Header>
              <p>
                <strong>Verdict:</strong>{' '}
                <Label
                  color={getIdentityVerdictColor(identityVerdict)}
                  size="tiny"
                >
                  {identityVerdict || 'unclassified'}
                </Label>
                <br />
                <strong>Confidence:</strong> {formatPercent(identityConfidence)}
                <br />
                <strong>Reasoning:</strong> {identitySummary || 'None'}
              </p>
            </>
          ) : null}

          {showSynthetic ? (
            <>
              <Header as="h5">Synthetic Signals</Header>
              <div style={{ marginBottom: '1em' }}>
                <Popup
                  content={renderSyntheticPopupContent()}
                  position="top left"
                  trigger={
                    <Label
                      color={getSyntheticVerdictColor(
                        run.syntheticAssessment?.verdict,
                      )}
                      size="tiny"
                    >
                      Synthetic {run.syntheticAssessment?.verdict || 'unknown'}
                    </Label>
                  }
                />
                <Popup
                  content="Confidence applies to the synthetic-likelihood readout only. It does not control download decisions when identity is strong."
                  position="top left"
                  trigger={
                    <Label size="tiny">
                      Confidence{' '}
                      {run.forensicMatrix?.confidenceScore ||
                        run.syntheticAssessment?.confidence ||
                        'low'}
                    </Label>
                  }
                />
                {run.forensicMatrix?.familyLabel ? (
                  <Popup
                    content={`Known-family score ${run.forensicMatrix.knownFamilyScore || 0}. This is a hint lane, not a download-control signal.`}
                    position="top left"
                    trigger={
                      <Label size="tiny">
                        Family {run.forensicMatrix.familyLabel}
                      </Label>
                    }
                  />
                ) : null}
                {run.forensicMatrix?.qualityClass ? (
                  <Label size="tiny">
                    Quality {run.forensicMatrix.qualityClass}
                  </Label>
                ) : null}
                {run.forensicMatrix?.perturbationStability !== undefined ? (
                  <Label size="tiny">
                    Stability{' '}
                    {Math.round(
                      (run.forensicMatrix.perturbationStability || 0) * 100,
                    )}
                  </Label>
                ) : null}
              </div>
              {run.syntheticAssessment?.summary ? (
                <p style={{ marginTop: 0 }}>
                  {run.syntheticAssessment.summary}
                </p>
              ) : null}
              <div style={{ marginBottom: '1em' }}>
                {renderLane('Provenance', run.forensicMatrix?.provenanceLane)}
                {renderLane('Spectral', run.forensicMatrix?.spectralArtifactLane)}
                {renderLane('Descriptor', run.forensicMatrix?.descriptorPriorsLane)}
                {renderLane('Lyrics', run.forensicMatrix?.lyricsSpeechLane)}
                {renderLane('Structure', run.forensicMatrix?.structuralLane)}
                {renderLane('Family', run.forensicMatrix?.generatorFamilyLane)}
              </div>
            </>
          ) : null}

          {hasScorecardSignal(run.scorecard) ? (
            <details style={detailStyle}>
              <summary style={detailSummaryStyle}>Scorecard</summary>
              <div style={{ marginBottom: '1em' }}>
                <Label size="tiny">Audio {run.scorecard.analysisAudioSource || 'none'}</Label>
                <Label size="tiny">Clips {run.scorecard.clipCount || 0}</Label>
                <Label size="tiny">AcoustID {run.scorecard.acoustIdHitCount || 0}</Label>
                <Label size="tiny">AcoustID Raw {run.scorecard.rawAcoustIdHitCount || 0}</Label>
                <Label size="tiny">SongRec {run.scorecard.songRecHitCount || 0}</Label>
                <Label size="tiny">SongRec Distinct {run.scorecard.songRecDistinctMatchCount || 0}</Label>
                <Label size="tiny">Transcript {run.scorecard.transcriptCount || 0}</Label>
                <Label size="tiny">OCR {run.scorecard.ocrCount || 0}</Label>
                <Label size="tiny">Comments {run.scorecard.commentFindingCount || 0}</Label>
                <Label size="tiny">Timestamps {run.scorecard.timestampHintCount || 0}</Label>
                <Label size="tiny">Chapters {run.scorecard.chapterHintCount || 0}</Label>
                <Label size="tiny">Playlist Requests {run.scorecard.playlistRequestCount || 0}</Label>
                <Label size="tiny">AI Mentions {run.scorecard.aiCommentMentionCount || 0}</Label>
                <Label size="tiny">Panako {run.scorecard.panakoHitCount || 0}</Label>
                <Label size="tiny">Audfprint {run.scorecard.audfprintHitCount || 0}</Label>
                <Label size="tiny">Corpus {run.scorecard.corpusMatchCount || 0}</Label>
                <Label size="tiny">
                  Provenance {run.scorecard.provenanceSignalCount || 0}
                </Label>
                <Label size="tiny">
                  AI Heuristics {run.scorecard.aiArtifactClipCount || 0}
                </Label>
              </div>
              {Array.isArray(run.scorecard.embeddedMetadataKeys) &&
              run.scorecard.embeddedMetadataKeys.length > 0 ? (
                <div style={{ marginBottom: '1em' }}>
                  <strong>Metadata Keys:</strong>{' '}
                  {run.scorecard.embeddedMetadataKeys.join(', ')}
                </div>
              ) : null}
              {Array.isArray(run.scorecard.provenanceSignals) &&
              run.scorecard.provenanceSignals.length > 0 ? (
                <div style={{ marginBottom: '1em' }}>
                  <strong>Provenance Signals:</strong>{' '}
                  {run.scorecard.provenanceSignals.join(', ')}
                </div>
              ) : null}
            </details>
          ) : null}

          {run.fullSourceFingerprint ? (
            <details style={detailStyle}>
              <summary style={detailSummaryStyle}>Full-Source Fingerprint</summary>
              <p>
                <strong>Duration:</strong>{' '}
                {Math.round(run.fullSourceFingerprint.durationSeconds || 0)}s
                <br />
                <strong>Fingerprint Length:</strong>{' '}
                {run.fullSourceFingerprint.fingerprintLength || 0}
                <br />
                <strong>Path:</strong> {run.fullSourceFingerprint.path}
              </p>
            </details>
          ) : null}

          {run.provenance && run.provenance.signalCount > 0 ? (
            <>
              <Header as="h5">Provenance</Header>
              <div style={{ marginBottom: '0.75em' }}>
                <Label size="tiny">
                  Tool {run.provenance.toolAvailable ? 'available' : 'unavailable'}
                </Label>
                <Label size="tiny">
                  Manifest {run.provenance.manifestHint ? 'hinted' : 'none'}
                </Label>
                <Label size="tiny">
                  Validation {run.provenance.validationState || 'unknown'}
                </Label>
                {run.provenance.verified ? (
                  <Label color="green" size="tiny">
                    verified
                  </Label>
                ) : null}
              </div>
              <List bulleted>
                {(run.provenance.signals || []).map((signal) => (
                  <List.Item key={signal}>{signal}</List.Item>
                ))}
              </List>
            </>
          ) : null}

          {run.aiHeuristics ? (
            <details style={detailStyle}>
              <summary style={detailSummaryStyle}>AI Audio Heuristics</summary>
              <div style={{ marginBottom: '1em' }}>
                <Label size="tiny">
                  Score {Math.round((run.aiHeuristics.artifactScore || 0) * 100)}
                </Label>
                <Label size="tiny">{run.aiHeuristics.artifactLabel || 'unknown'}</Label>
                <Label size="tiny">
                  Peaks {run.aiHeuristics.peakCount || 0}
                </Label>
                <Label size="tiny">
                  Periodicity{' '}
                  {Math.round((run.aiHeuristics.periodicityStrength || 0) * 100)}
                </Label>
                <Label size="tiny">
                  Centroid {Math.round(run.aiHeuristics.spectralCentroid || 0)}
                </Label>
                <Label size="tiny">
                  Flux {Math.round((run.aiHeuristics.spectralFlux || 0) * 1000)}
                </Label>
                <Label size="tiny">
                  Pitch {Math.round((run.aiHeuristics.pitchSalience || 0) * 100)}
                </Label>
              </div>
            </details>
          ) : null}

          {Array.isArray(run.perturbations) && run.perturbations.length > 0 ? (
            <details style={detailStyle}>
              <summary style={detailSummaryStyle}>
                Perturbation Stability ({run.perturbations.length})
              </summary>
              <List divided relaxed>
                {run.perturbations.map((item) => (
                  <List.Item key={item.perturbationId}>
                    <List.Content>
                      <List.Header>{item.label}</List.Header>
                      <List.Description>
                        Delta {Math.round((item.baselineDelta || 0) * 100)}
                      </List.Description>
                      {item.heuristics ? (
                        <List.Description style={{ marginTop: '0.35em' }}>
                          Score {Math.round((item.heuristics.artifactScore || 0) * 100)} ·
                          {` `}Label {item.heuristics.artifactLabel || 'unknown'}
                        </List.Description>
                      ) : null}
                    </List.Content>
                  </List.Item>
                ))}
              </List>
            </details>
          ) : null}

          {Array.isArray(run.corpusMatches) && run.corpusMatches.length > 0 ? (
            <>
              <Header as="h5">Corpus Matches</Header>
              <List divided relaxed>
                {run.corpusMatches.map((match) => (
                  <List.Item key={match.matchId}>
                    <List.Content>
                      <List.Header>{match.label || match.source}</List.Header>
                      <List.Description>
                        Similarity {Math.round((match.similarityScore || 0) * 100)}
                      </List.Description>
                      {match.artist || match.title ? (
                        <List.Description style={{ marginTop: '0.35em' }}>
                          {[match.artist, match.title].filter(Boolean).join(' - ')}
                        </List.Description>
                      ) : null}
                      {match.recordingId ? (
                        <List.Description style={{ marginTop: '0.35em' }}>
                          Recording: {match.recordingId}
                        </List.Description>
                      ) : null}
                    </List.Content>
                  </List.Item>
                ))}
              </List>
            </>
          ) : null}

          {Array.isArray(run.stems) && run.stems.length > 0 ? (
            <>
              <Header as="h5">Demucs Stems</Header>
              <List bulleted>
                {run.stems.map((stem) => (
                  <List.Item key={stem.artifactId}>
                    {stem.label}: {stem.path}
                  </List.Item>
                ))}
              </List>
            </>
          ) : null}

          {Array.isArray(run.evidence) && run.evidence.length > 0 ? (
            <details style={detailStyle}>
              <summary style={detailSummaryStyle}>
                Evidence ({run.evidence.length})
              </summary>
              <List bulleted>
                {run.evidence.map((item) => (
                  <List.Item key={item}>{item}</List.Item>
                ))}
              </List>
            </details>
          ) : null}

          {Array.isArray(run.segments) && run.segments.length > 0 ? (
            <>
              <Header as="h5">Segment Decomposition</Header>
              <List divided relaxed>
                {run.segments.map((segment) => (
                  <List.Item key={segment.segmentId}>
                    <List.Content>
                      <List.Header>
                        {segment.label}{' '}
                        <Label size="tiny">
                          {Math.round((segment.confidence || 0) * 100)}%
                        </Label>
                      </List.Header>
                      <List.Description>
                        {segment.decompositionLabel || segment.sourceLabel}
                      </List.Description>
                      <List.Description style={{ marginTop: '0.35em' }}>
                        Search: <code>{segment.query}</code>
                      </List.Description>
                      {Array.isArray(segment.candidates) &&
                      segment.candidates.length > 0 ? (
                        <List.Description style={{ marginTop: '0.5em' }}>
                          {segment.candidates.map((candidate) => (
                            <Label key={candidate.candidateId} size="tiny">
                              {candidate.artist} - {candidate.title}
                            </Label>
                          ))}
                        </List.Description>
                      ) : null}
                      <div style={{ marginTop: '0.5em' }}>
                        {Array.isArray(segment.options) &&
                        segment.options.length > 0 ? (
                          <Popup
                            content="Run the best segment-level SongID action for this decomposed portion of the source."
                            position="top center"
                            trigger={
                              <Button
                                disabled={actionLoading}
                                onClick={() => handleOptionAction(segment.options[0])}
                                size="mini"
                              >
                                {segment.options[0].actionLabel}
                              </Button>
                            }
                          />
                        ) : (
                          <Popup
                            content="Search this decomposed segment directly when SongID found a plausible chapter or timestamp clue."
                            position="top center"
                            trigger={
                              <Button
                                disabled={actionLoading}
                                onClick={() =>
                                  handleTrackSearchBatch([segment.query])
                                }
                                size="mini"
                              >
                                Search Segment
                              </Button>
                            }
                          />
                        )}
                      </div>
                    </List.Content>
                  </List.Item>
                ))}
              </List>
            </>
          ) : null}

          {options.length > 0 ? (
            <details style={detailStyle}>
              <summary style={detailSummaryStyle}>
                Ranked Download Options ({options.length})
              </summary>
              <List divided relaxed>
                {options.map((option) => (
                  <List.Item key={option.optionId}>
                    <List.Content floated="right">
                      <Popup
                        content="Run this SongID acquisition option using the scored track, album, or discography path shown here."
                        position="top center"
                        trigger={
                          <Button
                            disabled={actionLoading}
                            onClick={() => handleOptionAction(option)}
                            size="small"
                          >
                            {option.actionLabel}
                          </Button>
                        }
                      />
                    </List.Content>
                    <List.Content>
                      <List.Header>
                        {option.title}{' '}
                        <Label size="tiny">{option.scope}</Label>
                        <Label size="tiny">{option.mode}</Label>
                        {option.duplicateCount > 1 ? (
                          <Label color="grey" size="tiny">
                            {option.duplicateCount} matches
                          </Label>
                        ) : null}
                      </List.Header>
                      <List.Description>{option.description}</List.Description>
                      {option.searchText ? (
                        <List.Description style={{ marginTop: '0.35em' }}>
                          Search: <code>{option.searchText}</code>
                        </List.Description>
                      ) : null}
                      {Array.isArray(option.searchTexts) &&
                      option.searchTexts.length > 0 ? (
                        <List.Description style={{ marginTop: '0.35em' }}>
                          Searches: <code>{option.searchTexts.join(' | ')}</code>
                        </List.Description>
                      ) : null}
                      {option.profile ? (
                        <List.Description style={{ marginTop: '0.35em' }}>
                          Profile: {option.profile}
                        </List.Description>
                      ) : null}
                      {renderOptionScores(option)}
                    </List.Content>
                  </List.Item>
                ))}
              </List>
            </details>
          ) : null}

          {Array.isArray(run.clips) && run.clips.length > 0 ? (
            <details style={detailStyle}>
              <summary style={detailSummaryStyle}>
                Clip Findings ({run.clips.length})
              </summary>
              <List divided relaxed>
                {run.clips.map((clip) => (
                  <List.Item key={clip.clipId}>
                    <List.Content>
                      <List.Header>
                        {clip.profile} @ {clip.startSeconds}s
                      </List.Header>
                      <List.Description>
                        {clip.durationSeconds}s clip
                      </List.Description>
                      {clip.acoustId ? (
                        <List.Description style={{ marginTop: '0.35em' }}>
                          AcoustID: {clip.acoustId.artist} - {clip.acoustId.title}
                        </List.Description>
                      ) : null}
                      {clip.songRec ? (
                        <List.Description style={{ marginTop: '0.35em' }}>
                          SongRec: {clip.songRec.artist} - {clip.songRec.title}
                        </List.Description>
                      ) : null}
                      {clip.panako ? (
                        <List.Description style={{ marginTop: '0.35em' }}>
                          Panako: {clip.panako.title || clip.panako.sourcePath}
                        </List.Description>
                      ) : null}
                      {clip.audfprint ? (
                        <List.Description style={{ marginTop: '0.35em' }}>
                          Audfprint: {clip.audfprint.title}
                        </List.Description>
                      ) : null}
                      {clip.aiHeuristics ? (
                        <List.Description style={{ marginTop: '0.35em' }}>
                          AI Heuristics: {clip.aiHeuristics.artifactLabel} (
                          {Math.round((clip.aiHeuristics.artifactScore || 0) * 100)})
                        </List.Description>
                      ) : null}
                    </List.Content>
                  </List.Item>
                ))}
              </List>
            </details>
          ) : null}

          {Array.isArray(run.transcripts) && run.transcripts.length > 0 ? (
            <details style={detailStyle}>
              <summary style={detailSummaryStyle}>
                Transcripts ({run.transcripts.length})
              </summary>
              <List divided relaxed>
                {run.transcripts.map((transcript) => (
                  <List.Item key={transcript.transcriptId}>
                    <List.Content>
                      <List.Header>{transcript.source}</List.Header>
                      <List.Description>{transcript.text}</List.Description>
                      <List.Description style={{ marginTop: '0.35em' }}>
                        Segments: {transcript.segmentCount || 0} · Language:{' '}
                        {transcript.language || 'unknown'} · Excerpt:{' '}
                        {transcript.excerptStartSeconds || 0}s /{' '}
                        {transcript.excerptDurationSeconds || 0}s
                      </List.Description>
                      {Array.isArray(transcript.musicBrainzQueries) &&
                      transcript.musicBrainzQueries.length > 0 ? (
                        <List.Description style={{ marginTop: '0.35em' }}>
                          Queries: {transcript.musicBrainzQueries.join(' | ')}
                        </List.Description>
                      ) : null}
                    </List.Content>
                  </List.Item>
                ))}
              </List>
            </details>
          ) : null}

          {Array.isArray(run.ocr) && run.ocr.length > 0 ? (
            <details style={detailStyle}>
              <summary style={detailSummaryStyle}>OCR ({run.ocr.length})</summary>
              <List bulleted>
                {run.ocr.map((item) => (
                  <List.Item key={item.ocrId}>
                    {item.timestampSeconds}s: {item.text}
                  </List.Item>
                ))}
              </List>
            </details>
          ) : null}

          {Array.isArray(run.comments) && run.comments.length > 0 ? (
            <details style={detailStyle}>
              <summary style={detailSummaryStyle}>
                Comments ({run.comments.length})
              </summary>
              <List bulleted>
                {run.comments.map((comment) => (
                  <List.Item key={comment.commentId}>
                    {comment.author ? `${comment.author}: ` : ''}
                    {comment.text}
                    {comment.timestampSeconds !== null &&
                    comment.timestampSeconds !== undefined
                      ? ` (${comment.timestampSeconds}s)`
                      : ''}
                  </List.Item>
                ))}
              </List>
            </details>
          ) : null}

          {Array.isArray(run.chapters) && run.chapters.length > 0 ? (
            <>
              <Header as="h5">Chapters</Header>
              <List bulleted>
                {run.chapters.map((chapter) => (
                  <List.Item key={chapter.chapterId}>
                    {chapter.startSeconds}s: {chapter.title || 'untitled chapter'}
                    {chapter.endSeconds !== null &&
                    chapter.endSeconds !== undefined
                      ? ` -> ${chapter.endSeconds}s`
                      : ''}
                  </List.Item>
                ))}
              </List>
            </>
          ) : null}

          {run.forensicMatrix ? (
            <details style={detailStyle}>
              <summary style={detailSummaryStyle}>Forensic Matrix</summary>
              <div style={{ marginBottom: '1em' }}>
                <Popup
                  content="Copy the complete forensic matrix JSON for this SongID run without changing ranking, downloads, or local files."
                  position="top left"
                  trigger={
                    <Button
                      aria-label="Copy SongID forensic matrix JSON"
                      disabled={copyLoading}
                      icon="copy"
                      loading={copyLoading}
                      onClick={copyForensicMatrix}
                      size="mini"
                    />
                  }
                />
              </div>
              <div style={{ marginBottom: '1em' }}>
                <Label size="tiny">Identity {run.forensicMatrix.identityScore || 0}</Label>
                <Label size="tiny">
                  Synthetic {run.forensicMatrix.syntheticScore || 0}
                </Label>
                <Label size="tiny">
                  Confidence {run.forensicMatrix.confidenceScore || 0}
                </Label>
                <Label size="tiny">
                  Family Score {run.forensicMatrix.knownFamilyScore || 0}
                </Label>
              </div>
              <div style={{ marginBottom: '1em' }}>
                {run.forensicMatrix.identityLane ? (
                  <Popup
                    content={run.forensicMatrix.identityLane.summary}
                    position="top left"
                    trigger={
                      <Label size="tiny">
                        Identity Lane {Math.round((run.forensicMatrix.identityLane.score || 0) * 100)}
                      </Label>
                    }
                  />
                ) : null}
                {run.forensicMatrix.confidenceLane ? (
                  <Popup
                    content={run.forensicMatrix.confidenceLane.summary}
                    position="top left"
                    trigger={
                      <Label size="tiny">
                        Confidence Lane {Math.round((run.forensicMatrix.confidenceLane.score || 0) * 100)}
                      </Label>
                    }
                  />
                ) : null}
                {run.forensicMatrix.spectralArtifactLane ? (
                  <Popup
                    content={run.forensicMatrix.spectralArtifactLane.summary}
                    position="top left"
                    trigger={
                      <Label size="tiny">
                        Spectral {Math.round((run.forensicMatrix.spectralArtifactLane.score || 0) * 100)}
                      </Label>
                    }
                  />
                ) : null}
                {run.forensicMatrix.lyricsSpeechLane ? (
                  <Popup
                    content={run.forensicMatrix.lyricsSpeechLane.summary}
                    position="top left"
                    trigger={
                      <Label size="tiny">
                        Lyrics {Math.round((run.forensicMatrix.lyricsSpeechLane.score || 0) * 100)}
                      </Label>
                    }
                  />
                ) : null}
                {run.forensicMatrix.structuralLane ? (
                  <Popup
                    content={run.forensicMatrix.structuralLane.summary}
                    position="top left"
                    trigger={
                      <Label size="tiny">
                        Structure {Math.round((run.forensicMatrix.structuralLane.score || 0) * 100)}
                      </Label>
                    }
                  />
                ) : null}
                {run.forensicMatrix.provenanceLane ? (
                  <Popup
                    content={run.forensicMatrix.provenanceLane.summary}
                    position="top left"
                    trigger={
                      <Label size="tiny">
                        Provenance {Math.round((run.forensicMatrix.provenanceLane.score || 0) * 100)}
                      </Label>
                    }
                  />
                ) : null}
              </div>
              {Array.isArray(run.forensicMatrix.topEvidenceFor) &&
              run.forensicMatrix.topEvidenceFor.length > 0 ? (
                <div style={{ marginBottom: '0.75em' }}>
                  <strong>For:</strong> {run.forensicMatrix.topEvidenceFor.join(' | ')}
                </div>
              ) : null}
              {Array.isArray(run.forensicMatrix.topEvidenceAgainst) &&
              run.forensicMatrix.topEvidenceAgainst.length > 0 ? (
                <div style={{ marginBottom: '0.75em' }}>
                  <strong>Against:</strong>{' '}
                  {run.forensicMatrix.topEvidenceAgainst.join(' | ')}
                </div>
              ) : null}
              {Array.isArray(run.forensicMatrix.notes) &&
              run.forensicMatrix.notes.length > 0 ? (
                <div style={{ marginBottom: '0.75em' }}>
                  <strong>Notes:</strong> {run.forensicMatrix.notes.join(' | ')}
                </div>
              ) : null}
            </details>
          ) : null}

    </>
  );
};

export default SongIDEvidenceDetails;
