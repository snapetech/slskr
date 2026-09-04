export const encodePathSegment = (value) =>
  encodeURIComponent(String(value ?? ''));

export const encodeUtf8Base64PathSegment = (value) => {
  const bytes = new TextEncoder().encode(String(value ?? ''));
  let binary = '';

  for (let index = 0; index < bytes.length; index += 0x8000) {
    binary += String.fromCharCode(...bytes.subarray(index, index + 0x8000));
  }

  return encodePathSegment(btoa(binary));
};
