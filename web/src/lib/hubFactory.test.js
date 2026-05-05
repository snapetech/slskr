import { tokenPassthroughValue } from '../config';
import { eventFeedProtocols, websocketAuthProtocolPrefix } from './hubFactory';
import { setToken } from './token';

describe('event feed websocket auth', () => {
  beforeEach(() => {
    localStorage.clear();
    sessionStorage.clear();
  });

  it('sends browser-safe auth through a websocket subprotocol', () => {
    setToken(sessionStorage, 'route-token/with space');

    expect(eventFeedProtocols()).toEqual([
      `${websocketAuthProtocolPrefix}route-token%2Fwith%20space`,
    ]);
  });

  it('omits auth subprotocols for passthrough and missing tokens', () => {
    expect(eventFeedProtocols()).toEqual([]);

    setToken(sessionStorage, tokenPassthroughValue);

    expect(eventFeedProtocols()).toEqual([]);
  });
});
