import api from './api';
import { encodePathSegment } from './pathEncoding';

const requireObjectResponse = (value, resource) => {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error(`Bridge API returned an invalid ${resource} response`);
  }

  return value;
};

const requireClientsResponse = (value) => {
  const response = requireObjectResponse(value, 'clients');
  if (!Array.isArray(response.clients)) {
    throw new Error('Bridge API returned an invalid client list response');
  }

  return response.clients;
};

/**
 * Bridge API library for legacy client compatibility.
 */

export const getConfig = async () => {
  const response = await api.get('/bridge/admin/config');
  return requireObjectResponse(response.data, 'configuration');
};

export const updateConfig = async (config) => {
  const response = await api.put('/bridge/admin/config', config);
  return response.data;
};

export const getDashboard = async () => {
  const response = await api.get('/bridge/admin/dashboard');
  return requireObjectResponse(response.data, 'dashboard');
};

export const getClients = async () => {
  const response = await api.get('/bridge/admin/clients');
  return requireClientsResponse(response.data);
};

export const getStats = async () => {
  const response = await api.get('/bridge/admin/stats');
  return requireObjectResponse(response.data, 'stats');
};

export const getStatus = async () => {
  const response = await api.get('/bridge/status');
  return requireObjectResponse(response.data, 'status');
};

export const startBridge = async () => {
  const response = await api.post('/bridge/start');
  return response.data;
};

export const stopBridge = async () => {
  const response = await api.post('/bridge/stop');
  return response.data;
};

export const getTransferProgress = async (transferId) => {
  const response = await api.get(
    `/bridge/transfer/${encodePathSegment(transferId)}/progress`,
  );
  return requireObjectResponse(response.data, 'transfer progress');
};
