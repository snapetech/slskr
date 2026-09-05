import '@testing-library/jest-dom';
import * as files from '../../../lib/files';
import Explorer from './Explorer';
import { cleanup, render, screen } from '@testing-library/react';
import React from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../../lib/files', () => ({
  deleteDirectory: vi.fn(),
  deleteFile: vi.fn(),
  list: vi.fn(),
}));

vi.mock('react-toastify', () => ({
  toast: { error: vi.fn() },
}));

describe('Files Explorer', () => {
  beforeEach(() => {
    files.list.mockRejectedValue(new Error('File service unavailable'));
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it('surfaces listing failures instead of showing an empty success state', async () => {
    render(<Explorer root="downloads" />);

    expect(await screen.findByTestId('files-load-error')).toHaveTextContent(
      'File service unavailable',
    );
    expect(screen.getByText('Files unavailable')).toBeInTheDocument();
    expect(screen.queryByText('No files or directories')).not.toBeInTheDocument();
  });
});
