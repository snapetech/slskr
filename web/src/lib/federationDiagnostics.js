// <copyright file="federationDiagnostics.js" company="slskR Team">
// Copyright (c) slskR Team. All rights reserved.
// </copyright>

import api from './api';

export const getDiagnostics = () => api.get('/federation/diagnostics');
