// Compatibility-only endpoints may be absent from older daemon profiles.
// Treat only a missing route as an empty compatibility response; authentication,
// authorization, and server failures must remain visible to the caller.
export const readOptionalApiResponse = async (request, fallback = { data: [] }) => {
  try {
    return await request();
  } catch (error) {
    if (error?.response?.status === 404) return fallback;
    throw error;
  }
};
