import { urlBase } from '../config';
import { fetchWithoutRedirects, readJsonResponse } from './http';
import * as session from './session';

const baseUrl = `${urlBase}/api/v0/mesh`;

export const getStats = async () => {
  const response = await fetchWithoutRedirects(`${baseUrl}/transport`, {
    headers: session.authHeaders(),
  });

  if (!response.ok) {
    throw new Error(
      `Failed to get mesh transport stats: ${response.statusText}`,
    );
  }

  return readJsonResponse(response);
};
