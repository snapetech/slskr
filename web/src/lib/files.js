import api from './api';
import {
  encodePathSegment,
  encodeUtf8Base64PathSegment,
} from './pathEncoding';

export const list = async ({ root, subdirectory = '' }) => {
  const response = (
    await api.get(
      `/files/${encodePathSegment(root)}/directories/${encodeUtf8Base64PathSegment(subdirectory)}`,
    )
  ).data;

  return response;
};

export const deleteDirectory = async ({ root, path }) => {
  const response = await api.delete(
    `/files/${encodePathSegment(root)}/directories/${encodeUtf8Base64PathSegment(path)}`,
  );

  return response;
};

export const deleteFile = async ({ root, path }) => {
  const response = await api.delete(
    `/files/${encodePathSegment(root)}/files/${encodeUtf8Base64PathSegment(path)}`,
  );

  return response;
};
