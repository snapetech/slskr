import React from 'react';
import ContentDescriptorPublishingPanel from './ContentDescriptorPublishingPanel';
import ContentDescriptorRetrievalPanel from './ContentDescriptorRetrievalPanel';

const MediaCoreDescriptorManagementPanel = ({ setVerificationResult }) => (
  <>
    <ContentDescriptorPublishingPanel />
    <ContentDescriptorRetrievalPanel
      setVerificationResult={setVerificationResult}
    />
  </>
);

export default React.memo(MediaCoreDescriptorManagementPanel);
