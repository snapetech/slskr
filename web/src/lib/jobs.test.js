// <copyright file="jobs.test.js" company="slskr Team">
// Copyright (c) slskr Team. All rights reserved.
// </copyright>

import api from './api';
import * as jobs from './jobs';

// Mock the api module
vi.mock('./api', () => ({
  __esModule: true,
  default: {
    get: vi.fn(),
    post: vi.fn(),
  },
}));

describe('jobs', () => {
  beforeEach(() => {
    // Clear both call history and rejected/resolved implementations. The
    // endpoint-compatibility cases deliberately change the same mock from a
    // 404 to a network failure; retaining implementations makes the outcome
    // depend on test-file scheduling in CI.
    vi.resetAllMocks();
  });

  describe('getJobs', () => {
    it('calls API with no parameters when options are empty', async () => {
      api.get.mockResolvedValue({ data: { jobs: [], total: 0 } });

      await jobs.getJobs();

      expect(api.get).toHaveBeenCalledWith('/jobs');
    });

    it('calls API with type filter', async () => {
      api.get.mockResolvedValue({ data: { jobs: [], total: 0 } });

      await jobs.getJobs({ type: 'discography' });

      expect(api.get).toHaveBeenCalledWith('/jobs?type=discography');
    });

    it('calls API with status filter', async () => {
      api.get.mockResolvedValue({ data: { jobs: [], total: 0 } });

      await jobs.getJobs({ status: 'running' });

      expect(api.get).toHaveBeenCalledWith('/jobs?status=running');
    });

    it('calls API with multiple filters', async () => {
      api.get.mockResolvedValue({ data: { jobs: [], total: 0 } });

      await jobs.getJobs({
        limit: 20,
        offset: 10,
        sortBy: 'created_at',
        sortOrder: 'desc',
        status: 'running',
        type: 'discography',
      });

      expect(api.get).toHaveBeenCalledWith(
        '/jobs?type=discography&status=running&limit=20&offset=10&sortBy=created_at&sortOrder=desc',
      );
    });

    it('returns jobs data from API response', async () => {
      const mockJobs = [
        {
          created_at: '2026-01-26T10:00:00Z',
          id: 'job-1',
          status: 'running',
          type: 'discography',
        },
      ];
      api.get.mockResolvedValue({
        data: { jobs: mockJobs, limit: 20, offset: 0, total: 1 },
      });

      const result = await jobs.getJobs();

      expect(result).toEqual({
        jobs: mockJobs,
        limit: 20,
        offset: 0,
        total: 1,
      });
    });

    it('handles undefined optional parameters', async () => {
      api.get.mockResolvedValue({ data: { jobs: [], total: 0 } });

      await jobs.getJobs({
        limit: undefined,
        status: undefined,
        type: 'discography',
      });

      expect(api.get).toHaveBeenCalledWith('/jobs?type=discography');
    });
  });

  describe('getJob', () => {
    it('calls API with encoded job ID', async () => {
      const mockJob = {
        id: 'job-123',
        status: 'completed',
        type: 'discography',
      };
      api.get.mockResolvedValue({ data: mockJob });

      await jobs.getJob('job-123');

      expect(api.get).toHaveBeenCalledWith('/jobs/job-123');
    });

    it('encodes special characters in job ID', async () => {
      api.get.mockResolvedValue({ data: {} });

      await jobs.getJob('job/with/slashes');

      expect(api.get).toHaveBeenCalledWith('/jobs/job%2Fwith%2Fslashes');
    });

    it('returns job data from API response', async () => {
      const mockJob = {
        id: 'job-1',
        progress: {
          releases_done: 5,
          releases_failed: 0,
          releases_total: 10,
        },
        status: 'running',
        type: 'discography',
      };
      api.get.mockResolvedValue({ data: mockJob });

      const result = await jobs.getJob('job-1');

      expect(result).toEqual(mockJob);
    });
  });

  describe('getActiveSwarmJobs', () => {
    it('returns empty array when API returns empty jobs array', async () => {
      api.get.mockResolvedValue({ data: { jobs: [] } });

      const result = await jobs.getActiveSwarmJobs();

      expect(result).toEqual([]);
      expect(api.get).toHaveBeenCalledWith('/multisource/jobs');
    });

    it('returns jobs array from API response', async () => {
      const mockJobs = [
        {
          activeSources: 3,
          downloadedBytes: 1_024,
          filename: '/path/to/file.mp3',
          jobId: 'swarm-1',
          progressPercent: 25,
          totalBytes: 4_096,
        },
      ];
      api.get.mockResolvedValue({ data: { jobs: mockJobs } });

      const result = await jobs.getActiveSwarmJobs();

      expect(result).toEqual(mockJobs);
    });

    it('normalizes native job metrics and legacy identifiers', async () => {
      api.get.mockResolvedValue({
        data: {
          jobs: [
            {
              completedChunks: 2,
              id: 'legacy-1',
              progress: 50,
              size: 8_192,
              sources: ['alice', 'bob'],
              status: 'running',
              bytesTransferred: 2_048,
            },
            {
              activeWorkers: 3,
              bytesDownloaded: 4_096,
              completedChunks: 4,
              jobId: 'native-1',
              totalChunks: 8,
              state: 'InProgress',
            },
          ],
        },
      });

      await expect(jobs.getActiveSwarmJobs()).resolves.toEqual([
        expect.objectContaining({
          activeSources: 2,
          downloadedBytes: 2_048,
          jobId: 'legacy-1',
          progressPercent: 50,
          totalBytes: 8_192,
        }),
        expect.objectContaining({
          activeSources: 3,
          downloadedBytes: 4_096,
          jobId: 'native-1',
          progressPercent: 50,
          totalBytes: 0,
        }),
      ]);
    });

    it('drops malformed swarm records before rendering', async () => {
      api.get.mockResolvedValue({
        data: { jobs: [null, 'invalid', {}, { id: 'valid-1' }] },
      });

      await expect(jobs.getActiveSwarmJobs()).resolves.toEqual([
        expect.objectContaining({ jobId: 'valid-1' }),
      ]);
    });

    it('rejects an envelope without a jobs list', async () => {
      api.get.mockResolvedValue({ data: { jobs: null } });

      await expect(jobs.getActiveSwarmJobs()).rejects.toThrow(
        'Jobs API returned an invalid swarm job list response',
      );
    });

    it('rejects an envelope without a jobs property', async () => {
      api.get.mockResolvedValue({ data: {} });

      await expect(jobs.getActiveSwarmJobs()).rejects.toThrow(
        'Jobs API returned an invalid swarm job list response',
      );
    });

    it('uses an empty list only when the optional endpoint is absent', async () => {
      const error = new Error('Not found');
      error.response = { status: 404 };
      api.get.mockRejectedValue(error);

      await expect(jobs.getActiveSwarmJobs()).resolves.toEqual([]);
    });

    it('propagates network errors instead of fabricating an empty list', async () => {
      api.get.mockRejectedValue(new Error('Network error'));

      await expect(jobs.getActiveSwarmJobs()).rejects.toThrow('Network error');
    });
  });

  describe('createDiscographyJob', () => {
    it('posts artist profile and target directory', async () => {
      api.post.mockResolvedValue({ data: { job_id: 'disc-1', status: 'pending' } });

      const result = await jobs.createDiscographyJob({
        artistId: 'artist-123',
        profile: 'ExtendedDiscography',
        targetDirectory: '/music/Artist',
      });

      expect(api.post).toHaveBeenCalledWith('/jobs/discography', {
        artist_id: 'artist-123',
        profile: 'ExtendedDiscography',
        target_dir: '/music/Artist',
      });
      expect(result).toEqual({ job_id: 'disc-1', status: 'pending' });
    });
  });

  describe('createMbReleaseJob', () => {
    it('posts a single-release job request', async () => {
      api.post.mockResolvedValue({ data: { job_id: 'album-1', status: 'pending' } });

      const result = await jobs.createMbReleaseJob({
        mbReleaseId: 'release-123',
        targetDir: '/music/Artist/Album',
      });

      expect(api.post).toHaveBeenCalledWith('/jobs/mb-release', {
        mb_release_id: 'release-123',
        target_dir: '/music/Artist/Album',
        tracks: 'all',
        constraints: null,
      });
      expect(result).toEqual({ job_id: 'album-1', status: 'pending' });
    });
  });

  describe('getSwarmJobStatus', () => {
    it('calls API with encoded job ID', async () => {
      const mockStatus = {
        jobId: 'swarm-1',
        percentComplete: 50,
        state: 'running',
      };
      api.get.mockResolvedValue({ data: mockStatus });

      await jobs.getSwarmJobStatus('swarm-1');

      expect(api.get).toHaveBeenCalledWith('/multisource/jobs/swarm-1');
    });

    it('returns job status from API response', async () => {
      const mockStatus = {
        activeWorkers: 3,
        chunksPerSecond: 10.5,
        completedChunks: 50,
        jobId: 'swarm-1',
        percentComplete: 50,
        state: 'running',
        totalChunks: 100,
      };
      api.get.mockResolvedValue({ data: mockStatus });

      const result = await jobs.getSwarmJobStatus('swarm-1');

      expect(result).toEqual(mockStatus);
    });

    it('returns null when job is not found (404)', async () => {
      const error = new Error('Not found');
      error.response = { status: 404 };
      api.get.mockRejectedValue(error);

      const result = await jobs.getSwarmJobStatus('nonexistent');

      expect(result).toBeNull();
    });

    it('throws error for non-404 errors', async () => {
      const error = new Error('Server error');
      error.response = { status: 500 };
      api.get.mockRejectedValue(error);

      await expect(jobs.getSwarmJobStatus('swarm-1')).rejects.toThrow(
        'Server error',
      );
    });

    it('rejects malformed successful status responses', async () => {
      api.get.mockResolvedValue({ data: [] });

      await expect(jobs.getSwarmJobStatus('swarm-1')).rejects.toThrow(
        'Jobs API returned an invalid swarm job response',
      );
    });

    it('handles errors without response property', async () => {
      const error = new Error('Network error');
      api.get.mockRejectedValue(error);

      await expect(jobs.getSwarmJobStatus('swarm-1')).rejects.toThrow(
        'Network error',
      );
    });
  });

  it('rejects malformed job envelopes and mutation responses', async () => {
    api.get.mockResolvedValue({ data: [] });
    await expect(jobs.getJobs()).rejects.toThrow(
      'Jobs API returned an invalid job list response',
    );
    await expect(jobs.getJob('job-1')).rejects.toThrow(
      'Jobs API returned an invalid job response',
    );

    api.post.mockResolvedValue({ data: [] });
    await expect(
      jobs.createDiscographyJob({ artistId: 'artist-1' }),
    ).rejects.toThrow('Jobs API returned an invalid discography job response');
    await expect(
      jobs.createMbReleaseJob({ mbReleaseId: 'release-1' }),
    ).rejects.toThrow('Jobs API returned an invalid release job response');
  });

  it('rejects malformed swarm trace responses and accepts a missing trace', async () => {
    api.get.mockResolvedValue({ data: [] });
    await expect(jobs.getSwarmTraceSummary('swarm-1')).rejects.toThrow(
      'Jobs API returned an invalid swarm trace response',
    );

    const error = new Error('Not found');
    error.response = { status: 404 };
    api.get.mockRejectedValue(error);
    await expect(jobs.getSwarmTraceSummary('missing')).resolves.toBeNull();
  });
});
