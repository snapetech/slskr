export const MAX_LOCAL_TEXT_FILE_BYTES = 1024 * 1024;

const validateMaximum = (maximum) => {
  if (!Number.isSafeInteger(maximum) || maximum <= 0) {
    throw new TypeError('File size limit must be a positive safe integer.');
  }
  return maximum;
};

const fileLabel = (file) => file?.name || 'Selected file';

const tooLargeError = (file, maximum) =>
  new Error(`${fileLabel(file)} exceeds the ${maximum} byte text-file limit.`);

export const readFileTextBounded = async (
  file,
  maximum = MAX_LOCAL_TEXT_FILE_BYTES,
) => {
  const limit = validateMaximum(maximum);
  const declaredSize = Number(file?.size);
  if (Number.isFinite(declaredSize) && declaredSize > limit) {
    throw tooLargeError(file, limit);
  }

  if (typeof file?.slice === 'function') {
    const bytes = new Uint8Array(
      await file.slice(0, limit + 1).arrayBuffer(),
    );
    if (bytes.byteLength > limit) {
      throw tooLargeError(file, limit);
    }
    return new TextDecoder().decode(bytes);
  }

  if (typeof file?.text !== 'function') {
    throw new TypeError(`${fileLabel(file)} does not provide a text reader.`);
  }

  const text = await file.text();
  if (new TextEncoder().encode(text).byteLength > limit) {
    throw tooLargeError(file, limit);
  }
  return text;
};
