import { urlBase } from '../config';
import { fetchWithoutRedirects, readJsonResponse } from './http';
import { encodePathSegment } from './pathEncoding';
import * as session from './session';

const baseUrl = `${urlBase}/api/v0/port-forwarding`;

export const startForwarding = async (config) => {
  const response = await fetchWithoutRedirects(`${baseUrl}/start`, {
    body: JSON.stringify(config),
    headers: {
      ...session.authHeaders({ csrf: true }),
      'Content-Type': 'application/json',
    },
    method: 'POST',
  });

  if (!response.ok) {
    const errorData = await readJsonResponse(response)
      .catch(() => ({ error: response.statusText }));
    throw new Error(
      errorData.error ||
        `Failed to start port forwarding: ${response.statusText}`,
    );
  }

  return readJsonResponse(response);
};

export const stopForwarding = async (localPort) => {
  const response = await fetchWithoutRedirects(
    `${baseUrl}/stop/${encodePathSegment(localPort)}`,
    {
      headers: session.authHeaders({ csrf: true }),
      method: 'POST',
    },
  );

  if (!response.ok) {
    const errorData = await readJsonResponse(response)
      .catch(() => ({ error: response.statusText }));
    throw new Error(
      errorData.error ||
        `Failed to stop port forwarding: ${response.statusText}`,
    );
  }

  return readJsonResponse(response);
};

export const getForwardingStatus = async () => {
  const response = await fetchWithoutRedirects(`${baseUrl}/status`, {
    headers: session.authHeaders(),
  });

  if (!response.ok) {
    throw new Error(`Failed to get forwarding status: ${response.statusText}`);
  }

  return readJsonResponse(response);
};

export const getForwardingStatusByPort = async (localPort) => {
  const response = await fetchWithoutRedirects(
    `${baseUrl}/status/${encodePathSegment(localPort)}`,
    {
      headers: session.authHeaders(),
    },
  );

  if (!response.ok) {
    if (response.status === 404) {
      return null;
    }

    const errorData = await readJsonResponse(response)
      .catch(() => ({ error: response.statusText }));
    throw new Error(
      errorData.error ||
        `Failed to get forwarding status: ${response.statusText}`,
    );
  }

  return readJsonResponse(response);
};

export const getAvailablePorts = async (
  startPort = 1_024,
  endPort = 65_535,
) => {
  const parameters = new URLSearchParams({
    startPort: String(startPort),
    endPort: String(endPort),
  });
  const response = await fetchWithoutRedirects(
    `${baseUrl}/available-ports?${parameters.toString()}`,
    {
      headers: session.authHeaders(),
    },
  );

  if (!response.ok) {
    throw new Error(`Failed to get available ports: ${response.statusText}`);
  }

  return readJsonResponse(response);
};
