// <copyright file="quarantineJury.js" company="slskr Team">
// Copyright (c) slskr Team. All rights reserved.
// </copyright>

import api from './api';

const baseUrl = '/quarantine-jury';

const requireArrayResponse = (value, resource) => {
  if (!Array.isArray(value)) {
    throw new Error(`Quarantine Jury API returned an invalid ${resource} response`);
  }

  return value;
};

export const getRequests = async () => {
  const { data } = await api.get(`${baseUrl}/requests`);
  return requireArrayResponse(data, 'request list');
};

export const getReview = async (requestId) =>
  (await api.get(`${baseUrl}/requests/${encodeURIComponent(requestId)}/review`))
    .data;

export const acceptReleaseCandidate = async (requestId, request = {}) =>
  (
    await api.post(
      `${baseUrl}/requests/${encodeURIComponent(requestId)}/accept-release-candidate`,
      request,
    )
  ).data;

export const routeRequest = async (requestId, request = {}) =>
  (
    await api.post(
      `${baseUrl}/requests/${encodeURIComponent(requestId)}/routes`,
      request,
    )
  ).data;
