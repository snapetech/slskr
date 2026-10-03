import User from './User';
import { render, screen } from '@testing-library/react';

const baseUser = {
  address: 'Unknown',
  description: '',
  hasPicture: false,
  picture: '',
  port: 'Unknown',
  presence: 'Offline',
  queueLength: 'Unknown',
  uploadSlots: 'Unknown',
  username: 'commons_peer',
};

describe('user profile summary', () => {
  it.each([
    [true, 'Available'],
    [false, 'Unavailable'],
    [undefined, 'Unknown'],
  ])('shows %s only when the peer reports it', (hasFreeUploadSlot, label) => {
    render(<User {...baseUser} hasFreeUploadSlot={hasFreeUploadSlot} />);

    expect(screen.getByText(label)).toBeInTheDocument();
    expect(screen.getByRole('heading', { level: 2, name: 'commons_peer' }))
      .toBeInTheDocument();
  });

  it('describes a missing profile description without implying all data is missing', () => {
    render(<User {...baseUser} hasFreeUploadSlot={false} />);

    expect(screen.getByText('No profile description provided.')).toBeInTheDocument();
    expect(screen.queryByText('No user info.')).not.toBeInTheDocument();
  });

  it('labels presence and replaces repeated missing values with one clear message', () => {
    render(<User {...baseUser} hasFreeUploadSlot={undefined} />);

    expect(screen.getByText('Presence')).toBeInTheDocument();
    expect(screen.getByText('Offline')).toBeInTheDocument();
    expect(screen.getByText('Live network details are currently unavailable.'))
      .toBeInTheDocument();
    expect(screen.queryByText('IP address')).not.toBeInTheDocument();
    expect(screen.queryByText('Queue length')).not.toBeInTheDocument();
    expect(screen.getByLabelText('Presence: Offline')).toBeInTheDocument();
  });

  it('shows only network details returned by the peer', () => {
    render(
      <User
        {...baseUser}
        address="192.0.2.10"
        hasFreeUploadSlot={true}
        port="2242"
        queueLength="0"
        uploadSlots="2"
      />,
    );

    expect(screen.getByText('Upload slots')).toBeInTheDocument();
    expect(screen.getByText('2')).toBeInTheDocument();
    expect(screen.getByText('Queue length')).toBeInTheDocument();
    expect(screen.getByText('0')).toBeInTheDocument();
    expect(screen.getByText('IP address')).toBeInTheDocument();
    expect(screen.getByText('192.0.2.10')).toBeInTheDocument();
    expect(screen.getByText('Port')).toBeInTheDocument();
    expect(screen.getByText('2242')).toBeInTheDocument();
    expect(screen.queryByText('Live network details are currently unavailable.'))
      .not.toBeInTheDocument();
  });
});
