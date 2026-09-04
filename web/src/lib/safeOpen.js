const SAFE_PROTOCOLS = new Set(['http:', 'https:']);

export const isSafeBlankUrl = (url) => {
  if (typeof url !== 'string' || url.trim() === '') {
    return false;
  }

  try {
    const parsed = new URL(url, window.location.href);
    return SAFE_PROTOCOLS.has(parsed.protocol);
  } catch {
    return false;
  }
};

export const safeOpenBlank = (url) => {
  if (!isSafeBlankUrl(url)) {
    return null;
  }

  try {
    const opened = window.open(url, '_blank', 'noopener,noreferrer');
    if (opened) {
      opened.opener = null;
    }
    return opened;
  } catch {
    return null;
  }
};
