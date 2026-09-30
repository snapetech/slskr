import api from './api';
import * as chat from './chat';
import * as rooms from './rooms';
import * as jobs from './jobs';
import * as slskr from './slskr';
import * as autoReplace from './autoReplace';
import * as analytics from './swarmAnalytics';
import { auditResponseContract } from '../../scripts/audit-response-contracts.mjs';

vi.mock('./api', () => ({ default: { get: vi.fn() } }));

beforeEach(() => {
  vi.resetAllMocks();
  api.get.mockImplementation(async (endpoint) => {
    const data = auditResponseContract(new URL(endpoint, 'http://audit.test').pathname);
    if (data === undefined) throw new Error(`Missing audit contract for ${endpoint}`);
    return { data };
  });
});

describe('browser audit fixtures pass the actual frontend response contracts', () => {
  it.each([
    ['unread activity', chat.hasUnAcknowledgedMessages, false],
    ['room activity', rooms.getActivity, {}],
    ['metadata processing', slskr.getMetadataProcessingStatus, { active: [], history: [] }],
    ['job list', jobs.getJobs, { jobs: [], total: 0, limit: 20, offset: 0 }],
    ['auto-replace status', autoReplace.getAutoReplaceStatus, { enabled: false }],
    ['performance metrics', analytics.getPerformanceMetrics, auditResponseContract('/swarm/analytics/performance')],
    ['efficiency metrics', analytics.getEfficiencyMetrics, auditResponseContract('/swarm/analytics/efficiency')],
    ['trend data', analytics.getTrends, {}],
  ])('validates %s', async (_name, load, expected) => {
    expect(await load()).toEqual(expected);
    expect(api.get).toHaveBeenCalledTimes(1);
  });

  it('leaves unknown routes to the existing fallback', () => {
    expect(auditResponseContract('/searches')).toBeUndefined();
    expect(auditResponseContract('/conversations/alice')).toBeUndefined();
  });

  it('does not share mutable fixture data between requests', () => {
    const first = auditResponseContract('/hashdb/metadata-processing');
    first.active.push({ id: 'test' });
    expect(auditResponseContract('/hashdb/metadata-processing')).toEqual({ active: [], history: [] });
  });
});
