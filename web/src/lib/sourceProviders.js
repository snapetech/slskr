import api from './api';

const requireObjectResponse = (value) => {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error('Source providers API returned an invalid catalog response');
  }

  return value;
};

const requireArrayField = (value, field) => {
  if (!Array.isArray(value)) {
    throw new Error(`Source providers API returned an invalid ${field} response`);
  }

  return value;
};

export const getSourceProviders = async () => {
  const response = requireObjectResponse((await api.get('/source-providers')).data);
  const acquisitionPlanningEnabled =
    response.acquisitionPlanningEnabled ?? response.AcquisitionPlanningEnabled;

  if (
    acquisitionPlanningEnabled !== undefined &&
    typeof acquisitionPlanningEnabled !== 'boolean'
  ) {
    throw new Error(
      'Source providers API returned an invalid acquisition-planning flag',
    );
  }

  return {
    acquisitionPlanningEnabled: acquisitionPlanningEnabled ?? false,
    profilePolicies: requireArrayField(
      response.profilePolicies ?? response.ProfilePolicies,
      'profile policies',
    ),
    providers: requireArrayField(
      response.providers ?? response.Providers,
      'providers',
    ),
  };
};
