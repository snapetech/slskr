// <copyright file="federationDiagnostics.js" company="slskr Team">
// Copyright (c) slskr Team. All rights reserved.
// </copyright>

import api from './api';

export const getDiagnostics = () => api.get('/federation/diagnostics');
