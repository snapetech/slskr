import * as identityAPI from '../../lib/identity';
import Contacts from './Contacts';
import QRCode from 'qrcode';
import React from 'react';
import { MemoryRouter } from 'react-router-dom';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../lib/identity', () => ({
  addContactFromInvite: vi.fn(),
  getContacts: vi.fn(),
  getNearby: vi.fn(),
  createInvite: vi.fn(),
}));

vi.mock('qrcode', () => ({
  default: {
    toDataURL: vi.fn(),
  },
}));

const renderContacts = () =>
  render(
    <MemoryRouter>
      <Contacts />
    </MemoryRouter>,
  );

describe('Contacts', () => {
  beforeEach(() => {
    identityAPI.getContacts.mockResolvedValue({ data: [] });
    identityAPI.getNearby.mockResolvedValue({ data: [] });
    identityAPI.createInvite.mockResolvedValue({
      data: {
        friendCode: 'FRIEND-1234',
        inviteLink: 'slskr://invite/test-invite',
      },
    });
    QRCode.toDataURL.mockResolvedValue('data:image/png;base64,inviteqr');
  });

  afterEach(() => {
    vi.restoreAllMocks();
    delete window.BarcodeDetector;
    delete window.createImageBitmap;
  });

  it('renders a QR code for newly created invites', async () => {
    renderContacts();

    fireEvent.click(await screen.findByText('Create Invite'));

    expect(await screen.findByTestId('contacts-invite-output')).toHaveValue(
      'slskr://invite/test-invite',
    );
    expect(screen.getByTestId('contacts-invite-qr')).toHaveAttribute(
      'src',
      'data:image/png;base64,inviteqr',
    );
    expect(QRCode.toDataURL).toHaveBeenCalledWith(
      'slskr://invite/test-invite',
      {
        errorCorrectionLevel: 'M',
        margin: 2,
        scale: 6,
      },
    );
  });

  it('fills the invite input from a scanned QR image', async () => {
    const close = vi.fn();
    const detect = vi.fn().mockResolvedValue([
      {
        rawValue: 'slskr://invite/scanned',
      },
    ]);

    window.BarcodeDetector = vi.fn(function BarcodeDetector() {
      return { detect };
    });
    window.createImageBitmap = vi.fn().mockResolvedValue({ close });

    renderContacts();

    fireEvent.click(await screen.findByText('Add Friend'));
    fireEvent.change(screen.getByTestId('contacts-add-invite-qr-file'), {
      target: {
        files: [new File(['qr'], 'invite.png', { type: 'image/png' })],
      },
    });

    await waitFor(() => {
      expect(screen.getByTestId('contacts-add-invite-input')).toHaveValue(
        'slskr://invite/scanned',
      );
    });

    expect(window.BarcodeDetector).toHaveBeenCalledWith({ formats: ['qr_code'] });
    expect(detect).toHaveBeenCalled();
    expect(close).toHaveBeenCalled();
  });

  it('surfaces access failures instead of showing an empty contact list', async () => {
    identityAPI.getContacts.mockRejectedValueOnce({
      response: {
        data: { message: 'Contacts access denied' },
        status: 403,
      },
    });

    renderContacts();

    expect(await screen.findByText(/Contacts access denied/)).toBeInTheDocument();
    expect(screen.getByText('Contacts unavailable')).toBeInTheDocument();
    expect(screen.queryByText('No contacts yet')).not.toBeInTheDocument();
  });

  it('keeps a missing contacts route as a compatibility empty state', async () => {
    identityAPI.getContacts.mockRejectedValueOnce({
      response: { status: 404 },
    });

    renderContacts();

    expect(await screen.findByText('No contacts yet')).toBeInTheDocument();
    expect(screen.queryByText('Contacts unavailable')).not.toBeInTheDocument();
  });

  it('retains nearby peers and identifies discovery failures', async () => {
    identityAPI.getNearby
      .mockResolvedValueOnce({
        data: [{ displayName: 'nearby-fixture', peerId: 'peer-fixture' }],
      })
      .mockRejectedValueOnce(new Error('mDNS discovery unavailable'));

    renderContacts();

    expect(await screen.findByText('nearby-fixture')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Refresh Nearby' }));

    await waitFor(() => {
      expect(screen.getByTestId('contacts-nearby-load-error')).toHaveTextContent(
        'mDNS discovery unavailable',
      );
      expect(screen.getByText('nearby-fixture')).toBeInTheDocument();
    });
    expect(screen.queryByText('No nearby peers found')).not.toBeInTheDocument();
  });
});
