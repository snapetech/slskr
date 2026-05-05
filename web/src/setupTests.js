// <copyright file="setupTests.js" company="slskr Team">
// Copyright (c) slskr Team. All rights reserved.
// </copyright>

// jest-dom adds custom jest matchers for asserting on DOM nodes.
// allows you to do things like:
// expect(element).toHaveTextContent(/react/i)
// learn more: https://github.com/testing-library/jest-dom
import '@testing-library/jest-dom';

// Vitest compatibility shim: alias 'jest' to 'vi' so test files written
// for Jest continue to work without modification.
// eslint-disable-next-line no-undef
globalThis.jest = vi;
