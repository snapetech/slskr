import {
  MAX_LOCAL_TEXT_FILE_BYTES,
  readFileTextBounded,
} from './fileReaders';

describe('readFileTextBounded', () => {
  it('reads a small UTF-8 file', async () => {
    await expect(
      readFileTextBounded(new File(['hello, world'], 'fixture.txt')),
    ).resolves.toBe('hello, world');
  });

  it('rejects a file from its declared size before reading it', async () => {
    const file = {
      name: 'oversized.txt',
      size: MAX_LOCAL_TEXT_FILE_BYTES + 1,
      slice: vi.fn(),
    };

    await expect(readFileTextBounded(file)).rejects.toThrow(
      'oversized.txt exceeds the 1048576 byte text-file limit',
    );
    expect(file.slice).not.toHaveBeenCalled();
  });

  it('catches a reader that returns more bytes than declared', async () => {
    const file = {
      name: 'lying.txt',
      size: 1,
      slice: () => ({
        arrayBuffer: () => Promise.resolve(new Uint8Array([1, 2]).buffer),
      }),
    };

    await expect(readFileTextBounded(file, 1)).rejects.toThrow(
      'lying.txt exceeds the 1 byte text-file limit',
    );
  });

  it('supports text-only test doubles with the same byte limit', async () => {
    await expect(
      readFileTextBounded({
        name: 'fallback.txt',
        text: () => Promise.resolve('é'),
      }, 2),
    ).resolves.toBe('é');

    await expect(
      readFileTextBounded({
        name: 'fallback.txt',
        text: () => Promise.resolve('é!'),
      }, 2),
    ).rejects.toThrow('fallback.txt exceeds the 2 byte text-file limit');
  });
});
