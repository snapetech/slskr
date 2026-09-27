import React from 'react';
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import SongIDEvidenceDetails from './SongIDEvidenceDetails';

describe('SongIDEvidenceDetails', () => {
  it('renders evidence sections and forwards segment and ranked-option actions', () => {
    const handleOptionAction = vi.fn();
    const handleTrackSearchBatch = vi.fn();
    const option = {
      actionLabel: 'Acquire candidate',
      description: 'Search the strongest recording match.',
      mode: 'single',
      optionId: 'option-1',
      overallScore: 0.9,
      qualityScore: 0.8,
      readinessScore: 0.7,
      scope: 'track',
      title: 'Candidate recording',
    };
    const run = {
      evidence: ['AcoustID match'],
      forensicMatrix: {
        confidenceScore: 'high',
        identityLane: { score: 0.9, summary: 'Recording evidence agrees.' },
        identityScore: 0.9,
        notes: ['Stable across clips'],
        topEvidenceFor: ['AcoustID match'],
      },
      identityAssessment: {
        confidence: 0.95,
        summary: 'The recording match is strong.',
        verdict: 'matched',
      },
      scorecard: { clipCount: 1 },
      segments: [{
        confidence: 0.8,
        label: 'Opening section',
        options: [],
        query: 'artist - title opening',
        segmentId: 'segment-1',
      }],
      syntheticAssessment: {
        summary: 'No synthetic signal was detected.',
        verdict: 'unlikely',
      },
    };

    render(
      <SongIDEvidenceDetails
        actionLoading={false}
        copyForensicMatrix={vi.fn()}
        copyLoading={false}
        handleOptionAction={handleOptionAction}
        handleTrackSearchBatch={handleTrackSearchBatch}
        options={[option]}
        run={run}
      />,
    );

    expect(screen.getByText(/The recording match is strong\./)).toBeInTheDocument();
    expect(screen.getByText('No synthetic signal was detected.')).toBeInTheDocument();
    expect(screen.getAllByText('AcoustID match')).toHaveLength(2);
    expect(screen.getByText('Ranked Download Options (1)')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Search Segment' }));
    expect(handleTrackSearchBatch).toHaveBeenCalledWith(['artist - title opening']);

    fireEvent.click(screen.getByRole('button', { name: 'Acquire candidate' }));
    expect(handleOptionAction).toHaveBeenCalledWith(option);
  });
});
