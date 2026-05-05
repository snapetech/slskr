export const safeOpenBlank = (url) => {
  const opened = window.open(url, '_blank', 'noopener,noreferrer');
  if (opened) {
    opened.opener = null;
  }
  return opened;
};
